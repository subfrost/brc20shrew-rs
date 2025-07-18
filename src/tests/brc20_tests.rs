// Chadson v69.0.0: This file contains all tests for BRC20 logic.
// It validates the parsing and processing of BRC20 operations,
// ensuring the indexer's state is updated correctly.

use crate::brc20::{Brc20Indexer, Brc20Operation, Balance, TransferInfo};
use crate::indexer::InscriptionIndexer;
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};
use crate::tests::helpers;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_parse_deploy_operation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ordi", "max": "21000000", "lim": "1000" }"#;
    let operation = indexer.parse_operation(content).unwrap();
    match operation {
        Brc20Operation::Deploy {
            ticker,
            max_supply,
            limit_per_mint,
            decimals,
        } => {
            assert_eq!(ticker, "ordi");
            assert_eq!(max_supply, 21000000);
            assert_eq!(limit_per_mint, 1000);
            assert_eq!(decimals, 18);
        }
        _ => panic!("Incorrect operation parsed"),
    }
}

#[wasm_bindgen_test]
fn test_parse_mint_operation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "mint", "tick": "ordi", "amt": "100" }"#;
    let operation = indexer.parse_operation(content).unwrap();
    match operation {
        Brc20Operation::Mint { ticker, amount } => {
            assert_eq!(ticker, "ordi");
            assert_eq!(amount, 100);
        }
        _ => panic!("Incorrect operation parsed"),
    }
}

#[wasm_bindgen_test]
fn test_parse_transfer_operation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "transfer", "tick": "ordi", "amt": "50" }"#;
    let operation = indexer.parse_operation(content).unwrap();
    match operation {
        Brc20Operation::Transfer { ticker, amount } => {
            assert_eq!(ticker, "ordi");
            assert_eq!(amount, 50);
        }
        _ => panic!("Incorrect operation parsed"),
    }
}

#[wasm_bindgen_test]
fn test_process_deploy_operation() {
    helpers::clear();
    let indexer = Brc20Indexer::new();
    let tickers_table = Brc20Tickers::new();
    let owner = helpers::get_test_address(0).to_string();

    let inscription_id = "inscription_id_1";
    let deploy_op = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21000000,
        limit_per_mint: 1000,
        decimals: 18,
    };

    indexer.process_operation(&deploy_op, inscription_id, &owner).unwrap();
    let ticker_data = tickers_table.get("ordi").unwrap();
    let ticker: crate::brc20::Ticker = serde_json::from_slice(&ticker_data).unwrap();
    assert_eq!(ticker.name, "ordi");
    assert_eq!(ticker.max_supply, 21000000);
    assert_eq!(ticker.limit_per_mint, 1000);
    assert_eq!(ticker.current_supply, 0);
}

#[wasm_bindgen_test]
fn test_process_mint_operation() {
    helpers::clear();
    let indexer = Brc20Indexer::new();
    let tickers_table = Brc20Tickers::new();
    let balances_table = Brc20Balances::new();
    let owner = helpers::get_test_address(0).to_string();

    let deploy_op = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21000000,
        limit_per_mint: 1000,
        decimals: 18,
    };

    let mint_op = Brc20Operation::Mint {
        ticker: "ordi".to_string(),
        amount: 100,
    };

    indexer.process_operation(&deploy_op, "inscription_id_1", &owner).unwrap();
    indexer.process_operation(&mint_op, "inscription_id_2", &owner).unwrap();
    
    let ticker_data = tickers_table.get("ordi").unwrap();
    let ticker: crate::brc20::Ticker = serde_json::from_slice(&ticker_data).unwrap();
    assert_eq!(ticker.current_supply, 100);

    let balance_data = balances_table.get(&owner, "ordi").unwrap();
    let balance: Balance = serde_json::from_slice(&balance_data).unwrap();
    assert_eq!(balance.total_balance, 100);
    assert_eq!(balance.available_balance, 100);
}

