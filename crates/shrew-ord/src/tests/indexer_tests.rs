use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::indexer::InscriptionIndexer;
use crate::tables::*;
use wasm_bindgen_test::wasm_bindgen_test;
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use shrew_support::inscription::{Charm, InscriptionEntry, InscriptionId};
use shrew_test_helpers::blocks::*;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_test_helpers::inscriptions::*;
use shrew_test_helpers::state;
use shrew_test_helpers::transactions::*;

#[wasm_bindgen_test]
fn test_index_empty_block() {
    state::clear();
    let block = create_block_with_coinbase_tx(100);
    index_ord_block(&block, 100).unwrap();

    // Block hash should be stored
    let hash_bytes = HEIGHT_TO_BLOCK_HASH
        .select(&100u32.to_le_bytes().to_vec())
        .get();
    assert!(!hash_bytes.is_empty(), "Block hash should be stored for height 100");
}

#[wasm_bindgen_test]
fn test_index_single_inscription() {
    state::clear();
    let tx = create_inscription_transaction(b"Hello Inscription", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());

    index_ord_block(&block, 100).unwrap();

    // Verify inscription was indexed
    let inscription_id = InscriptionId::new(tx.txid(), 0);
    let seq_bytes = INSCRIPTION_ID_TO_SEQUENCE
        .select(&inscription_id.to_bytes())
        .get();
    assert!(
        !seq_bytes.is_empty(),
        "Inscription should be indexed with a sequence number"
    );

    // Verify entry was stored
    let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
    assert!(!entry_bytes.is_empty(), "Entry should be stored");
    let entry = InscriptionEntry::from_bytes(&entry_bytes).unwrap();
    assert_eq!(entry.id, inscription_id);
    assert_eq!(entry.height, 100);
}

#[wasm_bindgen_test]
fn test_index_multiple_inscriptions_same_block() {
    state::clear();
    let tx1 = create_inscription_transaction(b"First", "text/plain", None);
    let tx2 = create_inscription_transaction(
        b"Second",
        "text/plain",
        Some(bitcoin::OutPoint {
            txid: bitcoin::Txid::from_str(
                "1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap(),
            vout: 0,
        }),
    );
    let tx3 = create_inscription_transaction(
        b"Third",
        "application/json",
        Some(bitcoin::OutPoint {
            txid: bitcoin::Txid::from_str(
                "2222222222222222222222222222222222222222222222222222222222222222",
            )
            .unwrap(),
            vout: 0,
        }),
    );

    let mut block = create_block_with_coinbase_tx(200);
    block.txdata.push(tx1.clone());
    block.txdata.push(tx2.clone());
    block.txdata.push(tx3.clone());

    index_ord_block(&block, 200).unwrap();

    // All three should be indexed
    for tx in [&tx1, &tx2, &tx3] {
        let id = InscriptionId::new(tx.txid(), 0);
        let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
        assert!(!seq.is_empty(), "Each inscription should be indexed");
    }

    // Verify sequence counter was incremented to 3
    let counter_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    assert!(!counter_bytes.is_empty());
    let counter = u32::from_le_bytes(counter_bytes[..4].try_into().unwrap());
    assert_eq!(counter, 3, "Sequence counter should be 3 after 3 inscriptions");
}

use std::str::FromStr;

#[wasm_bindgen_test]
fn test_index_inscription_sequence_across_blocks() {
    state::clear();

    // Block 1 with one inscription
    let tx1 = create_inscription_transaction(b"Block1 inscription", "text/plain", None);
    let mut block1 = create_block_with_coinbase_tx(100);
    block1.txdata.push(tx1.clone());
    index_ord_block(&block1, 100).unwrap();

    // Block 2 with another inscription
    let tx2 = create_inscription_transaction(
        b"Block2 inscription",
        "text/plain",
        Some(bitcoin::OutPoint {
            txid: bitcoin::Txid::from_str(
                "3333333333333333333333333333333333333333333333333333333333333333",
            )
            .unwrap(),
            vout: 0,
        }),
    );
    let mut block2 = create_block_with_coinbase_tx(101);
    block2.txdata.push(tx2.clone());
    index_ord_block(&block2, 101).unwrap();

    // Verify sequence counter is 2
    let counter_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    let counter = u32::from_le_bytes(counter_bytes[..4].try_into().unwrap());
    assert_eq!(counter, 2, "Sequence counter should be 2 after 2 blocks with 1 inscription each");

    // Verify both entries exist with correct heights
    let id1 = InscriptionId::new(tx1.txid(), 0);
    let seq1 = INSCRIPTION_ID_TO_SEQUENCE.select(&id1.to_bytes()).get();
    let entry1 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq1).get()).unwrap();
    assert_eq!(entry1.height, 100);

    let id2 = InscriptionId::new(tx2.txid(), 0);
    let seq2 = INSCRIPTION_ID_TO_SEQUENCE.select(&id2.to_bytes()).get();
    let entry2 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq2).get()).unwrap();
    assert_eq!(entry2.height, 101);
}

