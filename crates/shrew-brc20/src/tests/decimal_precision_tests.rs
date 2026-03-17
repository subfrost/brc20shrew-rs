///! Decimal Precision Tests
///!
///! Tests for ticker-specific decimal precision validation per OPI spec.
///! When a ticker is deployed with dec=N, all amounts (max, lim, mint, transfer)
///! must have at most N decimal places.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::brc20::{Brc20Indexer, Brc20Operation, Ticker, Balance, TransferInfo};
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::assertions::{assert_brc20_balance, assert_brc20_supply};

const SCALE: u128 = 1_000_000_000_000_000_000u128;

// ============================================================================
// parse_amount with ticker_decimals parameter
// ============================================================================

#[test]
fn test_parse_amount_dec18_allows_18_places() {
    let result = Brc20Indexer::parse_amount("1.000000000000000001", 18);
    assert!(result.is_some(), "18 decimal places should be allowed with dec=18");
    assert_eq!(result.unwrap(), SCALE + 1);
}

#[test]
fn test_parse_amount_dec18_rejects_19_places() {
    let result = Brc20Indexer::parse_amount("1.0000000000000000001", 18);
    assert!(result.is_none(), "19 decimal places should be rejected with dec=18");
}

#[test]
fn test_parse_amount_dec0_rejects_any_decimal() {
    let result = Brc20Indexer::parse_amount("1.5", 0);
    assert!(result.is_none(), "Any decimal places should be rejected with dec=0");
}

#[test]
fn test_parse_amount_dec0_accepts_integer() {
    let result = Brc20Indexer::parse_amount("100", 0);
    assert!(result.is_some(), "Integer should be accepted with dec=0");
    assert_eq!(result.unwrap(), 100 * SCALE);
}

#[test]
fn test_parse_amount_dec2_accepts_2_places() {
    let result = Brc20Indexer::parse_amount("1.23", 2);
    assert!(result.is_some(), "2 decimal places should be allowed with dec=2");
    // 1.23 * 10^18 = 1230000000000000000
    assert_eq!(result.unwrap(), 1_230_000_000_000_000_000u128);
}

#[test]
fn test_parse_amount_dec2_rejects_3_places() {
    let result = Brc20Indexer::parse_amount("1.234", 2);
    assert!(result.is_none(), "3 decimal places should be rejected with dec=2");
}

#[test]
fn test_parse_amount_dec2_accepts_1_place() {
    let result = Brc20Indexer::parse_amount("1.2", 2);
    assert!(result.is_some(), "1 decimal place should be allowed with dec=2 (fewer than max)");
}

#[test]
fn test_parse_amount_dec2_accepts_integer() {
    let result = Brc20Indexer::parse_amount("100", 2);
    assert!(result.is_some(), "Integer should always be accepted regardless of dec");
    assert_eq!(result.unwrap(), 100 * SCALE);
}

#[test]
fn test_parse_amount_dec8_sats_style() {
    // Bitcoin-style: 8 decimal places like satoshis
    let result = Brc20Indexer::parse_amount("1.12345678", 8);
    assert!(result.is_some());
    let result2 = Brc20Indexer::parse_amount("1.123456789", 8);
    assert!(result2.is_none(), "9 decimal places should be rejected with dec=8");
}

#[test]
fn test_parse_amount_dec1_boundary() {
    assert!(Brc20Indexer::parse_amount("1.1", 1).is_some());
    assert!(Brc20Indexer::parse_amount("1.12", 1).is_none());
}

// ============================================================================
// Deploy: max and lim validated against dec
// ============================================================================

#[test]
fn test_deploy_max_exceeds_dec_rejected() {
    let indexer = Brc20Indexer::new();
    // Deploy with dec=2 but max has 3 decimal places
    let content = br#"{"p":"brc-20","op":"deploy","tick":"sats","max":"1000.123","lim":"100","dec":"2"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Deploy max with more decimals than dec should be rejected");
}

#[test]
fn test_deploy_max_within_dec_accepted() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"sats","max":"1000.12","lim":"100","dec":"2"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Deploy max within dec precision should be accepted");
}

#[test]
fn test_deploy_lim_exceeds_dec_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"sats","max":"1000","lim":"100.123","dec":"2"}"#;
    assert!(indexer.parse_operation(content, 840000).is_none(),
        "Deploy lim with more decimals than dec should be rejected");
}

#[test]
fn test_deploy_lim_within_dec_accepted() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"sats","max":"1000","lim":"100.12","dec":"2"}"#;
    assert!(indexer.parse_operation(content, 840000).is_some());
}

