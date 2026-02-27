use crate::tables::{BITMAP_NUMBER_TO_ID, BITMAP_ID_TO_NUMBER, BITMAP_HEIGHT_TO_ENTRIES};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::indexing::{index_ord_block, index_bitmap_block};
use shrew_test_helpers::bitmap::{create_bitmap_inscription_block, create_invalid_bitmap_block};
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use shrew_support::inscription::InscriptionId;
use metashrew_support::index_pointer::KeyValuePointer;
#[allow(unused_imports)]
use bitcoin_hashes::Hash;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Valid bitmap tests
// ---------------------------------------------------------------------------

#[test]
fn test_valid_bitmap_inscription() {
    clear();
    let height = 100u32;
    let (block, tx) = create_bitmap_inscription_block(100, height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&100u64.to_le_bytes().to_vec()).get();
    assert!(!data.is_empty(), "Bitmap 100 should be registered at height 100");

    let stored_id = InscriptionId::from_bytes(&data).expect("Valid inscription id");
    assert_eq!(stored_id.txid, tx.txid(), "Stored inscription should match the tx");
}

#[test]
fn test_bitmap_at_exact_height() {
    clear();
    let height = 500u32;
    let (block, _tx) = create_bitmap_inscription_block(500, height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&500u64.to_le_bytes().to_vec()).get();
    assert!(!data.is_empty(), "Bitmap 500 should be registered at exact height 500");
}

#[test]
fn test_bitmap_future_height_rejected() {
    clear();
    let height = 500u32;
    let (block, _tx) = create_bitmap_inscription_block(1000, height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&1000u64.to_le_bytes().to_vec()).get();
    assert!(data.is_empty(), "Bitmap 1000 should be rejected at height 500 (future height)");
}

#[test]
fn test_bitmap_zero() {
    clear();
    let height = 0u32;
    let (block, _tx) = create_bitmap_inscription_block(0, height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&0u64.to_le_bytes().to_vec()).get();
    assert!(!data.is_empty(), "Bitmap 0 should be valid");
}

#[test]
fn test_bitmap_leading_zeros_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_invalid_bitmap_block("007.bitmap", height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&7u64.to_le_bytes().to_vec()).get();
    assert!(data.is_empty(), "Bitmap '007.bitmap' should be rejected due to leading zeros");
}

#[test]
fn test_bitmap_non_numeric_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_invalid_bitmap_block("abc.bitmap", height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    // abc is non-numeric, so nothing should be registered
    // Check that no bitmap entries were stored at this height
    let entries = BITMAP_HEIGHT_TO_ENTRIES.select(&height.to_le_bytes().to_vec()).get();
    assert!(entries.is_empty(), "'abc.bitmap' should be rejected as non-numeric");
}

#[test]
fn test_bitmap_missing_suffix_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_invalid_bitmap_block("100", height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&100u64.to_le_bytes().to_vec()).get();
    assert!(data.is_empty(), "'100' without .bitmap suffix should be rejected");
}

#[test]
fn test_bitmap_wrong_suffix_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_invalid_bitmap_block("100.ord", height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&100u64.to_le_bytes().to_vec()).get();
    assert!(data.is_empty(), "'100.ord' should be rejected (wrong suffix)");
}

#[test]
fn test_bitmap_first_wins() {
    clear();
    let height = 100u32;

    // First inscription for bitmap 50
    let (block1, _tx1) = create_bitmap_inscription_block(50, height);
    index_ord_block(&block1, height).unwrap();
    index_bitmap_block(&block1, height);

    let data1 = BITMAP_NUMBER_TO_ID.select(&50u64.to_le_bytes().to_vec()).get();
    assert!(!data1.is_empty(), "First bitmap 50 should be registered");
    let first_id = InscriptionId::from_bytes(&data1).unwrap();

    // Second inscription for same bitmap 50 at a later height (different outpoint for unique txid)
    let height2 = 101u32;
    let outpoint2 = bitcoin::OutPoint {
        txid: bitcoin::Txid::from_str("3333333333333333333333333333333333333333333333333333333333333333").unwrap(),
        vout: 0,
    };
    let tx2 = create_inscription_transaction(b"50.bitmap", "text/plain", Some(outpoint2));
    let block2 = create_block_with_txs(vec![create_coinbase_transaction(height2), tx2]);
    index_ord_block(&block2, height2).unwrap();
    index_bitmap_block(&block2, height2);

    let data2 = BITMAP_NUMBER_TO_ID.select(&50u64.to_le_bytes().to_vec()).get();
    let stored_id = InscriptionId::from_bytes(&data2).unwrap();
    assert_eq!(stored_id, first_id, "First inscription should win for bitmap 50");
}

