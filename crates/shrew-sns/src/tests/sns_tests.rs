use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::tables::{SNS_NAME_TO_ID, SNS_NAMESPACE_TO_ID, SNS_HEIGHT_TO_NAMES};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::indexing::{index_ord_block, index_sns_block};
use shrew_test_helpers::sns::{create_sns_reg_block, create_sns_ns_block};
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use shrew_support::inscription::InscriptionId;
use metashrew_support::index_pointer::KeyValuePointer;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Name registration tests
// ---------------------------------------------------------------------------

#[test]
fn test_sns_register_name() {
    clear();
    let height = 100u32;
    let (block, tx) = create_sns_reg_block("alice.btc", height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "Name 'alice.btc' should be registered");

    let stored_id = InscriptionId::from_bytes(&data).expect("Valid inscription id");
    assert_eq!(stored_id.txid, tx.txid());
}

#[test]
fn test_sns_register_lowercased() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_sns_reg_block("Alice.BTC", height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    // Should be stored in lowercase
    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "'Alice.BTC' should be stored as 'alice.btc'");

    // Uppercase lookup should fail
    let data_upper = SNS_NAME_TO_ID.select(&"Alice.BTC".as_bytes().to_vec()).get();
    assert!(data_upper.is_empty(), "Uppercase lookup should fail");
}

#[test]
fn test_sns_duplicate_name_rejected() {
    clear();
    let height = 100u32;

    // Register first
    let (block1, _tx1) = create_sns_reg_block("alice.btc", height);
    index_ord_block(&block1, height).unwrap();
    index_sns_block(&block1, height);

    let first_id_data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    let first_id = InscriptionId::from_bytes(&first_id_data).unwrap();

    // Try to register again at height 101 (different outpoint for unique txid)
    let height2 = 101u32;
    let outpoint2 = bitcoin::OutPoint {
        txid: bitcoin::Txid::from_str("3333333333333333333333333333333333333333333333333333333333333333").unwrap(),
        vout: 0,
    };
    let tx2 = create_inscription_transaction(
        r#"{"p":"sns","op":"reg","name":"alice.btc"}"#.as_bytes(),
        "application/json",
        Some(outpoint2),
    );
    let block2 = create_block_with_txs(vec![create_coinbase_transaction(height2), tx2]);
    index_ord_block(&block2, height2).unwrap();
    index_sns_block(&block2, height2);

    // First registration should still be stored
    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    let stored_id = InscriptionId::from_bytes(&data).unwrap();
    assert_eq!(stored_id, first_id, "First registration should win for 'alice.btc'");
}

// ---------------------------------------------------------------------------
// Namespace registration tests
// ---------------------------------------------------------------------------

#[test]
fn test_sns_register_namespace() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_sns_ns_block("btc", height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAMESPACE_TO_ID.select(&"btc".as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "Namespace 'btc' should be registered");
}

#[test]
fn test_sns_namespace_no_dots() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_sns_ns_block("btc.x", height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAMESPACE_TO_ID.select(&"btc.x".as_bytes().to_vec()).get();
    assert!(data.is_empty(), "Namespace 'btc.x' with dots should be rejected");
}

// ---------------------------------------------------------------------------
// Name validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_sns_name_exactly_one_dot() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_sns_reg_block("alice.btc", height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "Name with exactly one dot should be accepted");
}

#[test]
fn test_sns_name_no_dots_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_sns_reg_block("alice", height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&"alice".as_bytes().to_vec()).get();
    assert!(data.is_empty(), "Name 'alice' without dots should be rejected");
}

#[test]
fn test_sns_name_two_dots_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_sns_reg_block("alice.bob.btc", height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&"alice.bob.btc".as_bytes().to_vec()).get();
    assert!(data.is_empty(), "Name 'alice.bob.btc' with two dots should be rejected");
}

#[test]
fn test_sns_name_max_length() {
    clear();
    let height = 100u32;
    // 95 chars + ".btc" = 99 chars, well under 2048
    let long_prefix: String = "a".repeat(95);
    let name = format!("{}.btc", long_prefix);
    let (block, _tx) = create_sns_reg_block(&name, height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&name.to_lowercase().as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "100-char name should be accepted (under 2048)");
}

#[test]
fn test_sns_name_too_long_rejected() {
    clear();
    let height = 100u32;
    // Create a name over 2048 bytes
    let long_prefix: String = "a".repeat(2045);
    let name = format!("{}.btc", long_prefix);
    assert!(name.len() > 2048);
    let (block, _tx) = create_sns_reg_block(&name, height);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&name.to_lowercase().as_bytes().to_vec()).get();
    assert!(data.is_empty(), "Name > 2048 bytes should be rejected");
}

// ---------------------------------------------------------------------------
// Protocol validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_sns_wrong_protocol_rejected() {
    clear();
    let height = 100u32;
    let content = r#"{"p":"brc-20","op":"reg","name":"alice.btc"}"#;
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    assert!(data.is_empty(), "Wrong protocol 'brc-20' should be rejected");
}

#[test]
fn test_sns_invalid_op_rejected() {
    clear();
    let height = 100u32;
    let content = r#"{"p":"sns","op":"xxx","name":"alice.btc"}"#;
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    assert!(data.is_empty(), "Invalid op 'xxx' should be ignored");
}

#[test]
fn test_sns_missing_name_rejected() {
    clear();
    let height = 100u32;
    let content = r#"{"p":"sns","op":"reg"}"#;
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let names = SNS_HEIGHT_TO_NAMES.select(&height.to_le_bytes().to_vec()).get();
    assert!(names.is_empty(), "Registration without 'name' field should be rejected");
}

#[test]
fn test_sns_cursed_inscription_rejected() {
    clear();
    let height = 100u32;
    let content = format!(r#"{{"p":"sns","op":"reg","name":"alice.btc"}}"#);
    let witness = shrew_test_helpers::inscriptions::create_inscription_envelope(
        b"application/json",
        content.as_bytes(),
    );
    let mut coinbase = create_coinbase_transaction(height);
    coinbase.input[0].witness = witness;
    let block = create_block_with_txs(vec![coinbase]);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    assert!(data.is_empty(), "Cursed (coinbase) inscription should not register SNS name");
}

#[test]
fn test_sns_non_json_rejected() {
    clear();
    let height = 100u32;
    let content = "this is not json";
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    let names = SNS_HEIGHT_TO_NAMES.select(&height.to_le_bytes().to_vec()).get();
    assert!(names.is_empty(), "Non-JSON content should be rejected");
}

#[test]
fn test_sns_first_whitespace_token() {
    clear();
    let height = 100u32;
    // The name field contains whitespace; the indexer should take the first token
    let content = r#"{"p":"sns","op":"reg","name":"alice.btc other stuff"}"#;
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    // Should be registered as "alice.btc" (first whitespace-delimited token)
    let data = SNS_NAME_TO_ID.select(&"alice.btc".as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "'alice.btc other stuff' should register as 'alice.btc'");

    // The full string should NOT be registered
    let data_full = SNS_NAME_TO_ID.select(&"alice.btc other stuff".as_bytes().to_vec()).get();
    assert!(data_full.is_empty(), "Full string with whitespace should not be a key");
}