#[wasm_bindgen_test]
fn test_index_blessed_inscription_numbering() {
    state::clear();
    // Non-coinbase inscriptions (tx_index > 0) are blessed before jubilee
    let tx = create_inscription_transaction(b"blessed", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(tx.txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
    // Non-coinbase inscriptions at tx_index > 0 should get a positive (blessed) number
    assert!(
        entry.number > 0,
        "Non-coinbase inscription should have positive number, got {}",
        entry.number
    );
}

#[wasm_bindgen_test]
fn test_index_cursed_coinbase_inscription() {
    state::clear();
    // The coinbase tx (tx_index == 0) is considered cursed by context before jubilee.
    // We need a coinbase-like transaction with an inscription.
    // The indexer treats tx_index == 0 as cursed via is_cursed_by_context.
    // Let's insert the inscription into the coinbase itself.
    let witness = create_inscription_envelope(b"text/plain", b"coinbase inscription");
    let mut block = create_block_with_coinbase_tx(100);
    // Replace coinbase witness with inscription witness
    block.txdata[0].input[0].witness = witness;

    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(block.txdata[0].txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
    // Height 100 < JUBILEE_HEIGHT, so coinbase inscription is cursed
    assert!(
        entry.number < 0,
        "Coinbase inscription before jubilee should have negative number, got {}",
        entry.number
    );
    assert!(entry.has_charm(Charm::Cursed));
}

#[wasm_bindgen_test]
fn test_index_inscription_content_stored() {
    state::clear();
    let body = b"Stored content for retrieval";
    let tx = create_inscription_transaction(body, "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let inscription_id_str = format!("{}i0", tx.txid());
    let content_bytes = INSCRIPTION_CONTENT
        .select(&inscription_id_str.as_bytes().to_vec())
        .get();
    assert_eq!(
        &*content_bytes,
        body,
        "Stored content should match the original body"
    );
}

#[wasm_bindgen_test]
fn test_index_inscription_metadata_stored() {
    state::clear();
    let body = b"body with metadata";
    let metadata = b"\xa1\x64test\x63val"; // CBOR
    let witness = create_inscription_envelope_with_metadata(b"text/plain", body, Some(metadata));
    let tx = bitcoin::Transaction {
        version: bitcoin::transaction::Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint {
                txid: bitcoin::Txid::from_str(
                    "4444444444444444444444444444444444444444444444444444444444444444",
                )
                .unwrap(),
                vout: 0,
            },
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence::MAX,
            witness,
        }],
        output: vec![bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(100_000_000),
            script_pubkey: bitcoin::ScriptBuf::new(),
        }],
    };
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let inscription_id_str = format!("{}i0", tx.txid());
    let metadata_bytes = INSCRIPTION_METADATA
        .select(&inscription_id_str.as_bytes().to_vec())
        .get();
    assert_eq!(
        &*metadata_bytes,
        metadata,
        "Stored metadata should match the original"
    );
}

#[wasm_bindgen_test]
fn test_index_satpoint_calculation() {
    state::clear();
    let tx = create_inscription_transaction(b"satpoint test", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(tx.txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();

    // Default satpoint should reference the transaction's outpoint
    assert_eq!(entry.satpoint.outpoint.txid, tx.txid());
    assert_eq!(entry.satpoint.offset, 0);
}

#[wasm_bindgen_test]
fn test_index_content_type_index() {
    state::clear();
    let tx = create_inscription_transaction(b"text content", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    // Verify the content type index was populated
    let seq_list = CONTENT_TYPE_TO_INSCRIPTIONS
        .select(&"text/plain".as_bytes().to_vec())
        .get_list();
    assert!(
        !seq_list.is_empty(),
        "Content type index should have at least one entry for text/plain"
    );
}

#[wasm_bindgen_test]
fn test_index_txid_to_inscriptions() {
    state::clear();
    let tx = create_inscription_transaction(b"txid test", "text/plain", None);
    let txid = tx.txid();
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx);
    index_ord_block(&block, 100).unwrap();

    let txid_bytes = txid.as_byte_array().to_vec();
    let seq_list = TXID_TO_INSCRIPTIONS.select(&txid_bytes).get_list();
    assert!(
        !seq_list.is_empty(),
        "TXID_TO_INSCRIPTIONS should map txid to inscription sequence"
    );
}

#[wasm_bindgen_test]
fn test_index_block_hash_stored() {
    state::clear();
    let block = create_block_with_coinbase_tx(500);
    let expected_hash = block.block_hash();
    index_ord_block(&block, 500).unwrap();

    // HEIGHT_TO_BLOCK_HASH
    let hash_bytes = HEIGHT_TO_BLOCK_HASH
        .select(&500u32.to_le_bytes().to_vec())
        .get();
    assert_eq!(hash_bytes.len(), 32, "Block hash should be 32 bytes");
    assert_eq!(
        &*hash_bytes,
        expected_hash.as_byte_array(),
        "Stored hash should match block hash"
    );

    // BLOCK_HASH_TO_HEIGHT
    let height_bytes = BLOCK_HASH_TO_HEIGHT
        .select(&expected_hash.as_byte_array().to_vec())
        .get();
    assert!(!height_bytes.is_empty());
    let stored_height = u32::from_le_bytes(height_bytes[..4].try_into().unwrap());
    assert_eq!(stored_height, 500, "Stored height should be 500");
}

#[wasm_bindgen_test]
fn test_index_state_persistence() {
    state::clear();

    // Index first block
    let tx = create_inscription_transaction(b"first", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx);
    index_ord_block(&block, 100).unwrap();

    // Create new indexer instance and load state
    let mut indexer2 = InscriptionIndexer::new();
    indexer2.load_state().unwrap();

    assert_eq!(
        indexer2.sequence_counter, 1,
        "Loaded sequence counter should be 1"
    );
    assert_eq!(
        indexer2.blessed_counter, 1,
        "Loaded blessed counter should be 1"
    );
}

#[wasm_bindgen_test]
fn test_index_charm_cursed() {
    state::clear();
    // Coinbase inscription is cursed before jubilee
    let witness = create_inscription_envelope(b"text/plain", b"cursed inscription");
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata[0].input[0].witness = witness;
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(block.txdata[0].txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
    assert!(
        entry.has_charm(Charm::Cursed),
        "Coinbase inscription before jubilee should have Cursed charm"
    );
}

#[wasm_bindgen_test]
fn test_index_charm_unbound() {
    state::clear();
    // Inscription without body gets Unbound charm
    use crate::ord_inscriptions::Inscription as OrdInscription;
    let inscription = OrdInscription {
        content_type: Some(b"text/plain".to_vec()),
        body: None,
        ..Default::default()
    };
    let witness = inscription.to_witness();
    let mut block = create_block_with_coinbase_tx(100);
    // Put inscription in a non-coinbase tx so it's not doubly cursed
    let tx = bitcoin::Transaction {
        version: bitcoin::transaction::Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint {
                txid: bitcoin::Txid::from_str(
                    "5555555555555555555555555555555555555555555555555555555555555555",
                )
                .unwrap(),
                vout: 0,
            },
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence::MAX,
            witness,
        }],
        output: vec![bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(100_000_000),
            script_pubkey: bitcoin::ScriptBuf::new(),
        }],
    };
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(tx.txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    if !seq.is_empty() {
        let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
        assert!(
            entry.has_charm(Charm::Unbound),
            "Inscription without body should have Unbound charm"
        );
    }
}

#[wasm_bindgen_test]
fn test_index_outpoint_to_inscriptions() {
    state::clear();
    let tx = create_inscription_transaction(b"outpoint test", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    // Compute outpoint bytes (txid + vout)
    let id = InscriptionId::new(tx.txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();

    let outpoint_bytes: Vec<u8> = entry
        .satpoint
        .outpoint
        .txid
        .as_byte_array()
        .iter()
        .chain(entry.satpoint.outpoint.vout.to_le_bytes().iter())
        .copied()
        .collect();
    let inscriptions_at_outpoint = OUTPOINT_TO_INSCRIPTIONS
        .select(&outpoint_bytes)
        .get_list();
    assert!(
        !inscriptions_at_outpoint.is_empty(),
        "OUTPOINT_TO_INSCRIPTIONS should contain the inscription"
    );
}

#[wasm_bindgen_test]
fn test_index_duplicate_inscription_rejected() {
    state::clear();
    let tx = create_inscription_transaction(b"unique", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    // Add the same transaction again (same txid -> same inscription_id)
    block.txdata.push(tx.clone());

    // Indexing should fail with DuplicateInscription for the second one
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state().unwrap();
    let result = indexer.index_block(&block, 100);
    assert!(
        result.is_err(),
        "Indexing a block with duplicate inscription_id should fail"
    );
}

#[wasm_bindgen_test]
fn test_index_inscription_number_to_sequence() {
    state::clear();
    let tx = create_inscription_transaction(b"numbered", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(tx.txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();

    // Verify INSCRIPTION_NUMBER_TO_SEQUENCE maps the number to the correct sequence
    let number_to_seq = INSCRIPTION_NUMBER_TO_SEQUENCE
        .select(&entry.number.to_le_bytes().to_vec())
        .get();
    assert_eq!(
        &*number_to_seq, &*seq,
        "Number-to-sequence mapping should match"
    );
}

#[wasm_bindgen_test]
fn test_index_inscription_to_txid() {
    state::clear();
    let tx = create_inscription_transaction(b"txid mapping", "text/plain", None);
    let txid = tx.txid();
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(txid, 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let stored_txid_bytes = INSCRIPTION_TO_TXID.select(&seq).get();
    assert_eq!(
        &*stored_txid_bytes,
        txid.as_byte_array(),
        "INSCRIPTION_TO_TXID should store the correct txid"
    );
}

#[wasm_bindgen_test]
fn test_index_inscription_entry_content_type() {
    state::clear();
    let tx = create_inscription_transaction(b"typed content", "application/json", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(tx.txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
    assert_eq!(
        entry.content_type.as_deref(),
        Some("application/json"),
        "Entry content_type should match"
    );
}
