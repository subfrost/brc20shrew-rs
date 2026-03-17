///! Self-mint and extended ticker tests.
///!
///! These tests cover:
///! - 5-byte self-mint tickers (enabled at block 837,090)
///! - 6-byte predeploy tickers (enabled at block 912,690)
///! - Height-gated validation
///! - Self-mint specific deploy/mint rules

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::brc20::{Brc20Indexer, Brc20Operation, Ticker, MAX_AMOUNT};
use crate::tables::Brc20Tickers;
use shrew_test_helpers::state::clear;

const SCALE: u128 = 1_000_000_000_000_000_000u128; // 10^18

// ============================================================================
// 5-byte self-mint deploy tests
// ============================================================================

#[test]
fn test_self_mint_deploy_accepted_at_correct_height() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"1000","lim":"100","self_mint":"true"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_some(), "5-byte self-mint deploy should succeed at height 837090");
    match result.unwrap() {
        Brc20Operation::Deploy { ticker, self_mint, .. } => {
            assert_eq!(ticker, "abcde");
            assert!(self_mint, "self_mint flag should be true for 5-byte ticker");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_self_mint_deploy_rejected_before_activation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"1000","lim":"100","self_mint":"true"}"#;
    let result = indexer.parse_operation(content, 837089);
    assert!(result.is_none(), "5-byte ticker should be rejected before height 837090");
}

#[test]
fn test_self_mint_deploy_without_flag_rejected() {
    let indexer = Brc20Indexer::new();
    // 5-byte ticker without "self_mint":"true"
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_none(), "5-byte ticker without self_mint flag should be rejected");
}

#[test]
fn test_self_mint_deploy_wrong_flag_value_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"1000","lim":"100","self_mint":"false"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_none(), "self_mint='false' should be rejected for 5-byte ticker");
}

#[test]
fn test_self_mint_deploy_max_supply_zero_defaults_to_max_amount() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"0","lim":"1000","self_mint":"true"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_some(), "Self-mint with max_supply=0 should succeed (defaults to MAX_AMOUNT)");
    match result.unwrap() {
        Brc20Operation::Deploy { max_supply, .. } => {
            assert_eq!(max_supply, MAX_AMOUNT, "max_supply=0 should default to MAX_AMOUNT for self-mint");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_self_mint_deploy_lim_zero_defaults_to_max_amount() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"0","lim":"0","self_mint":"true"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_some(), "Self-mint with lim=0 should succeed (defaults to MAX_AMOUNT)");
    match result.unwrap() {
        Brc20Operation::Deploy { limit_per_mint, .. } => {
            assert_eq!(limit_per_mint, MAX_AMOUNT, "lim=0 should default to MAX_AMOUNT for self-mint");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_self_mint_deploy_explicit_supply_preserved() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"5000","lim":"100","self_mint":"true"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_some());
    match result.unwrap() {
        Brc20Operation::Deploy { max_supply, limit_per_mint, .. } => {
            assert_eq!(max_supply, 5000 * SCALE, "Explicit max_supply should be preserved");
            assert_eq!(limit_per_mint, 100 * SCALE, "Explicit lim should be preserved");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_self_mint_flag_stored_in_ticker() {
    clear();
    let indexer = Brc20Indexer::new();
    let op = Brc20Operation::Deploy {
        ticker: "abcde".to_string(),
        max_supply: 1000 * SCALE,
        limit_per_mint: 100 * SCALE,
        decimals: 18,
        self_mint: true,
    };
    indexer.process_operation(&op, "deploy_0i0", "bc1qdeployer").unwrap();

    let data = Brc20Tickers::new().get("abcde").expect("Ticker should exist");
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert!(ticker.is_self_mint, "is_self_mint should be true in stored ticker");
}

#[test]
fn test_regular_4byte_ticker_not_self_mint() {
    clear();
    let indexer = Brc20Indexer::new();
    let op = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&op, "deploy_0i0", "bc1qdeployer").unwrap();

    let data = Brc20Tickers::new().get("ordi").expect("Ticker should exist");
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert!(!ticker.is_self_mint, "4-byte ticker should not be self-mint");
}

// ============================================================================
// 6-byte predeploy ticker tests
// ============================================================================

#[test]
fn test_6byte_ticker_accepted_at_correct_height() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcdef","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_some(), "6-byte ticker should succeed at height 912690");
    match result.unwrap() {
        Brc20Operation::Deploy { ticker, self_mint, .. } => {
            assert_eq!(ticker, "abcdef");
            assert!(!self_mint, "6-byte ticker should not be self-mint");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_6byte_ticker_rejected_before_activation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcdef","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912689);
    assert!(result.is_none(), "6-byte ticker should be rejected before height 912690");
}

#[test]
fn test_6byte_ticker_alphanumeric_accepted() {
    let indexer = Brc20Indexer::new();
    // Alphanumeric + dash
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abc-12","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_some(), "6-byte alphanumeric+dash ticker should be accepted");
}

#[test]
fn test_6byte_ticker_underscore_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abc_12","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_none(), "6-byte ticker with underscore should be rejected");
}

