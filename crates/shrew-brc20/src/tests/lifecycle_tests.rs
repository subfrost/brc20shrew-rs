use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::brc20::{Brc20Indexer, Brc20Operation, TransferInfo};
use crate::tables::{Brc20Tickers, Brc20Balances};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::assertions::{assert_brc20_balance, assert_brc20_supply};

const SCALE: u128 = 1_000_000_000_000_000_000u128; // 10^18

/// Helper: deploy a ticker via process_operation
fn deploy(indexer: &Brc20Indexer, ticker: &str, max_supply: u128, lim: u128, inscription_id: &str) {
    let op = Brc20Operation::Deploy {
        ticker: ticker.to_string(),
        max_supply,
        limit_per_mint: lim,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&op, inscription_id, "bc1qdeployer").unwrap();
}

/// Helper: mint tokens via process_operation
fn mint(indexer: &Brc20Indexer, ticker: &str, amount: u128, owner: &str, inscription_id: &str) {
    let op = Brc20Operation::Mint {
        ticker: ticker.to_string(),
        amount,
    };
    indexer.process_operation(&op, inscription_id, owner).unwrap();
}

/// Helper: inscribe a transfer via process_operation
fn transfer_inscribe(
    indexer: &Brc20Indexer,
    ticker: &str,
    amount: u128,
    owner: &str,
    inscription_id: &str,
) {
    let op = Brc20Operation::Transfer {
        ticker: ticker.to_string(),
        amount,
    };
    indexer.process_operation(&op, inscription_id, owner).unwrap();
}

/// Helper: claim a transfer
fn transfer_claim(indexer: &Brc20Indexer, ticker: &str, amount: u128, sender: &str, recipient: &str) {
    let info = TransferInfo {
        ticker: ticker.to_string(),
        amount,
        sender: sender.to_string(),
    };
    indexer.claim_transfer(recipient, &info).unwrap();
}

// ---------------------------------------------------------------------------
// Full lifecycle tests
// ---------------------------------------------------------------------------

#[test]
fn test_deploy_mint_transfer_lifecycle() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "ordi", 21_000_000 * SCALE, 1000 * SCALE, "deploy_0i0");

    let ticker_data = Brc20Tickers::new().get("ordi");
    assert!(ticker_data.is_some(), "Ticker should exist after deploy");

    mint(&indexer, "ordi", 1000 * SCALE, "bc1qalice", "mint_0i0");
    assert_brc20_supply("ordi", 1000 * SCALE);
    assert_brc20_balance("bc1qalice", "ordi", 1000 * SCALE, 1000 * SCALE);

    transfer_inscribe(&indexer, "ordi", 400 * SCALE, "bc1qalice", "xfer_inscribe_0i0");
    assert_brc20_balance("bc1qalice", "ordi", 600 * SCALE, 1000 * SCALE);

    transfer_claim(&indexer, "ordi", 400 * SCALE, "bc1qalice", "bc1qbob");
    assert_brc20_balance("bc1qalice", "ordi", 600 * SCALE, 600 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 400 * SCALE, 400 * SCALE);

    assert_brc20_supply("ordi", 1000 * SCALE);
}

#[test]
fn test_multiple_mints_same_ticker() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "pepe", 1_000_000 * SCALE, 500 * SCALE, "deploy_0i0");

    mint(&indexer, "pepe", 500 * SCALE, "bc1qminer1", "mint1_0i0");
    mint(&indexer, "pepe", 500 * SCALE, "bc1qminer2", "mint2_0i0");
    mint(&indexer, "pepe", 500 * SCALE, "bc1qminer3", "mint3_0i0");

    assert_brc20_supply("pepe", 1500 * SCALE);
    assert_brc20_balance("bc1qminer1", "pepe", 500 * SCALE, 500 * SCALE);
    assert_brc20_balance("bc1qminer2", "pepe", 500 * SCALE, 500 * SCALE);
    assert_brc20_balance("bc1qminer3", "pepe", 500 * SCALE, 500 * SCALE);
}

