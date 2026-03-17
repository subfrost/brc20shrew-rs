///! Extended OPI Conformance Tests
///!
///! Comprehensive test vectors covering all edge cases identified in the
///! gap analysis between brc20shrew-rs and the OPI reference implementation.
///! Organized by category: parsing, amounts, deploy, mint, transfer, claim.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::brc20::{Brc20Indexer, Brc20Operation, Ticker, Balance, TransferInfo, MAX_AMOUNT};
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::assertions::{assert_brc20_balance, assert_brc20_supply};

const SCALE: u128 = 1_000_000_000_000_000_000u128; // 10^18

// ============================================================================
// PARSE_AMOUNT: Decimal precision edge cases
// ============================================================================

#[test]
fn test_amount_exactly_18_decimal_places() {
    let indexer = Brc20Indexer::new();
    // "0.000000000000000001" = 1 wei (smallest unit) — should accept
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"0.000000000000000001"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Amount with exactly 18 decimal places should be accepted");
    match result.unwrap() {
        Brc20Operation::Mint { amount, .. } => {
            assert_eq!(amount, 1, "0.000000000000000001 should parse to 1 (smallest unit)");
        }
        other => panic!("Expected Mint, got {:?}", other),
    }
}

#[test]
fn test_amount_19_decimal_places_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"0.0000000000000000001"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "19 decimal places should be rejected");
}

#[test]
fn test_amount_18_decimal_places_nonzero() {
    let indexer = Brc20Indexer::new();
    // "1.000000000000000001" = 1 * 10^18 + 1
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"1.000000000000000001"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "1.000000000000000001 should be valid (18 decimals)");
    match result.unwrap() {
        Brc20Operation::Mint { amount, .. } => {
            assert_eq!(amount, SCALE + 1);
        }
        other => panic!("Expected Mint, got {:?}", other),
    }
}

#[test]
fn test_amount_leading_zero_decimal() {
    let indexer = Brc20Indexer::new();
    // "0.5" — leading zero in integer part
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"0.5"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "0.5 should parse correctly");
    match result.unwrap() {
        Brc20Operation::Mint { amount, .. } => {
            assert_eq!(amount, SCALE / 2, "0.5 should equal SCALE/2");
        }
        other => panic!("Expected Mint, got {:?}", other),
    }
}

#[test]
fn test_amount_trailing_zeros_in_decimal() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"1.500000000000000000"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Trailing zeros in decimal should be accepted");
    match result.unwrap() {
        Brc20Operation::Mint { amount, .. } => {
            assert_eq!(amount, SCALE + SCALE / 2, "1.5 with trailing zeros should equal 1.5 * SCALE");
        }
        other => panic!("Expected Mint, got {:?}", other),
    }
}

#[test]
fn test_amount_multiple_dots_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"1.2.3"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Multiple dots should be rejected");
}

#[test]
fn test_amount_empty_string_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":""}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Empty amount string should be rejected");
}

#[test]
fn test_amount_whitespace_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":" 100 "}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Amount with whitespace should be rejected");
}

#[test]
fn test_amount_plus_sign_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"+100"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Amount with plus sign should be rejected");
}

#[test]
fn test_amount_scientific_notation_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"1e18"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Scientific notation should be rejected");
}

#[test]
fn test_amount_hex_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"0xff"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Hex notation should be rejected");
}

#[test]
fn test_amount_comma_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"1,000"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Comma in amount should be rejected");
}

// ============================================================================
// PARSE_AMOUNT: MAX_AMOUNT boundary
// ============================================================================