#[test]
fn test_6byte_ticker_space_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abc 12","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_none(), "6-byte ticker with space should be rejected");
}

#[test]
fn test_6byte_ticker_special_char_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abc!@#","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_none(), "6-byte ticker with special chars should be rejected");
}

// ============================================================================
// Self-mint mint parent validation tests (process_operation level)
// Note: Parent validation in process_brc20_inscriptions requires indexed state.
// These test the stored is_self_mint flag behavior.
// ============================================================================

#[test]
fn test_self_mint_ticker_mint_via_process_operation() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy self-mint ticker
    let deploy = Brc20Operation::Deploy {
        ticker: "abcde".to_string(),
        max_supply: 1000 * SCALE,
        limit_per_mint: 100 * SCALE,
        decimals: 18,
        self_mint: true,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Regular mint (via process_operation, bypasses parent check which is in process_brc20_inscriptions)
    let mint = Brc20Operation::Mint {
        ticker: "abcde".to_string(),
        amount: 50 * SCALE,
    };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    // Verify ticker data
    let data = Brc20Tickers::new().get("abcde").expect("Ticker should exist");
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert!(ticker.is_self_mint);
    assert_eq!(ticker.current_supply, 50 * SCALE);
}

// ============================================================================
// 5-byte ticker mint parsing (parse_operation level)
// ============================================================================

#[test]
fn test_5byte_ticker_mint_parsed_at_correct_height() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"abcde","amt":"100"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_some(), "5-byte ticker mint should parse at height >= 837090");
}

#[test]
fn test_5byte_ticker_mint_rejected_before_activation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"abcde","amt":"100"}"#;
    let result = indexer.parse_operation(content, 837089);
    assert!(result.is_none(), "5-byte ticker mint should be rejected before height 837090");
}

#[test]
fn test_5byte_ticker_transfer_parsed_at_correct_height() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"transfer","tick":"abcde","amt":"100"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_some(), "5-byte ticker transfer should parse at height >= 837090");
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_4byte_ticker_unaffected_by_height() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"21000000","lim":"1000"}"#;
    // Should work at any height (even before self-mint activation)
    let result = indexer.parse_operation(content, 1);
    assert!(result.is_some(), "4-byte ticker should work at any height");
}

#[test]
fn test_self_mint_deploy_no_self_mint_flag_on_4byte_ignored() {
    let indexer = Brc20Indexer::new();
    // 4-byte ticker with self_mint field — should be ignored (not a 5-byte ticker)
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"21000000","lim":"1000","self_mint":"true"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "4-byte ticker with self_mint field should still deploy normally");
    match result.unwrap() {
        Brc20Operation::Deploy { self_mint, .. } => {
            assert!(!self_mint, "4-byte ticker should not be marked as self-mint");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}
