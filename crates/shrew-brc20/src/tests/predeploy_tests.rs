///! 6-byte Ticker Pre-deploy Snipe Protection Tests
///!
///! Per BRC-20 proposal 001:
///! - 6-byte tickers require a "predeploy" inscription with hash = sha256(sha256(ticker + salt + pkscript))
///! - Deploy inscription must be a child of the predeploy inscription
///! - Deploy must be at least 3 blocks after the predeploy
///! - Predeploy accepted from block 912680; 6-byte deploys from 912690
///! - Salt field is required in the deploy JSON for 6-byte tickers
///! - Hash format: double sha256 of (ticker_utf8 + salt_hex_decoded + pkscript_hex_decoded)

use crate::brc20::{Brc20Indexer, Brc20Operation};
use crate::tables::Brc20Tickers;
use shrew_test_helpers::state::clear;
use wasm_bindgen_test::wasm_bindgen_test;

// ============================================================================
// TEST 1: parse_operation recognizes "predeploy" op
// ============================================================================

#[wasm_bindgen_test]
fn test_parse_predeploy_operation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "predeploy", "hash": "e87852f99ef17cae507d1eda1ddd29c2271812e05d9af33abf1e6301eba83618" }"#;
    let result = indexer.parse_operation(content, 912680);
    assert!(
        result.is_some(),
        "predeploy operation should be parsed at or after predeploy activation height"
    );
    match result.unwrap() {
        Brc20Operation::Predeploy { hash } => {
            assert_eq!(hash, "e87852f99ef17cae507d1eda1ddd29c2271812e05d9af33abf1e6301eba83618");
        }
        other => panic!("Expected Predeploy, got {:?}", other),
    }
}

// ============================================================================
// TEST 2: predeploy rejected before activation height 912680
// ============================================================================

#[wasm_bindgen_test]
fn test_predeploy_rejected_before_activation() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "predeploy", "hash": "e87852f99ef17cae507d1eda1ddd29c2271812e05d9af33abf1e6301eba83618" }"#;
    let result = indexer.parse_operation(content, 912679);
    assert!(result.is_none(), "predeploy should be rejected before block 912680");
}

// ============================================================================
// TEST 3: predeploy requires hash field
// ============================================================================

#[wasm_bindgen_test]
fn test_predeploy_requires_hash() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "predeploy" }"#;
    let result = indexer.parse_operation(content, 912680);
    assert!(result.is_none(), "predeploy without hash should be rejected");
}

// ============================================================================
// TEST 4: 6-byte deploy parse includes salt field
// ============================================================================

#[wasm_bindgen_test]
fn test_parse_6byte_deploy_with_salt() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ticker", "max": "21000000", "lim": "1000", "salt": "73616c74" }"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_some(), "6-byte deploy with salt should parse");
    match result.unwrap() {
        Brc20Operation::Deploy { ticker, salt, .. } => {
            assert_eq!(ticker, "ticker");
            assert_eq!(salt, Some("73616c74".to_string()), "salt should be captured");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

// ============================================================================
// TEST 5: 6-byte deploy without salt is rejected
// ============================================================================

#[wasm_bindgen_test]
fn test_6byte_deploy_without_salt_rejected() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ticker", "max": "21000000", "lim": "1000" }"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_none(), "6-byte deploy without salt should be rejected");
}

// ============================================================================
// TEST 6: 4-byte deploy does NOT require salt (backwards compat)
// ============================================================================