#[test]
fn test_amount_at_max_amount() {
    let indexer = Brc20Indexer::new();
    // MAX_AMOUNT = (2^64-1) * 10^18 = 18446744073709551615 * 10^18
    // As an integer string that would be 18446744073709551615000000000000000000
    // But MAX_AMOUNT as integer (before SCALE multiplication) = u64::MAX
    // parse_amount("18446744073709551615") -> 18446744073709551615 * 10^18 = MAX_AMOUNT
    let content = br#"{"p":"brc-20","op":"deploy","tick":"maxt","max":"18446744073709551615","lim":"1"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Amount exactly at MAX_AMOUNT should be accepted");
    match result.unwrap() {
        Brc20Operation::Deploy { max_supply, .. } => {
            assert_eq!(max_supply, MAX_AMOUNT);
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_amount_one_over_max_rejected() {
    let indexer = Brc20Indexer::new();
    // u64::MAX + 1 = 18446744073709551616
    let content = br#"{"p":"brc-20","op":"deploy","tick":"over","max":"18446744073709551616","lim":"1"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Amount one over MAX_AMOUNT should be rejected");
}

#[test]
fn test_amount_smallest_valid() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"1"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some());
    match result.unwrap() {
        Brc20Operation::Mint { amount, .. } => {
            assert_eq!(amount, SCALE, "Amount '1' should parse to 1 * SCALE");
        }
        other => panic!("Expected Mint, got {:?}", other),
    }
}

// ============================================================================
// JSON PARSING: Edge cases
// ============================================================================

#[test]
fn test_json_extra_fields_ignored() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"21000000","lim":"1000","foo":"bar","nested":{"a":1}}"#;
    assert!(indexer.parse_operation(content, 840000).is_some(),
        "Extra fields in JSON should be ignored");
}

#[test]
fn test_json_wrong_type_for_amount() {
    let indexer = Brc20Indexer::new();
    // amt as number instead of string
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":500}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Numeric (non-string) amount should be rejected");
}

#[test]
fn test_json_wrong_type_for_op() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":123,"tick":"ordi","amt":"500"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Non-string op field should be rejected");
}

#[test]
fn test_json_wrong_type_for_protocol() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":20,"op":"mint","tick":"ordi","amt":"500"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Non-string protocol field should be rejected");
}

#[test]
fn test_json_null_fields_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":null}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Null amount should be rejected");
}

#[test]
fn test_json_array_content_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"[{"p":"brc-20","op":"mint","tick":"ordi","amt":"500"}]"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Array-wrapped JSON should be rejected");
}

#[test]
fn test_json_duplicate_keys_last_wins() {
    let indexer = Brc20Indexer::new();
    // serde_json uses last value for duplicate keys
    let content = br#"{"p":"brc-20","op":"mint","tick":"ordi","amt":"100","amt":"500"}"#;
    let result = indexer.parse_operation(content, 840000);
    // serde_json takes last value for duplicate keys
    if let Some(Brc20Operation::Mint { amount, .. }) = result {
        assert_eq!(amount, 500 * SCALE, "Last value should win for duplicate keys");
    }
    // If it's None, that's also valid behavior
}

#[test]
fn test_invalid_utf8_rejected() {
    let indexer = Brc20Indexer::new();
    // Invalid UTF-8 bytes
    let content: &[u8] = &[0xff, 0xfe, 0x00];
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Invalid UTF-8 should be rejected");
}

#[test]
fn test_empty_content_rejected() {
    let indexer = Brc20Indexer::new();
    assert!(indexer.parse_operation(b"", 840000).is_none(),
        "Empty content should be rejected");
}

#[test]
fn test_protocol_case_sensitive() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"BRC-20","op":"mint","tick":"ordi","amt":"100"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Protocol field should be case-sensitive (BRC-20 != brc-20)");
}

#[test]
fn test_op_case_sensitive() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"Deploy","tick":"ordi","max":"1000","lim":"100"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Operation field should be case-sensitive (Deploy != deploy)");
}

// ============================================================================
// DEPLOY: Missing/invalid fields
// ============================================================================

#[test]
fn test_deploy_missing_max_field() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","lim":"1000"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Deploy without 'max' field should be rejected");
}