#[test]
fn test_mint_to_max_supply() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "tiny", 1000 * SCALE, 1000 * SCALE, "deploy_0i0");

    mint(&indexer, "tiny", 1000 * SCALE, "bc1qminter", "mint1_0i0");
    assert_brc20_supply("tiny", 1000 * SCALE);
    assert_brc20_balance("bc1qminter", "tiny", 1000 * SCALE, 1000 * SCALE);

    // Next mint should be rejected (supply exhausted)
    mint(&indexer, "tiny", 1 * SCALE, "bc1qminter2", "mint2_0i0");

    assert_brc20_supply("tiny", 1000 * SCALE);
    let balance = Brc20Balances::new().get("bc1qminter2", "tiny");
    assert!(balance.is_none(), "Mint beyond max supply should be rejected");
}

#[test]
fn test_multiple_tickers() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "ordi", 21_000_000 * SCALE, 1000 * SCALE, "deploy_ordi_0i0");
    deploy(&indexer, "pepe", 420_000_000 * SCALE, 5000 * SCALE, "deploy_pepe_0i0");

    mint(&indexer, "ordi", 1000 * SCALE, "bc1qalice", "mint_ordi_0i0");
    mint(&indexer, "pepe", 5000 * SCALE, "bc1qalice", "mint_pepe_0i0");
    mint(&indexer, "ordi", 800 * SCALE, "bc1qbob", "mint_ordi_1i0");

    assert_brc20_supply("ordi", 1800 * SCALE);
    assert_brc20_supply("pepe", 5000 * SCALE);
    assert_brc20_balance("bc1qalice", "ordi", 1000 * SCALE, 1000 * SCALE);
    assert_brc20_balance("bc1qalice", "pepe", 5000 * SCALE, 5000 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 800 * SCALE, 800 * SCALE);

    let bob_pepe = Brc20Balances::new().get("bc1qbob", "pepe");
    assert!(bob_pepe.is_none(), "Bob should have no pepe tokens");
}

#[test]
fn test_brc20_across_blocks() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "ordi", 21_000_000 * SCALE, 1000 * SCALE, "deploy_0i0");
    assert_brc20_supply("ordi", 0);

    mint(&indexer, "ordi", 1000 * SCALE, "bc1qalice", "mint_0i0");
    assert_brc20_supply("ordi", 1000 * SCALE);
    assert_brc20_balance("bc1qalice", "ordi", 1000 * SCALE, 1000 * SCALE);

    mint(&indexer, "ordi", 500 * SCALE, "bc1qbob", "mint2_0i0");
    transfer_inscribe(&indexer, "ordi", 300 * SCALE, "bc1qalice", "xfer_0i0");

    assert_brc20_supply("ordi", 1500 * SCALE);
    assert_brc20_balance("bc1qalice", "ordi", 700 * SCALE, 1000 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 500 * SCALE, 500 * SCALE);

    transfer_claim(&indexer, "ordi", 300 * SCALE, "bc1qalice", "bc1qbob");
    assert_brc20_balance("bc1qalice", "ordi", 700 * SCALE, 700 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 800 * SCALE, 800 * SCALE);
}

#[test]
fn test_transfer_to_self() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "ordi", 21_000_000 * SCALE, 1000 * SCALE, "deploy_0i0");
    mint(&indexer, "ordi", 1000 * SCALE, "bc1qsame", "mint_0i0");

    transfer_inscribe(&indexer, "ordi", 400 * SCALE, "bc1qsame", "xfer_0i0");
    assert_brc20_balance("bc1qsame", "ordi", 600 * SCALE, 1000 * SCALE);

    transfer_claim(&indexer, "ordi", 400 * SCALE, "bc1qsame", "bc1qsame");
    assert_brc20_balance("bc1qsame", "ordi", 1000 * SCALE, 1000 * SCALE);

    assert_brc20_supply("ordi", 1000 * SCALE);
}