#[wasm_bindgen_test]
fn test_4byte_deploy_no_salt_required() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ordi", "max": "21000000", "lim": "1000" }"#;
    let result = indexer.parse_operation(content, 840000);
    assert!(result.is_some(), "4-byte deploy should work without salt");
    match result.unwrap() {
        Brc20Operation::Deploy { salt, .. } => {
            assert_eq!(salt, None, "4-byte deploy should have no salt");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}

// ============================================================================
// TEST 7: Predeploy hash verification - known test vector
//
// From the spec:
// pkscript: 5120fcdc5a7bd66b4d3a8c91f1a1cf94ad7d561f3a304bf18faf5678b1ee47e783b7
// ticker: "ticker" (hex 7469636b6572)
// salt: "salt" (hex 73616c74)
// expected hash: e87852f99ef17cae507d1eda1ddd29c2271812e05d9af33abf1e6301eba83618
// ============================================================================

#[wasm_bindgen_test]
fn test_predeploy_hash_calculation() {
    use crate::brc20::compute_predeploy_hash;

    let ticker = "ticker";
    let salt_hex = "73616c74"; // "salt" in hex
    let pkscript_hex = "5120fcdc5a7bd66b4d3a8c91f1a1cf94ad7d561f3a304bf18faf5678b1ee47e783b7";

    let hash = compute_predeploy_hash(ticker, salt_hex, pkscript_hex);
    assert_eq!(
        hash,
        Some("e87852f99ef17cae507d1eda1ddd29c2271812e05d9af33abf1e6301eba83618".to_string()),
        "Hash should match the known test vector from BRC-20 proposal 001"
    );
}

// ============================================================================
// TEST 8: Invalid salt hex is rejected
// ============================================================================

#[wasm_bindgen_test]
fn test_predeploy_hash_invalid_salt_hex() {
    use crate::brc20::compute_predeploy_hash;

    let hash = compute_predeploy_hash("ticker", "not_valid_hex!", "5120abcd");
    assert!(hash.is_none(), "Invalid salt hex should return None");
}

// ============================================================================
// TEST 9: 6-byte deploy process_operation validates predeploy parent + hash
//
// Full flow: predeploy is stored, then deploy must reference it as parent
// and the hash must match sha256(sha256(ticker + salt + pkscript))
// ============================================================================

#[wasm_bindgen_test]
fn test_6byte_deploy_requires_valid_predeploy() {
    clear();
    let indexer = Brc20Indexer::new();

    // First: store a predeploy
    let predeploy_op = Brc20Operation::Predeploy {
        hash: "e87852f99ef17cae507d1eda1ddd29c2271812e05d9af33abf1e6301eba83618".to_string(),
    };
    let predeploy_id = "aaa111i0";
    let predeploy_pkscript = "5120fcdc5a7bd66b4d3a8c91f1a1cf94ad7d561f3a304bf18faf5678b1ee47e783b7";
    indexer.process_operation(&predeploy_op, predeploy_id, predeploy_pkscript).unwrap();

    // Deploy the 6-byte ticker with correct salt
    // The deploy must reference the predeploy as parent
    let deploy_op = Brc20Operation::Deploy {
        ticker: "ticker".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: Some("73616c74".to_string()),
    };
    // process_6byte_deploy needs predeploy_id, deployer pkscript
    let result = indexer.process_6byte_deploy(
        &deploy_op,
        "bbb222i0",
        predeploy_pkscript,
        Some(predeploy_id),
        912693, // 3 blocks after predeploy at 912690
        912690, // predeploy height
    );
    assert!(result.is_ok(), "6-byte deploy with valid predeploy should succeed");

    // Verify ticker was created
    let data = Brc20Tickers::new().get("ticker");
    assert!(data.is_some(), "Ticker should be deployed");
}

// ============================================================================
// TEST 10: 6-byte deploy without predeploy parent is rejected
// ============================================================================

#[wasm_bindgen_test]
fn test_6byte_deploy_without_predeploy_rejected() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy_op = Brc20Operation::Deploy {
        ticker: "ticker".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: Some("73616c74".to_string()),
    };
    let result = indexer.process_6byte_deploy(
        &deploy_op,
        "bbb222i0",
        "5120fcdc5a7bd66b4d3a8c91f1a1cf94ad7d561f3a304bf18faf5678b1ee47e783b7",
        None, // no parent
        912693,
        912690,
    );
    assert!(result.is_err(), "6-byte deploy without predeploy parent should fail");
}

// ============================================================================
// TEST 11: 6-byte deploy with wrong hash is rejected
// ============================================================================

#[wasm_bindgen_test]
fn test_6byte_deploy_wrong_hash_rejected() {
    clear();
    let indexer = Brc20Indexer::new();

    // Store predeploy with different hash
    let predeploy_op = Brc20Operation::Predeploy {
        hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
    };
    indexer.process_operation(&predeploy_op, "aaa111i0", "5120abcd").unwrap();

    // Deploy with salt that doesn't match the predeploy hash
    let deploy_op = Brc20Operation::Deploy {
        ticker: "ticker".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: Some("73616c74".to_string()),
    };
    let result = indexer.process_6byte_deploy(
        &deploy_op,
        "bbb222i0",
        "5120abcd",
        Some("aaa111i0"),
        912693,
        912690,
    );
    assert!(result.is_err(), "6-byte deploy with mismatching predeploy hash should fail");
}

// ============================================================================
// TEST 12: 3-block delay enforced — deploy too soon after predeploy
// ============================================================================

#[wasm_bindgen_test]
fn test_6byte_deploy_3_block_delay_enforced() {
    clear();
    let indexer = Brc20Indexer::new();

    let predeploy_op = Brc20Operation::Predeploy {
        hash: "e87852f99ef17cae507d1eda1ddd29c2271812e05d9af33abf1e6301eba83618".to_string(),
    };
    let pkscript = "5120fcdc5a7bd66b4d3a8c91f1a1cf94ad7d561f3a304bf18faf5678b1ee47e783b7";
    indexer.process_operation(&predeploy_op, "aaa111i0", pkscript).unwrap();

    let deploy_op = Brc20Operation::Deploy {
        ticker: "ticker".to_string(),
        max_supply: 21_000_000,
        limit_per_mint: 1000,
        decimals: 18,
        self_mint: false,
        salt: Some("73616c74".to_string()),
    };

    // Deploy only 2 blocks after predeploy — should fail
    let result = indexer.process_6byte_deploy(
        &deploy_op,
        "bbb222i0",
        pkscript,
        Some("aaa111i0"),
        912692, // only 2 blocks after 912690
        912690,
    );
    assert!(result.is_err(), "6-byte deploy within 3 blocks of predeploy should be rejected");
}

// ============================================================================
// TEST 13: 6-byte ticker can be self-mint
// ============================================================================

#[wasm_bindgen_test]
fn test_6byte_deploy_self_mint() {
    let indexer = Brc20Indexer::new();
    let content = br#"{ "p": "brc-20", "op": "deploy", "tick": "ticker", "max": "0", "lim": "0", "self_mint": "true", "salt": "73616c74" }"#;
    let result = indexer.parse_operation(content, 912690);
    assert!(result.is_some(), "6-byte self-mint deploy should parse");
    match result.unwrap() {
        Brc20Operation::Deploy { self_mint, .. } => {
            assert!(self_mint, "6-byte deploy with self_mint=true should be self-mint");
        }
        other => panic!("Expected Deploy, got {:?}", other),
    }
}