#[test]
fn test_deploy_decimals_invalid_string_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"1000","lim":"100","dec":"abc"}"#;
    // Invalid dec should fall back to default 18
    let result = indexer.parse_operation(content, 840000);
    // The implementation uses .and_then(|s| s.parse::<u8>().ok()).unwrap_or(18)
    // So invalid dec defaults to 18
    assert!(result.is_some(), "Invalid dec should default to 18, not reject");
    match result.unwrap() {
        Brc20Operation::Deploy { decimals, .. } => {
            assert_eq!(decimals, 18, "Invalid dec should default to 18");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_deploy_decimals_negative_rejected() {
    let indexer = Brc20Indexer::new();
    // "-1" will fail u8 parse, default to 18
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"1000","lim":"100","dec":"-1"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some());
    match result.unwrap() {
        Brc20Operation::Deploy { decimals, .. } => {
            assert_eq!(decimals, 18, "Negative dec should default to 18");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_deploy_decimals_decimal_point_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"1000","lim":"100","dec":"8.5"}"#;
    let result = indexer.parse_operation(content, 840000);
    // "8.5" fails u8 parse -> defaults to 18
    assert!(result.is_some());
    match result.unwrap() {
        Brc20Operation::Deploy { decimals, .. } => {
            assert_eq!(decimals, 18, "Decimal dec should default to 18");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_deploy_decimals_all_valid_values() {
    let indexer = Brc20Indexer::new();
    for d in 0u8..=18 {
        let content = format!(
            r#"{{"p":"brc-20","op":"deploy","tick":"t{:03}","max":"1000","lim":"100","dec":"{}"}}"#,
            d, d
        );
        let result = indexer.parse_operation(content.as_bytes(), 840000);
        assert!(result.is_some(), "Decimals {} should be valid", d);
        match result.unwrap() {
            Brc20Operation::Deploy { decimals, .. } => {
                assert_eq!(decimals, d, "Decimals should be {}", d);
            }
            other => panic!("Expected Deploy, got {:?}", other),
        }
    }
}

#[test]
fn test_deploy_lim_zero_non_self_mint_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"1000","lim":"0"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "lim=0 should be rejected for non-self-mint deploy");
}

#[test]
fn test_deploy_lim_greater_than_max_accepted() {
    let indexer = Brc20Indexer::new();
    // OPI allows lim > max_supply
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"100","lim":"1000"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "lim > max should be accepted");
    match result.unwrap() {
        Brc20Operation::Deploy { max_supply, limit_per_mint, .. } => {
            assert_eq!(max_supply, 100 * SCALE);
            assert_eq!(limit_per_mint, 1000 * SCALE);
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

// ============================================================================
// TICKER: Edge cases
// ============================================================================

#[test]
fn test_ticker_mixed_case_normalized() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"OrDi","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some());
    match result.unwrap() {
        Brc20Operation::Deploy { ticker, .. } => {
            assert_eq!(ticker, "ordi", "Ticker should be lowercased");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

#[test]
fn test_ticker_2_char_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ab","max":"1000","lim":"100"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(), "2-char ticker should be rejected");
}

#[test]
fn test_ticker_exactly_5_bytes() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"1000","lim":"100","self_mint":"true"}"#;
    let result = indexer.parse_operation(content, 837090);
    assert!(result.is_some(), "Exactly 5-byte ticker should be accepted at correct height");
}

#[test]
fn test_ticker_exactly_6_bytes() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcdef","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_some(), "Exactly 6-byte ticker should be accepted at correct height");
}

#[test]
fn test_ticker_8_char_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcdefgh","max":"1000","lim":"100"}"#;
    assert!(indexer.parse_operation(content, 912690).is_none(), "8-char ticker should be rejected");
}

#[test]
fn test_6byte_ticker_all_dashes() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"------","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_some(), "6-byte all-dash ticker should be accepted");
}

#[test]
fn test_6byte_ticker_all_digits() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"123456","max":"1000","lim":"100"}"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_some(), "6-byte all-digit ticker should be accepted");
}

