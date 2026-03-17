use crate::brc20::{Brc20Indexer, Brc20Operation, Ticker, TransferInfo};
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::assertions::{assert_brc20_balance, assert_brc20_supply};

const SCALE: u128 = 1_000_000_000_000_000_000u128; // 10^18

// ---------------------------------------------------------------------------
// Deploy tests
// ---------------------------------------------------------------------------

#[test]
fn test_deploy_creates_ticker() {
    clear();
    let indexer = Brc20Indexer::new();
    let op = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&op, "fake_inscription_id_0i0", "bc1qtest").unwrap();

    let data = Brc20Tickers::new().get("ordi");
    assert!(data.is_some(), "Ticker 'ordi' should exist after deploy");
    let ticker: Ticker = serde_json::from_slice(&data.unwrap()).unwrap();
    assert_eq!(ticker.name, "ordi");
    assert_eq!(ticker.max_supply, 21_000_000 * SCALE);
    assert_eq!(ticker.limit_per_mint, 1000 * SCALE);
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
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    let op2 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 99_999_999 * SCALE,
        limit_per_mint: 5000 * SCALE,
        decimals: 8,
        self_mint: false,
    };
    indexer.process_operation(&op1, "first_deploy_0i0", "bc1qfirst").unwrap();
    indexer.process_operation(&op2, "second_deploy_0i0", "bc1qsecond").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.max_supply, 21_000_000 * SCALE);
    assert_eq!(ticker.deploy_inscription_id, "first_deploy_0i0");
}

#[test]
fn test_deploy_case_insensitive() {
    // BRC-20 tickers are CASE-INSENSITIVE per OPI spec.
    // "ORDI" and "ordi" refer to the same ticker.
    clear();
    let indexer = Brc20Indexer::new();

    let op_lower = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    let op_upper = Brc20Operation::Deploy {
        ticker: "ordi".to_string(), // Already lowercase (parse_operation normalizes)
        max_supply: 10_000_000 * SCALE,
        limit_per_mint: 500 * SCALE,
        decimals: 8,
        self_mint: false,
    };
    indexer.process_operation(&op_lower, "lower_0i0", "bc1qa").unwrap();
    indexer.process_operation(&op_upper, "upper_0i0", "bc1qb").unwrap();

    // Only the first deploy should exist
    let data = Brc20Tickers::new().get("ordi");
    assert!(data.is_some(), "'ordi' ticker should exist");
    let ticker: Ticker = serde_json::from_slice(&data.unwrap()).unwrap();
    assert_eq!(ticker.max_supply, 21_000_000 * SCALE, "First deploy should win");
}

// ---------------------------------------------------------------------------
// Mint tests
// ---------------------------------------------------------------------------

#[test]
fn test_mint_increases_supply() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint {
        ticker: "ordi".to_string(),
        amount: 500 * SCALE,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qowner").unwrap();

    assert_brc20_supply("ordi", 500 * SCALE);
}

#[test]
fn test_mint_increases_balance() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint {
        ticker: "ordi".to_string(),
        amount: 750 * SCALE,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    assert_brc20_balance("bc1qminter", "ordi", 750 * SCALE, 750 * SCALE);
}

#[test]
fn test_mint_exceeds_limit_rejected() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint {
        ticker: "ordi".to_string(),
        amount: 1001 * SCALE,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    assert_brc20_supply("ordi", 0);
    let balance = Brc20Balances::new().get("bc1qminter", "ordi");
    assert!(balance.is_none(), "Balance should not exist for rejected mint");
}

#[test]
fn test_mint_exceeds_max_supply_clamped() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 1000 * SCALE,
        limit_per_mint: 800 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // First mint of 800: OK
    let mint1 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 800 * SCALE };
    indexer.process_operation(&mint1, "mint1_0i0", "bc1qminter").unwrap();
    assert_brc20_supply("ordi", 800 * SCALE);

    // Second mint of 800 requested, 200 remaining — should clamp to 200
    let mint2 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 800 * SCALE };
    indexer.process_operation(&mint2, "mint2_0i0", "bc1qminter").unwrap();

    assert_brc20_supply("ordi", 1000 * SCALE);
    assert_brc20_balance("bc1qminter", "ordi", 1000 * SCALE, 1000 * SCALE);
}

#[test]
fn test_mint_nonexistent_ticker_ignored() {
    clear();
    let indexer = Brc20Indexer::new();
    let mint = Brc20Operation::Mint {
        ticker: "fake".to_string(),
        amount: 100 * SCALE,
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

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 400 * SCALE };
    indexer.process_operation(&transfer, "transfer_0i0", "bc1qsender").unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 1000 * SCALE);

    let transferable = Brc20TransferableInscriptions::new().get("transfer_0i0");
    assert!(transferable.is_some(), "Transferable inscription should be recorded");
}

#[test]
fn test_transfer_inscribe_insufficient_balance_ignored() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 100 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 200 * SCALE };
    indexer.process_operation(&transfer, "transfer_0i0", "bc1qsender").unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 100 * SCALE, 100 * SCALE);

    let transferable = Brc20TransferableInscriptions::new().get("transfer_0i0");
    assert!(transferable.is_none(), "Transfer should not be recorded when balance insufficient");
}

#[test]
fn test_transfer_claim_moves_balance() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 400 * SCALE };
    indexer.process_operation(&transfer, "transfer_0i0", "bc1qsender").unwrap();

    let transfer_info = TransferInfo {
        ticker: "ordi".to_string(),
        amount: 400 * SCALE,
        sender: "bc1qsender".to_string(),
    };
    indexer.claim_transfer("bc1qrecipient", &transfer_info).unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 600 * SCALE);
    assert_brc20_balance("bc1qrecipient", "ordi", 400 * SCALE, 400 * SCALE);
}