#[wasm_bindgen_test]
fn test_process_transfer_inscribe_operation() {
    helpers::clear();
    let indexer = Brc20Indexer::new();
    let balances_table = Brc20Balances::new();
    let owner = helpers::get_test_address(0).to_string();

    // Mint some tokens first
    let deploy_op = Brc20Operation::Deploy { ticker: "ordi".to_string(), max_supply: 21000, limit_per_mint: 1000, decimals: 18 };
    let mint_op = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 };
    indexer.process_operation(&deploy_op, "inscription_id_1", &owner).unwrap();
    indexer.process_operation(&mint_op, "inscription_id_2", &owner).unwrap();

    // Inscribe a transfer
    let transfer_op = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 100 };
    indexer.process_operation(&transfer_op, "inscription_id_3", &owner).unwrap();

    // Check balance
    let balance_data = balances_table.get(&owner, "ordi").unwrap();
    let balance: Balance = serde_json::from_slice(&balance_data).unwrap();
    assert_eq!(balance.total_balance, 1000);
    assert_eq!(balance.available_balance, 900);

    // Check that the transfer info was stored
    let transferable_table = Brc20TransferableInscriptions::new();
    let transfer_info_data = transferable_table.get("inscription_id_3").unwrap();
    let transfer_info: TransferInfo = serde_json::from_slice(&transfer_info_data).unwrap();
    assert_eq!(transfer_info.ticker, "ordi");
    assert_eq!(transfer_info.amount, 100);
}

#[wasm_bindgen_test]
fn test_full_transfer_lifecycle() {
    helpers::clear();
    let mut indexer = InscriptionIndexer::new();
    indexer.network = bitcoin::Network::Regtest;
    let owner_address = helpers::get_test_address(0);
    let recipient_address = helpers::get_test_address(1);

    // 1. Deploy "DOGE" token
    let (deploy_block, deploy_tx) = helpers::create_test_block_with_brc20_deploy_op();
    indexer.index_block(&deploy_block, 0).unwrap();

    // 2. Mint 1000 DOGE to owner
    let (mint_block, _mint_tx) = helpers::create_test_block_with_brc20_mint_op("DOGE", 1000, &owner_address, &deploy_tx.txid());
    indexer.index_block(&mint_block, 1).unwrap();

    // Verify owner's initial balance
    let balances_table = Brc20Balances::new();
    let owner_balance_data = balances_table.get(&owner_address.to_string(), "DOGE").unwrap();
    let owner_balance: Balance = serde_json::from_slice(&owner_balance_data).unwrap();
    assert_eq!(owner_balance.total_balance, 1000);
    assert_eq!(owner_balance.available_balance, 1000);

    // 3. Inscribe a transfer for 100 DOGE
    let (inscribe_block, inscribe_tx) = helpers::create_test_block_with_brc20_transfer_inscribe_op("DOGE", 100, &owner_address, &deploy_tx.txid());
    let inscribe_inscription_id = format!("{}i0", inscribe_tx.txid());
    indexer.index_block(&inscribe_block, 2).unwrap();

    // Verify owner's balance after inscribing
    let owner_balance_data_after_inscribe = balances_table.get(&owner_address.to_string(), "DOGE").unwrap();
    let owner_balance_after_inscribe: Balance = serde_json::from_slice(&owner_balance_data_after_inscribe).unwrap();
    assert_eq!(owner_balance_after_inscribe.total_balance, 1000);
    assert_eq!(owner_balance_after_inscribe.available_balance, 900);

    // Verify transferable inscription exists
    let transferable_table = Brc20TransferableInscriptions::new();
    assert!(transferable_table.get(&inscribe_inscription_id).is_some());

    // 4. Spend the transfer inscription to the recipient
    let (claim_block, _claim_tx) = helpers::create_test_block_with_brc20_transfer_claim_op(&inscribe_tx, &recipient_address);
    indexer.index_block(&claim_block, 3).unwrap();

    // 5. Verify final balances
    // Owner's total balance is now debited
    let final_owner_balance_data = balances_table.get(&owner_address.to_string(), "DOGE").unwrap();
    let final_owner_balance: Balance = serde_json::from_slice(&final_owner_balance_data).unwrap();
    assert_eq!(final_owner_balance.total_balance, 900);
    assert_eq!(final_owner_balance.available_balance, 900);

    // Recipient's balance should be 100
    let recipient_balance_data = balances_table.get(&recipient_address.to_string(), "DOGE").unwrap();
    let recipient_balance: Balance = serde_json::from_slice(&recipient_balance_data).unwrap();
    assert_eq!(recipient_balance.total_balance, 100);
    assert_eq!(recipient_balance.available_balance, 100);

    // 6. Verify transferable inscription is deleted
    assert!(transferable_table.get(&inscribe_inscription_id).is_none());
}