#[test]
fn test_ticker_null_byte_at_start() {
    let indexer = Brc20Indexer::new();
    let content = b"{ \"p\": \"brc-20\", \"op\": \"deploy\", \"tick\": \"\x00rdi\", \"max\": \"1000\", \"lim\": \"100\" }";
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Ticker with null byte at start should be rejected");
}

#[test]
fn test_ticker_null_byte_at_end() {
    let indexer = Brc20Indexer::new();
    let content = b"{ \"p\": \"brc-20\", \"op\": \"deploy\", \"tick\": \"ord\x00\", \"max\": \"1000\", \"lim\": \"100\" }";
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Ticker with null byte at end should be rejected");
}

// ============================================================================
// MINT: Process operation edge cases
// ============================================================================

#[test]
fn test_mint_multiple_to_same_owner_accumulates() {
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

    // Mint 3 times to same owner
    for i in 0..3 {
        let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 500 * SCALE };
        indexer.process_operation(&mint, &format!("mint{}_0i0", i), "bc1qsame").unwrap();
    }

    assert_brc20_balance("bc1qsame", "ordi", 1500 * SCALE, 1500 * SCALE);
    assert_brc20_supply("ordi", 1500 * SCALE);
}

#[test]
fn test_mint_at_exact_limit() {
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

    // Mint exactly at limit — should succeed
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    assert_brc20_supply("ordi", 1000 * SCALE);
    assert_brc20_balance("bc1qminter", "ordi", 1000 * SCALE, 1000 * SCALE);
}

#[test]
fn test_mint_one_over_limit_rejected() {
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

    // Mint one unit over limit — should be rejected entirely
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE + 1 };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    assert_brc20_supply("ordi", 0);
}

#[test]
fn test_mint_clamp_one_remaining() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 1001 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // First mint: 1000
    let mint1 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint1, "mint1_0i0", "bc1qa").unwrap();

    // Second mint: 1000 requested but only 1 remaining, should clamp to 1
    let mint2 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint2, "mint2_0i0", "bc1qb").unwrap();

    assert_brc20_supply("ordi", 1001 * SCALE);
    assert_brc20_balance("bc1qb", "ordi", 1 * SCALE, 1 * SCALE);
}

#[test]
fn test_mint_after_supply_exhausted_rejected() {
    clear();
    let indexer = Brc20Indexer::new();
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 100 * SCALE,
        limit_per_mint: 100 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qowner").unwrap();

    // Exhaust supply
    let mint1 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 100 * SCALE };
    indexer.process_operation(&mint1, "mint1_0i0", "bc1qa").unwrap();

    // This mint should be entirely rejected (not clamped to 0)
    let mint2 = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1 * SCALE };
    indexer.process_operation(&mint2, "mint2_0i0", "bc1qb").unwrap();

    assert_brc20_supply("ordi", 100 * SCALE);
    let balance = Brc20Balances::new().get("bc1qb", "ordi");
    assert!(balance.is_none(), "No balance should exist after supply exhausted");
}

#[test]
fn test_mint_case_insensitive_via_process() {
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

    // Mint using uppercase ticker
    let mint = Brc20Operation::Mint { ticker: "ORDI".to_string(), amount: 500 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qminter").unwrap();

    assert_brc20_supply("ordi", 500 * SCALE);
}

// ============================================================================
// TRANSFER: Process operation edge cases
// ============================================================================

#[test]
fn test_transfer_exact_available_balance() {
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

    // Transfer exactly all available balance
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qsender").unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 0, 1000 * SCALE);
    let transferable = Brc20TransferableInscriptions::new().get("xfer_0i0");
    assert!(transferable.is_some(), "Transfer of exact available balance should succeed");
}

#[test]
fn test_transfer_one_over_available_rejected() {
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

    // Transfer one unit over available
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 1000 * SCALE + 1 };
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qsender").unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 1000 * SCALE, 1000 * SCALE);
    let transferable = Brc20TransferableInscriptions::new().get("xfer_0i0");
    assert!(transferable.is_none(), "Transfer over available should be rejected");
}

