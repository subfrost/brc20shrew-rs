///! OPI Event Hash v3 Tests
///!
///! These tests verify the BRC-20 event hash calculation matches the OPI
///! reference implementation (event_hash_version=3).
///!
///! Hash format per OPI pseudocode:
///! - Events are formatted as semicolon-delimited strings
///! - Events within a block are joined by '|' (EVENT_SEPARATOR)
///! - block_hash = sha256_hex(block_str)
///! - cumulative_hash = sha256_hex(last_cumulative_hash + block_hash)

use crate::event_hash::{
    Brc20Event, EventHasher, format_amount,
};
use sha2::{Sha256, Digest};
use wasm_bindgen_test::wasm_bindgen_test;

/// Helper: compute sha256 hex of a string
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ============================================================================
// TEST 1: Event string formatting — deploy-inscribe
// ============================================================================

#[wasm_bindgen_test]
fn test_format_deploy_inscribe_event() {
    let event = Brc20Event::DeployInscribe {
        inscription_id: "abc123i0".to_string(),
        deployer_pkscript: "0014abcdef".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        max_supply: "21000000".to_string(),
        decimals: 18,
        limit_per_mint: "1000".to_string(),
        is_self_mint: false,
    };

    let formatted = event.to_event_string();
    assert_eq!(
        formatted,
        "deploy-inscribe;abc123i0;0014abcdef;ordi;ordi;21000000;18;1000;false",
        "Deploy event format should match OPI v3 spec"
    );
}

// ============================================================================
// TEST 2: Event string formatting — mint-inscribe
// ============================================================================

#[wasm_bindgen_test]
fn test_format_mint_inscribe_event() {
    let event = Brc20Event::MintInscribe {
        inscription_id: "def456i0".to_string(),
        minter_pkscript: "0014aaaaaa".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ORDI".to_string(),
        amount: "500".to_string(),
        parent_id: None,
    };

    let formatted = event.to_event_string();
    assert_eq!(
        formatted,
        "mint-inscribe;def456i0;0014aaaaaa;ordi;ORDI;500;",
        "Mint event with null parent should have empty string for parent_id"
    );
}

#[wasm_bindgen_test]
fn test_format_mint_inscribe_event_with_parent() {
    let event = Brc20Event::MintInscribe {
        inscription_id: "def456i0".to_string(),
        minter_pkscript: "0014aaaaaa".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ORDI".to_string(),
        amount: "500".to_string(),
        parent_id: Some("abc123i0".to_string()),
    };

    let formatted = event.to_event_string();
    assert_eq!(
        formatted,
        "mint-inscribe;def456i0;0014aaaaaa;ordi;ORDI;500;abc123i0",
        "Mint event with parent should include parent_id"
    );
}

// ============================================================================
// TEST 3: Event string formatting — transfer-inscribe
// ============================================================================

#[wasm_bindgen_test]
fn test_format_transfer_inscribe_event() {
    let event = Brc20Event::TransferInscribe {
        inscription_id: "ghi789i0".to_string(),
        source_pkscript: "0014bbbbbb".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        amount: "100".to_string(),
    };

    let formatted = event.to_event_string();
    assert_eq!(
        formatted,
        "transfer-inscribe;ghi789i0;0014bbbbbb;ordi;ordi;100"
    );
}

// ============================================================================
// TEST 4: Event string formatting — transfer-transfer
// ============================================================================

#[wasm_bindgen_test]
fn test_format_transfer_transfer_event() {
    let event = Brc20Event::TransferTransfer {
        inscription_id: "jkl012i0".to_string(),
        source_pkscript: "0014cccccc".to_string(),
        sent_pkscript: "0014dddddd".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        amount: "50".to_string(),
    };

    let formatted = event.to_event_string();
    assert_eq!(
        formatted,
        "transfer-transfer;jkl012i0;0014cccccc;0014dddddd;ordi;ordi;50"
    );
}

#[wasm_bindgen_test]
fn test_format_transfer_transfer_sent_as_fee() {
    // If sent as fee, sent_pkscript should be empty
    let event = Brc20Event::TransferTransfer {
        inscription_id: "jkl012i0".to_string(),
        source_pkscript: "0014cccccc".to_string(),
        sent_pkscript: "".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        amount: "50".to_string(),
    };

    let formatted = event.to_event_string();
    assert_eq!(
        formatted,
        "transfer-transfer;jkl012i0;0014cccccc;;ordi;ordi;50",
        "Fee transfer should have empty sent_pkscript"
    );
}

// ============================================================================
// TEST 5: Block hash calculation — single event
// ============================================================================

#[wasm_bindgen_test]
fn test_block_hash_single_event() {
    let mut hasher = EventHasher::new();

    let event = Brc20Event::DeployInscribe {
        inscription_id: "abc123i0".to_string(),
        deployer_pkscript: "0014abcdef".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        max_supply: "21000000".to_string(),
        decimals: 18,
        limit_per_mint: "1000".to_string(),
        is_self_mint: false,
    };

    hasher.add_event(&event);
    let block_hash = hasher.compute_block_hash();

    // Manually compute expected hash
    let event_str = "deploy-inscribe;abc123i0;0014abcdef;ordi;ordi;21000000;18;1000;false";
    let expected = sha256_hex(event_str);

    assert_eq!(block_hash, expected, "Block hash should match sha256 of event string");
}