#[test]
fn test_bitmap_different_numbers() {
    clear();
    let height = 200u32;

    // Create a block with two bitmap inscription txs using different outpoints
    let outpoint1 = bitcoin::OutPoint {
        txid: bitcoin::Txid::from_str("1111111111111111111111111111111111111111111111111111111111111111").unwrap(),
        vout: 0,
    };
    let outpoint2 = bitcoin::OutPoint {
        txid: bitcoin::Txid::from_str("2222222222222222222222222222222222222222222222222222222222222222").unwrap(),
        vout: 0,
    };
    let tx1 = create_inscription_transaction(b"100.bitmap", "text/plain", Some(outpoint1));
    let tx2 = create_inscription_transaction(b"150.bitmap", "text/plain", Some(outpoint2));
    let block = create_block_with_txs(vec![
        create_coinbase_transaction(height),
        tx1.clone(),
        tx2.clone(),
    ]);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data100 = BITMAP_NUMBER_TO_ID.select(&100u64.to_le_bytes().to_vec()).get();
    let data150 = BITMAP_NUMBER_TO_ID.select(&150u64.to_le_bytes().to_vec()).get();
    assert!(!data100.is_empty(), "Bitmap 100 should be registered");
    assert!(!data150.is_empty(), "Bitmap 150 should be registered");
}

#[test]
fn test_bitmap_decimal_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_invalid_bitmap_block("1.5.bitmap", height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    // "1.5.bitmap" is not a valid integer prefix
    let entries = BITMAP_HEIGHT_TO_ENTRIES.select(&height.to_le_bytes().to_vec()).get();
    assert!(entries.is_empty(), "'1.5.bitmap' should be rejected (decimal number)");
}

#[test]
fn test_bitmap_empty_content_rejected() {
    clear();
    let height = 100u32;
    let (block, _tx) = create_invalid_bitmap_block("", height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let entries = BITMAP_HEIGHT_TO_ENTRIES.select(&height.to_le_bytes().to_vec()).get();
    assert!(entries.is_empty(), "Empty content should be rejected");
}

#[test]
fn test_bitmap_cursed_inscription_rejected() {
    clear();
    let height = 100u32;

    // A coinbase inscription is cursed (negative number) so it should be skipped.
    // We put the inscription in the coinbase transaction itself.
    let content = b"50.bitmap";
    let witness = shrew_test_helpers::inscriptions::create_inscription_envelope(
        b"text/plain",
        content,
    );
    let mut coinbase = create_coinbase_transaction(height);
    coinbase.input[0].witness = witness;
    let block = create_block_with_txs(vec![coinbase]);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    let data = BITMAP_NUMBER_TO_ID.select(&50u64.to_le_bytes().to_vec()).get();
    assert!(data.is_empty(), "Cursed (coinbase) inscription should not register a bitmap");
}

#[test]
fn test_bitmap_lookup_by_number() {
    clear();
    let height = 300u32;
    let (block, _tx) = create_bitmap_inscription_block(200, height);
    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    // Forward lookup: number -> id
    let id_data = BITMAP_NUMBER_TO_ID.select(&200u64.to_le_bytes().to_vec()).get();
    assert!(!id_data.is_empty(), "Forward lookup should find bitmap 200");

    let inscription_id = InscriptionId::from_bytes(&id_data).unwrap();

    // Reverse lookup: id -> number
    let number_data = BITMAP_ID_TO_NUMBER.select(&inscription_id.to_bytes()).get();
    assert!(!number_data.is_empty(), "Reverse lookup should find the number");
    let number = u64::from_le_bytes(number_data[..8].try_into().unwrap());
    assert_eq!(number, 200, "Reverse lookup should return 200");
}
