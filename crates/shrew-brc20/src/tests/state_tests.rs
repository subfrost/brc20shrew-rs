use crate::brc20::{Brc20Indexer, Brc20Operation, Ticker, TransferInfo};
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::assertions::{assert_brc20_balance, assert_brc20_supply};

// ---------------------------------------------------------------------------
// Deploy tests
// ---------------------------------------------------------------------------

#[test]
fn test_deploy_creates_ticker() {
    clear();
    let indexer = Brc20Indexer::new();
    let op = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    indexer.process_operation(&op, "fake_inscription_id_0i0", "bc1qtest").unwrap();

    let data = Brc20Tickers::new().get("ordi");
    assert!(data.is_some(), "Ticker 'ordi' should exist after deploy");
    let ticker: Ticker = serde_json::from_slice(&data.unwrap()).unwrap();
    assert_eq!(ticker.name, "ordi");
    assert_eq!(ticker.max_supply, 21_000_000);
    assert_eq!(ticker.limit_per_mint, 1000);
    assert_eq!(ticker.decimals, 18);
    assert_eq!(ticker.current_supply, 0);
    assert_eq!(ticker.deploy_inscription_id, "fake_inscription_id_0i0");
}

#[test]
fn test_deploy_first_wins() {
    clear();
    let indexer = Brc20Indexer::new();
    let op1 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    let op2 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 99_999_999,
        limit_per_mint: 5000,
        decimals: 8,
    };
    indexer.process_operation(&op1, "first_deploy_0i0", "bc1qfirst").unwrap();
    indexer.process_operation(&op2, "second_deploy_0i0", "bc1qsecond").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    // First deploy should win: max_supply should be 21M, not 99M
    assert_eq!(ticker.max_supply, 21_000_000);
    assert_eq!(ticker.deploy_inscription_id, "first_deploy_0i0");
}

#[test]
fn test_deploy_case_insensitive_check() {
    // In BRC20 v1, tickers are CASE-SENSITIVE.
    // "ORDI" and "ordi" are different tickers.
    clear();
    let indexer = Brc20Indexer::new();

    let op_lower = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    let op_upper = Brc20Operation::Deploy {
        ticker: "ORDI".to_string(),
        max_supply: 10_000_000,
        limit_per_mint: 500,
        decimals: 8,
    };
    indexer.process_operation(&op_lower, "lower_0i0", "bc1qa").unwrap();
    indexer.process_operation(&op_upper, "upper_0i0", "bc1qb").unwrap();

    // Both should exist independently since BRC20 tickers are case-sensitive
    let lower_data = Brc20Tickers::new().get("ordi");
    let upper_data = Brc20Tickers::new().get("ORDI");
    assert!(lower_data.is_some(), "'ordi' ticker should exist");
    assert!(upper_data.is_some(), "'ORDI' ticker should exist");

    let lower: Ticker = serde_json::from_slice(&lower_data.unwrap()).unwrap();
    let upper: Ticker = serde_json::from_slice(&upper_data.unwrap()).unwrap();
    assert_eq!(lower.max_supply, 21_000_000);
    assert_eq!(upper.max_supply, 10_000_000);
}

// ---------------------------------------------------------------------------
// Mint tests
// ---------------------------------------------------------------------------

#[test]
fn test_mint_increases_supply() {
    clear();
    let indexer = Brc20Indexer::new();
    // Deploy first
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // Mint
    let mint = Brc20Operation::Mint {
        ticker: "ordi".to_string(),
        amount: 500,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qowner").unwrap();

    assert_brc20_supply("ordi", 500);
}

#[test]
fn test_mint_increases_balance() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint {
        ticker: "ordi".to_string(),
        amount: 750,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    assert_brc20_balance("bc1qminter", "ordi", 750, 750);
}

#[test]
fn test_mint_exceeds_limit_rejected() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // Try to mint more than limit_per_mint
    let mint = Brc20Operation::Mint {
        ticker: "ordi".to_string(),
        amount: 1001,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    // Supply should remain 0 (mint was rejected)
    assert_brc20_supply("ordi", 0);
    // Balance should not exist
    let balance = Brc20Balances::new().get("bc1qminter", "ordi");
    assert!(balance.is_none(), "Balance should not exist for rejected mint");
}

#[test]
fn test_mint_exceeds_max_supply_rejected() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 1000,
        limit_per_mint: 800,
        decimals: 18,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // First mint of 800: OK
    let mint1 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 800 };
    indexer.process_operation(&mint1, "mint1_0i0", "bc1qminter").unwrap();
    assert_brc20_supply("ordi", 800);

    // Second mint of 800 would push supply to 1600 > 1000 max: rejected
    let mint2 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 800 };
    indexer.process_operation(&mint2, "mint2_0i0", "bc1qminter").unwrap();

    // Supply should still be 800
    assert_brc20_supply("ordi", 800);
    assert_brc20_balance("bc1qminter", "ordi", 800, 800);
}

#[test]
fn test_mint_nonexistent_ticker_ignored() {
    clear();
    let indexer = Brc20Indexer::new();
    // Mint for ticker that was never deployed
    let mint = Brc20Operation::Mint {
        ticker: "fake".to_string(),
        amount: 100,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    let ticker_data = Brc20Tickers::new().get("fake");
    assert!(ticker_data.is_none(), "Non-existent ticker should not appear after mint");
    let balance_data = Brc20Balances::new().get("bc1qminter", "fake");
    assert!(balance_data.is_none(), "Balance should not exist for non-deployed ticker mint");
}

// ---------------------------------------------------------------------------
// Transfer tests
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_inscribe_reduces_available() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy + Mint 1000
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    // Transfer inscribe 400
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 400 };
    indexer.process_operation(&transfer, "transfer_0i0", "bc1qsender").unwrap();

    // available should be 600 (1000 - 400), total should still be 1000
    assert_brc20_balance("bc1qsender", "ordi", 600, 1000);

    // Transferable inscription should exist
    let transferable = Brc20TransferableInscriptions::new().get("transfer_0i0");
    assert!(transferable.is_some(), "Transferable inscription should be recorded");
}

#[test]
fn test_transfer_inscribe_insufficient_balance_ignored() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy + Mint 100
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 100 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    // Try to transfer 200 (more than available 100)
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 200 };
    indexer.process_operation(&transfer, "transfer_0i0", "bc1qsender").unwrap();

    // Balance should be unchanged
    assert_brc20_balance("bc1qsender", "ordi", 100, 100);

    // Transferable inscription should NOT exist
    let transferable = Brc20TransferableInscriptions::new().get("transfer_0i0");
    assert!(transferable.is_none(), "Transfer should not be recorded when balance insufficient");
}

#[test]
fn test_transfer_claim_moves_balance() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Mint 1000 to sender
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    // Transfer inscribe 400
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 400 };
    indexer.process_operation(&transfer, "transfer_0i0", "bc1qsender").unwrap();

    // Claim transfer to recipient
    let transfer_info = TransferInfo {
        ticker: "ordi".to_string(),
        amount: 400,
        sender: "bc1qsender".to_string(),
    };
    indexer.claim_transfer("bc1qrecipient", &transfer_info).unwrap();

    // Sender: available=600, total=600 (400 deducted from total on claim)
    assert_brc20_balance("bc1qsender", "ordi", 600, 600);

    // Recipient: available=400, total=400
    assert_brc20_balance("bc1qrecipient", "ordi", 400, 400);
}