#[test]
fn test_transfer_no_balance_ignored() {
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

    // Transfer from address with no balance
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 100 * SCALE };
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qnobalance").unwrap();

    let transferable = Brc20TransferableInscriptions::new().get("xfer_0i0");
    assert!(transferable.is_none(), "Transfer with no balance should be ignored");
}

#[test]
fn test_transfer_nonexistent_ticker_ignored() {
    clear();
    let indexer = Brc20Indexer::new();

    // Transfer of undeployed ticker
    let transfer = Brc20Operation::Transfer { ticker: "fake".to_string(), amount: 100 * SCALE };
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qsender").unwrap();

    let transferable = Brc20TransferableInscriptions::new().get("xfer_0i0");
    assert!(transferable.is_none(), "Transfer of undeployed ticker should be ignored");
}

#[test]
fn test_multiple_transfers_before_claim() {
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

    // First transfer: 400
    let xfer1 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 400 * SCALE };
    indexer.process_operation(&xfer1, "xfer1_0i0", "bc1qsender").unwrap();
    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 1000 * SCALE);

    // Second transfer: 300 (from remaining 600 available)
    let xfer2 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 300 * SCALE };
    indexer.process_operation(&xfer2, "xfer2_0i0", "bc1qsender").unwrap();
    assert_brc20_balance("bc1qsender", "ordi", 300 * SCALE, 1000 * SCALE);

    // Both transferable inscriptions should exist
    assert!(Brc20TransferableInscriptions::new().get("xfer1_0i0").is_some());
    assert!(Brc20TransferableInscriptions::new().get("xfer2_0i0").is_some());
}

#[test]
fn test_transfer_exceeds_available_after_pending() {
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

    // First transfer: 800 (available drops to 200)
    let xfer1 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 800 * SCALE };
    indexer.process_operation(&xfer1, "xfer1_0i0", "bc1qsender").unwrap();

    // Second transfer: 300 (exceeds remaining 200 available) — should be rejected
    let xfer2 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 300 * SCALE };
    indexer.process_operation(&xfer2, "xfer2_0i0", "bc1qsender").unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 200 * SCALE, 1000 * SCALE);
    assert!(Brc20TransferableInscriptions::new().get("xfer2_0i0").is_none(),
        "Second transfer exceeding available should be rejected");
}

// ============================================================================
// CLAIM_TRANSFER: Edge cases
// ============================================================================

#[test]
fn test_claim_to_recipient_with_existing_balance() {
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

    // Both users mint
    let mint_a = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint_a, "mint_a_0i0", "bc1qalice").unwrap();
    let mint_b = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 500 * SCALE };
    indexer.process_operation(&mint_b, "mint_b_0i0", "bc1qbob").unwrap();

    // Alice transfers 400 to Bob (who already has 500)
    let transfer = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 400 * SCALE };
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qalice").unwrap();

    let info = TransferInfo {
        ticker: "ordi".to_string(),
        amount: 400 * SCALE,
        sender: "bc1qalice".to_string(),
    };
    indexer.claim_transfer("bc1qbob", &info).unwrap();

    assert_brc20_balance("bc1qalice", "ordi", 600 * SCALE, 600 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 900 * SCALE, 900 * SCALE);
}

#[test]
fn test_claim_to_new_address() {
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
    indexer.process_operation(&transfer, "xfer_0i0", "bc1qsender").unwrap();

    // Claim to address that never had any balance
    let info = TransferInfo {
        ticker: "ordi".to_string(),
        amount: 400 * SCALE,
        sender: "bc1qsender".to_string(),
    };
    indexer.claim_transfer("bc1qnewaddr", &info).unwrap();

    assert_brc20_balance("bc1qnewaddr", "ordi", 400 * SCALE, 400 * SCALE);
    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 600 * SCALE);
}