// ============================================================================
// TEST 6: Block hash calculation — multiple events joined by pipe
// ============================================================================

#[wasm_bindgen_test]
fn test_block_hash_multiple_events() {
    let mut hasher = EventHasher::new();

    let event1 = Brc20Event::DeployInscribe {
        inscription_id: "abc123i0".to_string(),
        deployer_pkscript: "0014abcdef".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        max_supply: "21000000".to_string(),
        decimals: 18,
        limit_per_mint: "1000".to_string(),
        is_self_mint: false,
    };
    let event2 = Brc20Event::MintInscribe {
        inscription_id: "def456i0".to_string(),
        minter_pkscript: "0014aaaaaa".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        amount: "500".to_string(),
        parent_id: None,
    };

    hasher.add_event(&event1);
    hasher.add_event(&event2);
    let block_hash = hasher.compute_block_hash();

    // Expected: events joined by pipe separator
    let expected_str = "deploy-inscribe;abc123i0;0014abcdef;ordi;ordi;21000000;18;1000;false|mint-inscribe;def456i0;0014aaaaaa;ordi;ordi;500;";
    let expected = sha256_hex(expected_str);

    assert_eq!(block_hash, expected, "Block hash should be sha256 of pipe-joined events");
}

// ============================================================================
// TEST 7: Cumulative hash calculation — chaining across blocks
// ============================================================================

#[wasm_bindgen_test]
fn test_cumulative_hash_chaining() {
    // Block 1
    let mut hasher1 = EventHasher::new();
    let event1 = Brc20Event::DeployInscribe {
        inscription_id: "abc123i0".to_string(),
        deployer_pkscript: "0014abcdef".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        max_supply: "21000000".to_string(),
        decimals: 18,
        limit_per_mint: "1000".to_string(),
        is_self_mint: false,
    };
    hasher1.add_event(&event1);
    let block_hash_1 = hasher1.compute_block_hash();

    // For first block, cumulative = sha256("" + block_hash)
    let cumulative_1 = sha256_hex(&format!("{}", block_hash_1));

    // Block 2
    let mut hasher2 = EventHasher::new();
    let event2 = Brc20Event::MintInscribe {
        inscription_id: "def456i0".to_string(),
        minter_pkscript: "0014aaaaaa".to_string(),
        ticker_lowercase: "ordi".to_string(),
        ticker_original: "ordi".to_string(),
        amount: "500".to_string(),
        parent_id: None,
    };
    hasher2.add_event(&event2);
    let block_hash_2 = hasher2.compute_block_hash();

    // cumulative_2 = sha256(cumulative_1 + block_hash_2)
    let cumulative_2 = sha256_hex(&format!("{}{}", cumulative_1, block_hash_2));

    // Verify using EventHasher::compute_cumulative_hash
    let computed_cumulative_1 = EventHasher::compute_cumulative_hash("", &block_hash_1);
    assert_eq!(computed_cumulative_1, cumulative_1);

    let computed_cumulative_2 = EventHasher::compute_cumulative_hash(&cumulative_1, &block_hash_2);
    assert_eq!(computed_cumulative_2, cumulative_2);
}

// ============================================================================
// TEST 8: Empty block — no events, no hash
// ============================================================================

#[wasm_bindgen_test]
fn test_empty_block_hash() {
    let hasher = EventHasher::new();
    let block_hash = hasher.compute_block_hash();
    assert!(
        block_hash.is_empty(),
        "Empty block (no events) should produce empty hash, not hash of empty string"
    );
}

// ============================================================================
// TEST 9: Amount formatting respects ticker decimals
// ============================================================================

#[wasm_bindgen_test]
fn test_format_amount_with_decimals() {
    // 18 decimals: 1000 * 10^18 = "1000.000000000000000000" -> "1000"
    // Actually per OPI: max_supply, limit_per_mint, amount decimal count
    // matches the ticker's decimals (no trailing dot if decimals is 0)

    // decimals=0: "1000" (no dot)
    assert_eq!(format_amount(1000_000_000_000_000_000_000u128, 0), "1000");

    // decimals=8: "1000.00000000"
    assert_eq!(format_amount(1000_00000000_0000000000u128, 8), "1000.00000000");

    // decimals=18: "1000.500000000000000000"
    assert_eq!(
        format_amount(1000_500_000_000_000_000_000u128, 18),
        "1000.500000000000000000"
    );

    // decimals=18: "0.000000000000000001" (smallest unit)
    assert_eq!(format_amount(1u128, 18), "0.000000000000000001");
}

// ============================================================================
// TEST 10: Self-mint event formatting
// ============================================================================

#[wasm_bindgen_test]
fn test_format_deploy_self_mint_event() {
    let event = Brc20Event::DeployInscribe {
        inscription_id: "abc123i0".to_string(),
        deployer_pkscript: "0014abcdef".to_string(),
        ticker_lowercase: "smore".to_string(),
        ticker_original: "SMORE".to_string(),
        max_supply: "0".to_string(),
        decimals: 18,
        limit_per_mint: "0".to_string(),
        is_self_mint: true,
    };

    let formatted = event.to_event_string();
    assert_eq!(
        formatted,
        "deploy-inscribe;abc123i0;0014abcdef;smore;SMORE;0;18;0;true",
        "Self-mint deploy should have is_self_mint=true"
    );
}
