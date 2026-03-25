///! OPI Conformance Tests
///!
///! These tests verify that the BRC-20 implementation matches the OPI reference
///! implementation behavior. Each test targets a specific divergence identified
///! in the audit. Tests are expected to FAIL initially and be fixed iteratively.
///!
///! After the u64->u128 type migration, amount assertions will be updated.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::brc20::{Brc20Indexer, Brc20Operation, Ticker, Balance};
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};
use shrew_test_helpers::state::clear;
use wasm_bindgen_test::wasm_bindgen_test;

// ============================================================================
// ISSUE 1: Decimal amount parsing
//
// OPI parses "500.5" as a valid amount. WASM uses parse::<u64>() which
// silently drops any inscription with decimal amounts.
// ============================================================================

#[wasm_bindgen_test]
fn test_parse_decimal_amount_mint() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "mint", "tick": "ordi", "amt": "500.5" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Decimal amount '500.5' should be parseable");
}

#[wasm_bindgen_test]
fn test_parse_decimal_amount_deploy() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "test", "max": "21000000.0", "lim": "1000.0" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Decimal deploy amounts should be parseable");
}

#[wasm_bindgen_test]
fn test_parse_amount_too_many_decimals_rejected() {
    let indexer = Brc20Indexer::new();
    // 19 decimal places exceeds max of 18
    let content = br#"{ "p": "brc-20", "op": "mint", "tick": "ordi", "amt": "1.0000000000000000001" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_none(), "Amount with >18 decimals should be rejected");
}

#[wasm_bindgen_test]
fn test_parse_integer_amount_still_works() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "mint", "tick": "ordi", "amt": "1000" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Integer amount should still parse");
}

// ============================================================================
// ISSUE 2: Case-insensitive ticker matching
//
// OPI lowercases all tickers. "ORDI" and "ordi" are the same ticker.
// ============================================================================

#[wasm_bindgen_test]
fn test_ticker_case_insensitive_deploy_dedup() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy "ordi"
    let op1 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&op1, "first_deploy_0i0", "bc1qfirst").unwrap();

    // Deploy "ORDI" — should be rejected (same ticker, case-insensitive)
    let op2 = Brc20Operation::Deploy {
        ticker: "ORDI".to_string(),
        max_supply: 99_999_999,
        limit_per_mint: 5000,
        decimals: 8,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&op2, "second_deploy_0i0", "bc1qsecond").unwrap();

    // Only the first deploy's params should be stored
    // The ticker should be stored under the lowercase key
    let data = Brc20Tickers::new().get("ordi").expect("'ordi' should exist");
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.max_supply, 21_000_000, "First deploy should win (case-insensitive)");

    // "ORDI" as a separate ticker should NOT exist
    // If the implementation normalizes to lowercase, then Brc20Tickers::get("ORDI")
    // either returns None (no separate entry) or returns the same as "ordi" (normalized lookup).
    // Either way, there must not be a separate ticker with max_supply=99_999_999.
    if let Some(upper_data) = Brc20Tickers::new().get("ORDI") {
        let upper_ticker: Ticker = serde_json::from_slice(&upper_data).unwrap();
        // If ORDI returns data, it must be the SAME ticker (the ordi deploy), not the second deploy
        assert_eq!(upper_ticker.max_supply, 21_000_000,
            "'ORDI' lookup must not create a separate ticker — should resolve to 'ordi'");
    }
    // If get("ORDI") returns None, that's also correct as long as no separate ticker exists.
}

#[wasm_bindgen_test]
fn test_ticker_case_insensitive_mint() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy as "ordi"
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // Mint using "ORDI" — should resolve to "ordi"
    let mint = Brc20Operation::Mint {
        ticker: "ORDI".to_string(),
        amount: 500,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    // Supply of "ordi" should have increased
    let data = Brc20Tickers::new().get("ordi").expect("'ordi' should exist");
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.current_supply, 500,
        "Minting 'ORDI' should increase 'ordi' supply (case-insensitive)");
}

// ============================================================================
// ISSUE 3: Protocol field ("p": "brc-20") validation
// ============================================================================

#[wasm_bindgen_test]
fn test_reject_missing_protocol_field() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "op": "deploy", "tick": "fake", "max": "1000", "lim": "100" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_none(), "Missing 'p' field should be rejected");
}

#[wasm_bindgen_test]
fn test_reject_wrong_protocol_field() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-21", "op": "deploy", "tick": "fake", "max": "1000", "lim": "100" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_none(), "Wrong protocol 'brc-21' should be rejected");
}

#[wasm_bindgen_test]
fn test_accept_correct_protocol_field() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ordi", "max": "21000000", "lim": "1000" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Correct protocol 'brc-20' should be accepted");
}

// ============================================================================
// ISSUE 4: Ticker length validation
// ============================================================================

#[wasm_bindgen_test]
fn test_reject_1_char_ticker() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "a", "max": "1000", "lim": "100" }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "1-char ticker should be rejected");
}

#[wasm_bindgen_test]
fn test_reject_3_char_ticker() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "abc", "max": "1000", "lim": "100" }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "3-char ticker should be rejected");
}

#[wasm_bindgen_test]
fn test_accept_4_char_ticker() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ordi", "max": "21000000", "lim": "1000" }"#;
    assert!(indexer.parse_operation(content, 840000).is_some(), "4-char ticker should be accepted");
}

#[wasm_bindgen_test]
fn test_reject_7_char_ticker() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "toolong", "max": "1000", "lim": "100" }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "7-char ticker should be rejected");
}