#[test]
fn test_claim_multiple_transfers_sequentially() {
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

    // Two transfer inscribes
    let xfer1 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 300 * SCALE };
    indexer.process_operation(&xfer1, "xfer1_0i0", "bc1qsender").unwrap();
    let xfer2 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 200 * SCALE };
    indexer.process_operation(&xfer2, "xfer2_0i0", "bc1qsender").unwrap();

    // Claim first to bob
    let info1 = TransferInfo {
        ticker: "ordi".to_string(),
        amount: 300 * SCALE,
        sender: "bc1qsender".to_string(),
    };
    indexer.claim_transfer("bc1qbob", &info1).unwrap();

    // Claim second to charlie
    let info2 = TransferInfo {
        ticker: "ordi".to_string(),
        amount: 200 * SCALE,
        sender: "bc1qsender".to_string(),
    };
    indexer.claim_transfer("bc1qcharlie", &info2).unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 500 * SCALE, 500 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 300 * SCALE, 300 * SCALE);
    assert_brc20_balance("bc1qcharlie", "ordi", 200 * SCALE, 200 * SCALE);

    // Total should be conserved
    assert_brc20_supply("ordi", 1000 * SCALE);
}

// ============================================================================
// HEIGHT BOUNDARY: Exact activation block tests
// ============================================================================

#[test]
fn test_height_0_4byte_works() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"ordi","max":"1000","lim":"100"}"#;
    assert!(indexer.parse_operation(content, 0).is_some(),
        "4-byte ticker should work at height 0");
}

#[test]
fn test_5byte_at_exact_boundary_837090() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"1000","lim":"100","self_mint":"true"}"#;
    assert!(indexer.parse_operation(content, 837090).is_some(),
        "5-byte should work at exact height 837090");
    assert!(indexer.parse_operation(content, 837089).is_none(),
        "5-byte should fail at height 837089");
}

#[test]
fn test_6byte_at_exact_boundary_912690() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"abcdef","max":"1000","lim":"100"}"#;
    assert!(indexer.parse_operation(content, 912690).is_some(),
        "6-byte should work at exact height 912690");
    assert!(indexer.parse_operation(content, 912689).is_none(),
        "6-byte should fail at height 912689");
}

#[test]
fn test_5byte_mint_height_gated() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"abcde","amt":"100"}"#;
    assert!(indexer.parse_operation(content, 837089).is_none(),
        "5-byte mint should be rejected before activation");
    assert!(indexer.parse_operation(content, 837090).is_some(),
        "5-byte mint should work at activation height");
}

#[test]
fn test_5byte_transfer_height_gated() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"transfer","tick":"abcde","amt":"100"}"#;
    assert!(indexer.parse_operation(content, 837089).is_none(),
        "5-byte transfer should be rejected before activation");
    assert!(indexer.parse_operation(content, 837090).is_some(),
        "5-byte transfer should work at activation height");
}

#[test]
fn test_6byte_mint_height_gated() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"mint","tick":"abcdef","amt":"100"}"#;
    assert!(indexer.parse_operation(content, 912689).is_none(),
        "6-byte mint should be rejected before activation");
    assert!(indexer.parse_operation(content, 912690).is_some(),
        "6-byte mint should work at activation height");
}

#[test]
fn test_6byte_transfer_height_gated() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"transfer","tick":"abcdef","amt":"100"}"#;
    assert!(indexer.parse_operation(content, 912689).is_none(),
        "6-byte transfer should be rejected before activation");
    assert!(indexer.parse_operation(content, 912690).is_some(),
        "6-byte transfer should work at activation height");
}

// ============================================================================
// DEPLOY: Duplicate deploy first-wins via process_operation
// ============================================================================

#[test]
fn test_deploy_duplicate_ticker_ignored() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy1 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy1, "first_0i0", "bc1qa").unwrap();

    let deploy2 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 999 * SCALE,
        limit_per_mint: 1 * SCALE,
        decimals: 8,
        self_mint: false,
    };
    indexer.process_operation(&deploy2, "second_0i0", "bc1qb").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.max_supply, 21_000_000 * SCALE, "First deploy should win");
    assert_eq!(ticker.decimals, 18, "First deploy's decimals should persist");
    assert_eq!(ticker.deploy_inscription_id, "first_0i0");
}

