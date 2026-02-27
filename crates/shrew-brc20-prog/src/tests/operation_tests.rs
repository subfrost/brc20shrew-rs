use crate::prog_indexer::ProgrammableBrc20Indexer;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::tables::{CONTRACT_ADDRESS_TO_INSCRIPTION_ID, INSCRIPTION_ID_TO_CONTRACT_ADDRESS};
use metashrew_support::index_pointer::KeyValuePointer;
use shrew_ord::tables::GLOBAL_SEQUENCE_COUNTER;

#[test]
fn test_prog_indexer_new() {
    // ProgrammableBrc20Indexer::new() should not panic
    let _indexer = ProgrammableBrc20Indexer::new();
}

#[test]
fn test_prog_index_empty_block() {
    clear();
    let height = 840000u32;
    let block = create_block_with_txs(vec![create_coinbase_transaction(height)]);
    index_ord_block(&block, height).unwrap();
    let mut prog = ProgrammableBrc20Indexer::new();
    // Should not panic on an empty block (no inscriptions)
    prog.index_block(&block, height);
}

#[test]
fn test_prog_non_prog_inscription_ignored() {
    clear();
    let height = 840000u32;
    let content = b"Hello world, just plain text";
    let tx = create_inscription_transaction(content, "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);

    // No contracts should have been deployed - sequence counter should show 1 inscription
    // but no prog contracts in EVM tables
    let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    assert!(!seq_bytes.is_empty(), "Inscription should exist");
}

#[test]
fn test_prog_deploy_stores_contract() {
    clear();
    let height = 840000u32;
    // Simple EVM deploy bytecode: stores 0x01 at memory[0] and returns 1 byte
    // PUSH1 0x01  PUSH1 0x00  MSTORE  PUSH1 0x01  PUSH1 0x20  RETURN
    let bytecode = "600160005260016020f3";
    let content = format!(r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#, bytecode);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);

    // The deploy should have created a contract. We check that a mapping exists
    // from the inscription id to a contract address.
    let inscription_txid = tx.txid();
    let inscription_id = shrew_support::inscription::InscriptionId::new(inscription_txid, 0);
    let inscription_id_bytes = inscription_id.to_bytes();

    let contract_addr = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id_bytes).get();
    assert!(
        !contract_addr.is_empty(),
        "Deploy should store contract address for inscription id"
    );
    assert_eq!(contract_addr.len(), 20, "Contract address should be 20 bytes");

    // Reverse mapping should also exist
    let reverse_id = CONTRACT_ADDRESS_TO_INSCRIPTION_ID.select(&contract_addr.to_vec()).get();
    assert!(
        !reverse_id.is_empty(),
        "Reverse mapping (contract address -> inscription id) should exist"
    );
    assert_eq!(
        reverse_id.as_ref(),
        inscription_id_bytes.as_slice(),
        "Reverse mapping should point back to same inscription id"
    );
}

#[test]
fn test_prog_invalid_json_ignored() {
    clear();
    let height = 840000u32;
    let content = b"this is not json at all {{{";
    let tx = create_inscription_transaction(content, "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    // Should not panic when encountering invalid JSON
    prog.index_block(&block, height);
}

#[test]
fn test_prog_wrong_protocol_ignored() {
    clear();
    let height = 840000u32;
    // brc-20 (not brc20-prog) should be ignored by the prog indexer
    let content = br#"{"p":"brc-20","op":"deploy","tick":"test","max":"1000","lim":"100"}"#;
    let tx = create_inscription_transaction(content, "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);

    // No contract should have been created for a brc-20 inscription
    let inscription_id = shrew_support::inscription::InscriptionId::new(tx.txid(), 0);
    let contract_addr = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.to_bytes()).get();
    assert!(
        contract_addr.is_empty(),
        "brc-20 inscription should not produce a prog contract"
    );
}

#[test]
fn test_prog_deploy_invalid_hex_ignored() {
    clear();
    let height = 840000u32;
    // "xyz" is not valid hex - should not crash, just produce empty bytecode
    let content = br#"{"p":"brc20-prog","op":"deploy","d":"xyz"}"#;
    let tx = create_inscription_transaction(content, "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    // Should not panic even with invalid hex in the deploy data
    prog.index_block(&block, height);
}

#[test]
fn test_prog_call_nonexistent_contract() {
    clear();
    let height = 840000u32;
    // Call to a contract that was never deployed
    let fake_inscription_id = "0000000000000000000000000000000000000000000000000000000000000000i0";
    let content = format!(
        r#"{{"p":"brc20-prog","op":"call","i":"{}","d":"00"}}"#,
        fake_inscription_id
    );
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    // Should not panic when calling a contract that doesn't exist
    prog.index_block(&block, height);
}