#[test]
fn test_deploy_dec0_integer_only() {
    let indexer = Brc20Indexer::new();
    // dec=0: only integers allowed
    let content = br#"{"p":"brc-20","op":"deploy","tick":"sats","max":"2100000000000000","lim":"100000000","dec":"0"}"#;
    assert!(indexer.parse_operation(content, 840000).is_some(),
        "Integer max/lim should work with dec=0");

    // Now with decimal in max
    let content2 = br#"{"p":"brc-20","op":"deploy","tick":"sat2","max":"2100000000000000.1","lim":"100000000","dec":"0"}"#;
    assert!(indexer.parse_operation(content2, 840000).is_none(),
        "Decimal max should be rejected with dec=0");
}

#[test]
fn test_deploy_dec8_bitcoin_style() {
    let indexer = Brc20Indexer::new();
    let content = br#"{"p":"brc-20","op":"deploy","tick":"sats","max":"21000000.00000000","lim":"1000.00000000","dec":"8"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "8 decimal places with dec=8 should work");

    let content2 = br#"{"p":"brc-20","op":"deploy","tick":"sat2","max":"21000000.000000001","lim":"1000","dec":"8"}"#;
    assert!(indexer.parse_operation(content2, 840000).is_none(),
        "9 decimal places should be rejected with dec=8");
}

// ============================================================================
// Mint: amount validated against deployed ticker's dec
// ============================================================================

#[test]
fn test_mint_respects_ticker_decimals() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy with dec=2
    let content = br#"{"p":"brc-20","op":"deploy","tick":"sats","max":"1000000","lim":"1000","dec":"2"}"#;
    indexer.parse_operation(content, 840000); // Just to verify it parses
    let deploy = Brc20Operation::Deploy {
        ticker: "sats".to_string(),
        max_supply: 1_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 2,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Mint with 2 decimal places — should succeed
    let mint_content = br#"{"p":"brc-20","op":"mint","tick":"sats","amt":"500.12"}"#;
    let result = indexer.parse_operation(mint_content, 840000);
    assert!(result.is_some(), "Mint with 2 decimals on dec=2 ticker should be accepted");

    // Mint with 3 decimal places — should be rejected
    let mint_content3 = br#"{"p":"brc-20","op":"mint","tick":"sats","amt":"500.123"}"#;
    let result3 = indexer.parse_operation(mint_content3, 840000);
    assert!(result3.is_none(), "Mint with 3 decimals on dec=2 ticker should be rejected");
}

#[test]
fn test_mint_dec0_rejects_decimal_amount() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "sats".to_string(),
        max_supply: 2_100_000_000_000_000u128 * SCALE,
        limit_per_mint: 100_000_000u128 * SCALE,
        decimals: 0,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Attempt to mint with decimal on dec=0 ticker
    let content = br#"{"p":"brc-20","op":"mint","tick":"sats","amt":"100.5"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_none(), "Decimal mint should be rejected on dec=0 ticker");

    // Integer mint should work
    let content2 = br#"{"p":"brc-20","op":"mint","tick":"sats","amt":"100"}"#;
    let result2 = indexer.parse_operation(content2, 840000);
    assert!(result2.is_some(), "Integer mint should work on dec=0 ticker");
}

#[test]
fn test_mint_undeployed_ticker_allows_18() {
    clear();
    let indexer = Brc20Indexer::new();

    // Mint a ticker that hasn't been deployed — falls back to 18 decimals
    // (will be rejected later in process_operation due to no ticker, but parsing should succeed)
    let content = br#"{"p":"brc-20","op":"mint","tick":"fake","amt":"1.123456789012345678"}"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "Undeployed ticker should allow up to 18 decimals at parse time");
}

// ============================================================================
// Transfer: amount validated against deployed ticker's dec
// ============================================================================

#[test]
fn test_transfer_respects_ticker_decimals() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "sats".to_string(),
        max_supply: 1_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 2,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Transfer with 2 decimal places — should succeed
    let xfer_content = br#"{"p":"brc-20","op":"transfer","tick":"sats","amt":"100.12"}"#;
    let result = indexer.parse_operation(xfer_content, 840000);
    assert!(result.is_some(), "Transfer with 2 decimals on dec=2 ticker should work");

    // Transfer with 3 decimal places — should be rejected
    let xfer_content3 = br#"{"p":"brc-20","op":"transfer","tick":"sats","amt":"100.123"}"#;
    let result3 = indexer.parse_operation(xfer_content3, 840000);
    assert!(result3.is_none(), "Transfer with 3 decimals on dec=2 ticker should be rejected");
}

