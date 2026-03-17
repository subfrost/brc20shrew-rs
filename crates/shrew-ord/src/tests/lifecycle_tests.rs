use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::proto::{
    get_inscription_request, GetBlockHashRequest, GetContentRequest, GetInscriptionRequest,
    GetInscriptionsRequest, InscriptionId as ProtoInscriptionId, PaginationRequest,
};
use crate::tables::*;
use crate::view;
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use shrew_support::inscription::{InscriptionEntry, InscriptionId};
use shrew_test_helpers::blocks::*;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_test_helpers::inscriptions::*;
use shrew_test_helpers::state;
use shrew_test_helpers::transactions::*;
use std::str::FromStr;

#[test]
fn test_full_inscription_lifecycle() {
    state::clear();

    // Step 1: Create and index an inscription
    let body = b"Full lifecycle test inscription";
    let tx = create_inscription_transaction(body, "text/plain", None);
    let txid = tx.txid();
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx);
    index_ord_block(&block, 100).unwrap();

    // Step 2: Verify via get_inscription (by ID)
    let request_by_id = GetInscriptionRequest {
        query: Some(get_inscription_request::Query::Id(ProtoInscriptionId {
            txid: txid.as_byte_array().to_vec(),
            index: 0,
        })),
        child_index: None,
    };
    let response = view::get_inscription(&request_by_id).unwrap();
    assert!(response.id.is_some());
    let resp_id = response.id.unwrap();
    assert_eq!(resp_id.txid, txid.as_byte_array().to_vec());
    assert_eq!(resp_id.index, 0);
    assert_eq!(response.number, 1); // blessed
    // Note: height is not currently populated in the view response (uses Default)
    assert_eq!(response.content_type.as_deref(), Some("text/plain"));

    // Step 3: Verify via get_inscription (by number)
    let request_by_number = GetInscriptionRequest {
        query: Some(get_inscription_request::Query::Number(1)),
        child_index: None,
    };
    let response2 = view::get_inscription(&request_by_number).unwrap();
    assert!(response2.id.is_some());
    assert_eq!(response2.id.unwrap().txid, txid.as_byte_array().to_vec());

    // Step 4: Verify content retrieval
    let content_request = GetContentRequest {
        id: Some(ProtoInscriptionId {
            txid: txid.as_byte_array().to_vec(),
            index: 0,
        }),
    };
    let content_response = view::get_content(&content_request).unwrap();
    assert_eq!(content_response.content, body);

    // Step 5: Verify block hash
    let block_hash = block.block_hash();
    let hash_request = GetBlockHashRequest {
        height: Some(100),
    };
    let hash_response = view::get_block_hash(&hash_request).unwrap();
    assert_eq!(hash_response.hash, block_hash.to_string());

    // Step 6: Verify inscriptions list
    let list_request = GetInscriptionsRequest {
        pagination: Some(PaginationRequest {
            page: 0,
            limit: 10,
        }),
        filter: None,
    };
    let list_response = view::get_inscriptions(&list_request).unwrap();
    assert_eq!(list_response.ids.len(), 1);
}