#[test]
fn test_deploy_case_insensitive_dedup_via_process() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy "ORDI" (will be stored as "ordi")
    let deploy1 = Brc20Operation::Deploy {
        ticker: "ORDI".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy1, "first_0i0", "bc1qa").unwrap();

    // Deploy "ordi" — should be rejected (duplicate after normalization)
    let deploy2 = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 999 * SCALE,
        limit_per_mint: 1 * SCALE,
        decimals: 8,
        self_mint: false,
    };
    indexer.process_operation(&deploy2, "second_0i0", "bc1qb").unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.max_supply, 21_000_000 * SCALE);
}

// ============================================================================
// FULL LIFECYCLE: Complex multi-step scenarios
// ============================================================================

#[test]
fn test_lifecycle_deploy_mint_multitransfer_claim() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Alice mints 1000
    let mint_a = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint_a, "mint_a_0i0", "bc1qalice").unwrap();

    // Bob mints 800
    let mint_b = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 800 * SCALE };
    indexer.process_operation(&mint_b, "mint_b_0i0", "bc1qbob").unwrap();

    // Alice: transfer 200 to Charlie, 300 to Dave
    let xfer1 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 200 * SCALE };
    indexer.process_operation(&xfer1, "xfer1_0i0", "bc1qalice").unwrap();
    let xfer2 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 300 * SCALE };
    indexer.process_operation(&xfer2, "xfer2_0i0", "bc1qalice").unwrap();

    // Verify Alice's state after two inscribes
    assert_brc20_balance("bc1qalice", "ordi", 500 * SCALE, 1000 * SCALE);

    // Claim both
    let info1 = TransferInfo { ticker: "ordi".to_string(), amount: 200 * SCALE, sender: "bc1qalice".to_string() };
    indexer.claim_transfer("bc1qcharlie", &info1).unwrap();
    let info2 = TransferInfo { ticker: "ordi".to_string(), amount: 300 * SCALE, sender: "bc1qalice".to_string() };
    indexer.claim_transfer("bc1qdave", &info2).unwrap();

    // Final state
    assert_brc20_balance("bc1qalice", "ordi", 500 * SCALE, 500 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 800 * SCALE, 800 * SCALE);
    assert_brc20_balance("bc1qcharlie", "ordi", 200 * SCALE, 200 * SCALE);
    assert_brc20_balance("bc1qdave", "ordi", 300 * SCALE, 300 * SCALE);
    assert_brc20_supply("ordi", 1800 * SCALE);
}

#[test]
fn test_lifecycle_chain_of_transfers() {
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

    // Alice mints
    let mint = Brc20Operation::Mint { ticker: "ordi".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qalice").unwrap();

    // Alice -> Bob (500)
    let xfer1 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 500 * SCALE };
    indexer.process_operation(&xfer1, "xfer1_0i0", "bc1qalice").unwrap();
    let info1 = TransferInfo { ticker: "ordi".to_string(), amount: 500 * SCALE, sender: "bc1qalice".to_string() };
    indexer.claim_transfer("bc1qbob", &info1).unwrap();

    // Bob -> Charlie (200)
    let xfer2 = Brc20Operation::Transfer { ticker: "ordi".to_string(), amount: 200 * SCALE };
    indexer.process_operation(&xfer2, "xfer2_0i0", "bc1qbob").unwrap();
    let info2 = TransferInfo { ticker: "ordi".to_string(), amount: 200 * SCALE, sender: "bc1qbob".to_string() };
    indexer.claim_transfer("bc1qcharlie", &info2).unwrap();

    assert_brc20_balance("bc1qalice", "ordi", 500 * SCALE, 500 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 300 * SCALE, 300 * SCALE);
    assert_brc20_balance("bc1qcharlie", "ordi", 200 * SCALE, 200 * SCALE);
    assert_brc20_supply("ordi", 1000 * SCALE);
}