// ============================================================================
// Full lifecycle with non-18 decimals
// ============================================================================

#[test]
fn test_lifecycle_dec2_deploy_mint_transfer() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy with dec=2
    let deploy = Brc20Operation::Deploy {
        ticker: "usd2".to_string(),
        max_supply: Brc20Indexer::parse_amount("1000000", 2).unwrap(),
        limit_per_mint: Brc20Indexer::parse_amount("1000", 2).unwrap(),
        decimals: 2,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Mint 100.50 (2 decimal places — valid)
    let mint_amt = Brc20Indexer::parse_amount("100.50", 2).unwrap();
    let mint = Brc20Operation::Mint { ticker: "usd2".to_string(), amount: mint_amt };
    indexer.process_operation(&mint, "mint_0i0", "bc1qalice").unwrap();

    assert_brc20_supply("usd2", mint_amt);
    assert_brc20_balance("bc1qalice", "usd2", mint_amt, mint_amt);

    // Transfer 25.25 (2 decimal places — valid)
    let xfer_amt = Brc20Indexer::parse_amount("25.25", 2).unwrap();
    let xfer = Brc20Operation::Transfer { ticker: "usd2".to_string(), amount: xfer_amt };
    indexer.process_operation(&xfer, "xfer_0i0", "bc1qalice").unwrap();

    let remaining = mint_amt - xfer_amt;
    assert_brc20_balance("bc1qalice", "usd2", remaining, mint_amt);

    // Claim
    let info = TransferInfo { ticker: "usd2".to_string(), amount: xfer_amt, sender: "bc1qalice".to_string() };
    indexer.claim_transfer("bc1qbob", &info).unwrap();

    assert_brc20_balance("bc1qalice", "usd2", remaining, remaining);
    assert_brc20_balance("bc1qbob", "usd2", xfer_amt, xfer_amt);
}

#[test]
fn test_lifecycle_dec0_integer_only() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "coin".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 0,
        self_mint: false,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Integer mint should work
    let mint = Brc20Operation::Mint { ticker: "coin".to_string(), amount: 100 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qalice").unwrap();
    assert_brc20_supply("coin", 100 * SCALE);

    // Integer transfer should work
    let xfer = Brc20Operation::Transfer { ticker: "coin".to_string(), amount: 50 * SCALE };
    indexer.process_operation(&xfer, "xfer_0i0", "bc1qalice").unwrap();
    assert_brc20_balance("bc1qalice", "coin", 50 * SCALE, 100 * SCALE);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_parse_amount_dec0_trailing_dot_rejected() {
    // "100." should be rejected regardless of dec (trailing dot)
    assert!(Brc20Indexer::parse_amount("100.", 0).is_none());
    assert!(Brc20Indexer::parse_amount("100.", 18).is_none());
}

#[test]
fn test_parse_amount_dec0_leading_dot_rejected() {
    assert!(Brc20Indexer::parse_amount(".5", 0).is_none());
    assert!(Brc20Indexer::parse_amount(".5", 18).is_none());
}

#[test]
fn test_all_dec_values_accept_integer() {
    // Integer amounts should always be accepted for any dec value
    for d in 0u8..=18 {
        assert!(Brc20Indexer::parse_amount("1000", d).is_some(),
            "Integer should be accepted with dec={}", d);
    }
}

#[test]
fn test_all_dec_values_boundary() {
    // For each dec value, test that exactly dec decimal places works
    // and dec+1 decimal places doesn't
    for d in 0u8..=17 {
        // Build a string with exactly d decimal places
        let valid = if d == 0 {
            "100".to_string()
        } else {
            let decimals: String = (0..d).map(|i| char::from(b'1' + (i % 9))).collect();
            format!("1.{}", decimals)
        };
        assert!(Brc20Indexer::parse_amount(&valid, d).is_some(),
            "Exactly {} decimal places should work with dec={}", d, d);

        // Build a string with d+1 decimal places
        let decimals_plus1: String = (0..=d).map(|i| char::from(b'1' + (i % 9))).collect();
        let invalid = format!("1.{}", decimals_plus1);
        assert!(Brc20Indexer::parse_amount(&invalid, d).is_none(),
            "{} decimal places should be rejected with dec={}", d + 1, d);
    }
}