#[wasm_bindgen_test]
fn test_reject_ticker_with_null_byte() {
    let indexer = Brc20Indexer::new();
    let content = b"{ \"p\": \"brc-20\", \"op\": \"deploy\", \"tick\": \"or\x00i\", \"max\": \"1000\", \"lim\": \"100\" }";
    assert!(indexer.parse_operation(content, 840000).is_none(), "Ticker with null byte should be rejected");
}

// ============================================================================
// ISSUE 5: Partial mint clamping
// ============================================================================

#[wasm_bindgen_test]
fn test_partial_mint_clamps_to_remaining() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 1000,
        limit_per_mint: 800,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // First mint: 800
    let mint1 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 800 };
    indexer.process_operation(&mint1, "mint1_0i0", "bc1qminter1").unwrap();

    // Second mint: 800 requested, 200 remaining. OPI clamps to 200.
    let mint2 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 800 };
    indexer.process_operation(&mint2, "mint2_0i0", "bc1qminter2").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.current_supply, 1000,
        "Supply should be clamped to max (1000), not remain at 800");

    let balance_data = Brc20Balances::new().get("bc1qminter2", "ordi")
        .expect("minter2 should have a balance from partial mint");
    let balance: Balance = serde_json::from_slice(&balance_data).unwrap();
    assert_eq!(balance.available_balance, 200,
        "minter2 should receive 200 (clamped remainder)");
}

// ============================================================================
// ISSUE 6: Zero amount rejection
// ============================================================================

#[wasm_bindgen_test]
fn test_reject_zero_mint_amount() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 0 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.current_supply, 0, "Zero amount mint should be rejected");

    let balance = Brc20Balances::new().get("bc1qminter", "ordi");
    assert!(balance.is_none(), "No balance should exist for zero mint");
}

#[wasm_bindgen_test]
fn test_reject_zero_transfer_amount() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 0 };
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qsender").unwrap();

    let transferable = Brc20TransferableInscriptions::new().get("xfer_0i0");
    assert!(transferable.is_none(), "Zero amount transfer should be rejected");
}

// ============================================================================
// ISSUE 7: MAX_AMOUNT overflow protection
// ============================================================================

#[wasm_bindgen_test]
fn test_reject_amount_exceeding_max() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "huge", "max": "999999999999999999999999999999999999999", "lim": "1" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_none(), "Amount exceeding MAX_AMOUNT should be rejected");
}

// ============================================================================
// ISSUE 8: lim defaults to max_supply when absent
// ============================================================================

#[wasm_bindgen_test]
fn test_deploy_lim_defaults_to_max() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ordi", "max": "21000000" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Deploy without 'lim' should default lim to max");
    match result.unwrap() {
        Brc20Operation::Deploy { max_supply, limit_per_mint, .. } => {
            assert_eq!(limit_per_mint, max_supply,
                "limit_per_mint should default to max_supply when 'lim' is absent");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

// ============================================================================
// ISSUE 9: Negative / invalid number rejection
// ============================================================================

#[wasm_bindgen_test]
fn test_reject_negative_amount() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "mint", "tick": "ordi", "amt": "-100" }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "Negative amount should be rejected");
}

#[wasm_bindgen_test]
fn test_reject_leading_dot_amount() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "mint", "tick": "ordi", "amt": ".5" }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "Leading dot should be rejected");
}

#[wasm_bindgen_test]
fn test_reject_trailing_dot_amount() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "mint", "tick": "ordi", "amt": "100." }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "Trailing dot should be rejected");
}

// ============================================================================
// ISSUE 10: Decimals validation
// ============================================================================

#[wasm_bindgen_test]
fn test_reject_decimals_over_18() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "test", "max": "1000", "lim": "100", "dec": "19" }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "Decimals > 18 should be rejected");
}

#[wasm_bindgen_test]
fn test_accept_decimals_0() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "sats", "max": "2100000000000000", "lim": "100000000", "dec": "0" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Decimals of 0 should be accepted");
    match result.unwrap() {
        Brc20Operation::Deploy { decimals, .. } => assert_eq!(decimals, 0),
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

// ============================================================================
// ISSUE 11: Zero max_supply rejection (non-self-mint)
// ============================================================================

#[wasm_bindgen_test]
fn test_reject_zero_max_supply_deploy() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "test", "max": "0", "lim": "100" }"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Zero max_supply should be rejected for non-self-mint deploy");
}

// ============================================================================
// Existing behavior tests that should CONTINUE to pass after fixes
// (These verify we don't regress working behavior)
// ============================================================================

#[wasm_bindgen_test]
fn test_regression_deploy_first_wins() {
    clear();
    let indexer = Brc20Indexer::new();
    let op1 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    let op2 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 99_999_999,
        limit_per_mint: 5000,
        decimals: 8,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&op1, "first_0i0", "bc1qa").unwrap();
    indexer.process_operation(&op2, "second_0i0", "bc1qb").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.max_supply, 21_000_000, "First deploy should win");
}

#[wasm_bindgen_test]
fn test_regression_mint_exceeds_limit_rejected() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1001 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.current_supply, 0, "Mint exceeding limit should be rejected");
}

#[wasm_bindgen_test]
fn test_regression_transfer_insufficient_balance_rejected() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 100 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    // Try to transfer 200 (more than the 100 available)
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 200 };
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qsender").unwrap();

    // Balance should be unchanged
    let bal_data = Brc20Balances::new().get("bc1qsender", "ordi").unwrap();
    let balance: Balance = serde_json::from_slice(&bal_data).unwrap();
    assert_eq!(balance.available_balance, 100);
    assert_eq!(balance.total_balance, 100);

    let transferable = Brc20TransferableInscriptions::new().get("xfer_0i0");
    assert!(transferable.is_none(), "Transfer should not be recorded");
}