#[test]
fn test_multi_block_lifecycle() {
    state::clear();

    let mut expected_count = 0u32;
    let base_height = 200;

    for i in 0..5u32 {
        let body = format!("Inscription in block {}", i);
        let prev_outpoint = bitcoin::OutPoint {
            txid: bitcoin::Txid::from_str(&format!(
                "{:0>64x}",
                (i + 1) as u64
            ))
            .unwrap(),
            vout: 0,
        };
        let tx = create_inscription_transaction(body.as_bytes(), "text/plain", Some(prev_outpoint));
        let mut block = create_block_with_coinbase_tx(base_height + i);
        block.txdata.push(tx);
        index_ord_block(&block, base_height + i).unwrap();
        expected_count += 1;
    }

    // Verify total inscription count
    let counter_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    let total = u32::from_le_bytes(counter_bytes[..4].try_into().unwrap());
    assert_eq!(total, expected_count, "Should have {} inscriptions", expected_count);

    // Verify all block hashes stored
    for i in 0..5u32 {
        let hash_bytes = HEIGHT_TO_BLOCK_HASH
            .select(&(base_height + i).to_le_bytes().to_vec())
            .get();
        assert!(
            !hash_bytes.is_empty(),
            "Block hash for height {} should be stored",
            base_height + i
        );
    }

    // Verify all inscriptions are queryable
    let list_request = GetInscriptionsRequest {
        pagination: Some(PaginationRequest {
            page: 0,
            limit: 100,
        }),
        filter: None,
    };
    let list_response = view::get_inscriptions(&list_request).unwrap();
    assert_eq!(list_response.ids.len(), 5);
}

#[test]
fn test_inscription_transfer() {
    state::clear();

    // Index an inscription
    let tx = create_inscription_transaction(b"transfer test", "text/plain", None);
    let txid = tx.txid();
    let mut block1 = create_block_with_coinbase_tx(100);
    block1.txdata.push(tx);
    index_ord_block(&block1, 100).unwrap();

    // Verify initial outpoint
    let id = InscriptionId::new(txid, 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
    assert_eq!(entry.satpoint.outpoint.txid, txid);

    // Create a transfer transaction and index it
    let transfer_tx = create_transfer_transaction(&txid, 0);
    let mut block2 = create_block_with_coinbase_tx(101);
    block2.txdata.push(transfer_tx.clone());
    index_ord_block(&block2, 101).unwrap();

    // The inscription entry satpoint is set at creation time and is not updated
    // by a simple transfer (the indexer in its current form does not track transfers
    // beyond creation). This test verifies the transfer block indexes without error.
    let hash_bytes = HEIGHT_TO_BLOCK_HASH
        .select(&101u32.to_le_bytes().to_vec())
        .get();
    assert!(!hash_bytes.is_empty(), "Transfer block should be indexed");
}

#[test]
fn test_inscription_with_all_fields() {
    state::clear();

    let body = b"full-fields inscription";
    let metadata = b"\xa1\x63key\x63val";
    let witness = create_inscription_envelope_with_metadata(b"application/json", body, Some(metadata));
    let tx = bitcoin::Transaction {
        version: 1,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint {
                txid: bitcoin::Txid::from_str(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                )
                .unwrap(),
                vout: 0,
            },
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence::MAX,
            witness,
        }],
        output: vec![bitcoin::TxOut {
            value: 100_000_000,
            script_pubkey: bitcoin::ScriptBuf::new(),
        }],
    };
    let txid = tx.txid();
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx);
    index_ord_block(&block, 100).unwrap();

    // Verify entry has content_type set
    let id = InscriptionId::new(txid, 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
    assert_eq!(entry.content_type.as_deref(), Some("application/json"));
    assert_eq!(entry.content_length, Some(body.len() as u64));

    // Verify content stored
    let inscription_id_str = format!("{}i0", txid);
    let stored_content = INSCRIPTION_CONTENT
        .select(&inscription_id_str.as_bytes().to_vec())
        .get();
    assert_eq!(&*stored_content, body);

    // Verify metadata stored
    let stored_metadata = INSCRIPTION_METADATA
        .select(&inscription_id_str.as_bytes().to_vec())
        .get();
    assert_eq!(&*stored_metadata, metadata);
}

#[test]
fn test_large_content_inscription() {
    state::clear();

    // Create a 400KB body
    let body: Vec<u8> = (0..400_000).map(|i| (i % 256) as u8).collect();
    let tx = create_inscription_transaction(&body, "application/octet-stream", None);
    let txid = tx.txid();
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx);
    index_ord_block(&block, 100).unwrap();

    // Verify inscription exists
    let id = InscriptionId::new(txid, 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    assert!(!seq.is_empty(), "Large inscription should be indexed");

    // Verify content length in entry
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
    assert_eq!(
        entry.content_length,
        Some(400_000),
        "Content length should be 400000"
    );

    // Verify actual content stored
    let inscription_id_str = format!("{}i0", txid);
    let stored_content = INSCRIPTION_CONTENT
        .select(&inscription_id_str.as_bytes().to_vec())
        .get();
    assert_eq!(stored_content.len(), 400_000, "Stored content should be 400KB");
}

#[test]
fn test_batch_inscriptions() {
    state::clear();

    let mut block = create_block_with_coinbase_tx(100);
    let mut txids = Vec::new();

    for i in 0..20u32 {
        let body = format!("Batch inscription #{}", i);
        let prev_outpoint = bitcoin::OutPoint {
            txid: bitcoin::Txid::from_str(&format!(
                "{:0>64x}",
                (i + 100) as u64
            ))
            .unwrap(),
            vout: 0,
        };
        let tx = create_inscription_transaction(body.as_bytes(), "text/plain", Some(prev_outpoint));
        txids.push(tx.txid());
        block.txdata.push(tx);
    }

    index_ord_block(&block, 100).unwrap();

    // Verify all 20 inscriptions were indexed
    let counter_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    let total = u32::from_le_bytes(counter_bytes[..4].try_into().unwrap());
    assert_eq!(total, 20, "Should have 20 inscriptions");

    // Verify each inscription exists
    for txid in &txids {
        let id = InscriptionId::new(*txid, 0);
        let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
        assert!(!seq.is_empty(), "Inscription for txid {} should exist", txid);
    }
}

#[test]
fn test_inscription_numbering_consistency() {
    state::clear();

    let mut numbers = Vec::new();
    for i in 0..5u32 {
        let prev_outpoint = bitcoin::OutPoint {
            txid: bitcoin::Txid::from_str(&format!(
                "{:0>64x}",
                (i + 50) as u64
            ))
            .unwrap(),
            vout: 0,
        };
        let tx = create_inscription_transaction(
            format!("numbered #{}", i).as_bytes(),
            "text/plain",
            Some(prev_outpoint),
        );
        let txid = tx.txid();
        let mut block = create_block_with_coinbase_tx(100 + i);
        block.txdata.push(tx);
        index_ord_block(&block, 100 + i).unwrap();

        let id = InscriptionId::new(txid, 0);
        let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
        let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();
        numbers.push(entry.number);
    }

    // All should be blessed (positive) since they are non-coinbase
    for n in &numbers {
        assert!(*n > 0, "Blessed inscription number should be positive, got {}", n);
    }

    // Numbers should be monotonically increasing
    for i in 1..numbers.len() {
        assert!(
            numbers[i] > numbers[i - 1],
            "Inscription numbers should be monotonically increasing: {} > {} at index {}",
            numbers[i],
            numbers[i - 1],
            i
        );
    }
}

#[test]
fn test_empty_blocks_no_errors() {
    state::clear();

    // Index 5 empty blocks (coinbase only)
    for i in 0..5u32 {
        let block = create_block_with_coinbase_tx(100 + i);
        index_ord_block(&block, 100 + i).unwrap();
    }

    // Verify all block hashes were stored
    for i in 0..5u32 {
        let hash_bytes = HEIGHT_TO_BLOCK_HASH
            .select(&(100 + i).to_le_bytes().to_vec())
            .get();
        assert!(
            !hash_bytes.is_empty(),
            "Block hash for height {} should be stored",
            100 + i
        );
    }

    // No inscriptions should exist
    let counter_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    let has_inscriptions = !counter_bytes.is_empty()
        && counter_bytes.len() >= 4
        && u32::from_le_bytes(counter_bytes[..4].try_into().unwrap()) > 0;
    assert!(
        !has_inscriptions,
        "Empty blocks should not produce any inscriptions"
    );
}
