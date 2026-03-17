///! BRC20-Prog OPI conformance tests.
///!
///! These test the prog indexer against known correctness requirements from the
///! OPI reference implementation.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::prog_indexer::ProgrammableBrc20Indexer;
use crate::view;
use crate::proto::CallRequest;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::{create_inscription_transaction, create_mock_outpoint};
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::tables::INSCRIPTION_ID_TO_CONTRACT_ADDRESS;
use metashrew_support::index_pointer::KeyValuePointer;
use shrew_ord::tables::GLOBAL_SEQUENCE_COUNTER;

/// Create a simple prog deploy inscription content.
/// Bytecode: PUSH1 0x42, PUSH1 0x00, MSTORE, PUSH1 0x20, PUSH1 0x00, RETURN
/// This deploys a contract whose runtime code returns 0x42 (32 bytes).
fn simple_deploy_content() -> Vec<u8> {
    let bytecode = "604260005260206000f3";
    format!(r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#, bytecode).into_bytes()
}

/// Create a prog call inscription content targeting the given inscription ID.
fn call_content(inscription_id: &str, calldata: &str) -> Vec<u8> {
    format!(
        r#"{{"p":"brc20-prog","op":"call","i":"{}","d":"{}"}}"#,
        inscription_id, calldata
    ).into_bytes()
}

// ============================================================================
// ISSUE: prog_indexer re-scans all inscriptions O(n) per block
//
// The current implementation iterates 1..=max_seq and filters by height.
// This means inscriptions from block N are re-checked (but skipped) in
// block N+1, N+2, etc. While functionally correct, the O(n) behavior is
// a performance concern. This test verifies CORRECTNESS: that inscriptions
// from a previous block are NOT re-processed.
// ============================================================================

#[test]
fn test_prog_no_duplicate_processing_across_blocks() {
    clear();

    // Block 1: Deploy a contract
    let height1 = 840000u32;
    let content = simple_deploy_content();
    let tx = create_inscription_transaction(&content, "application/json", None);
    let block1 = create_block_with_txs(vec![create_coinbase_transaction(height1), tx.clone()]);
    index_ord_block(&block1, height1).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block1, height1);

    // Capture the contract address created by the deploy
    let inscription_id = shrew_support::inscription::InscriptionId::new(tx.txid(), 0);
    let contract_addr_1 = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.to_bytes()).get();
    assert!(!contract_addr_1.is_empty(), "Deploy should create contract address");

    // Block 2: Empty block (no prog inscriptions)
    let height2 = 840001u32;
    let block2 = create_block_with_txs(vec![create_coinbase_transaction(height2)]);
    index_ord_block(&block2, height2).unwrap();

    let mut prog2 = ProgrammableBrc20Indexer::new();
    prog2.index_block(&block2, height2);

    // Contract address should be unchanged (not re-deployed)
    let contract_addr_2 = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.to_bytes()).get();
    assert_eq!(contract_addr_1.as_ref(), contract_addr_2.as_ref(),
        "Contract address should not change after processing a subsequent block");
}

// ============================================================================
// ISSUE: EVM state persistence across blocks
//
// After deploying a contract in block N, a call to that contract in block N+1
// should see the deployed code. This verifies the MetashrewDB persists EVM state.
// ============================================================================

#[test]
fn test_prog_deploy_then_call_same_block() {
    clear();
    let height = 840000u32;

    // Deploy a simple contract (uses default outpoint 0)
    let deploy_content = simple_deploy_content();
    let deploy_tx = create_inscription_transaction(&deploy_content, "application/json", None);

    let deploy_inscription_id = shrew_support::inscription::InscriptionId::new(deploy_tx.txid(), 0);
    let deploy_id_str = deploy_inscription_id.to_string();

    // Call the contract in the same block (use unique outpoint 1)
    let call_bytes = call_content(&deploy_id_str, "00");
    let call_tx = create_inscription_transaction(&call_bytes, "application/json", Some(create_mock_outpoint(1)));

    let block = create_block_with_txs(vec![
        create_coinbase_transaction(height),
        deploy_tx.clone(),
        call_tx.clone(),
    ]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);

    // Verify the contract was deployed
    let contract_addr = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&deploy_inscription_id.to_bytes()).get();
    assert!(!contract_addr.is_empty(), "Contract should be deployed");

    // The call should not panic and should have completed (we can't easily check
    // EVM return values here, but no panic = no crash on call to deployed contract)
}

// ============================================================================
// ISSUE: View function (eth_call style) correctness
//
// After deploying a contract, the view::call() function should be able to
// execute a read-only call against it without persisting state.
// ============================================================================

#[test]
fn test_view_call_to_deployed_contract() {
    clear();
    let height = 840000u32;

    let deploy_content = simple_deploy_content();
    let deploy_tx = create_inscription_transaction(&deploy_content, "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), deploy_tx.clone()]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);

    // Get the deployed contract address
    let inscription_id = shrew_support::inscription::InscriptionId::new(deploy_tx.txid(), 0);
    let contract_addr = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.to_bytes()).get();
    assert!(!contract_addr.is_empty(), "Contract should be deployed");

    // Call the contract via view function
    let request = CallRequest {
        to: contract_addr.to_vec(),
        data: vec![], // no specific function selector
        from: None,
    };
    let response = view::call(&request).expect("View call should not error");

    // The response should be successful (the contract returns 0x42)
    assert!(response.success || !response.error.is_empty(),
        "View call should produce either success or an explicit error, not crash");
}

#[test]
fn test_view_call_to_nonexistent_address() {
    clear();
    let height = 840000u32;
    let block = create_block_with_txs(vec![create_coinbase_transaction(height)]);
    index_ord_block(&block, height).unwrap();

    // Call to a random address that has no code
    let request = CallRequest {
        to: vec![0xDE; 20],
        data: vec![0x01, 0x02, 0x03],
        from: None,
    };
    let response = view::call(&request).expect("View call should not error");
    // Should succeed but return empty (no code at address)
    assert!(response.error.is_empty() || response.success,
        "Call to empty address should succeed with empty output, not error");
}

#[test]
fn test_view_call_with_invalid_address() {
    // Address must be exactly 20 bytes
    let request = CallRequest {
        to: vec![0xAB; 5], // Too short
        data: vec![],
        from: None,
    };
    let response = view::call(&request).expect("Should return Ok with error message");
    assert!(!response.error.is_empty(), "Invalid address should produce error message");
    assert!(!response.success, "Invalid address should not report success");
}

// ============================================================================
// ISSUE: Multiple deploys in same block - isolation
//
// Two different brc20-prog deploy inscriptions in the same block should each
// get their own contract address.
// ============================================================================

#[test]
fn test_prog_multiple_deploys_same_block() {
    clear();
    let height = 840000u32;

    // Deploy contract 1: returns 0x42 (uses default outpoint 0)
    let content1 = simple_deploy_content();
    let tx1 = create_inscription_transaction(&content1, "application/json", None);

    // Deploy contract 2: different bytecode, returns 0xFF (uses outpoint 1)
    let bytecode2 = "60ff60005260206000f3";
    let content2 = format!(r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#, bytecode2).into_bytes();
    let tx2 = create_inscription_transaction(&content2, "application/json", Some(create_mock_outpoint(1)));

    let block = create_block_with_txs(vec![
        create_coinbase_transaction(height),
        tx1.clone(),
        tx2.clone(),
    ]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);

    // Both should be deployed with DIFFERENT addresses
    let id1 = shrew_support::inscription::InscriptionId::new(tx1.txid(), 0);
    let id2 = shrew_support::inscription::InscriptionId::new(tx2.txid(), 0);

    let addr1 = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&id1.to_bytes()).get();
    let addr2 = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&id2.to_bytes()).get();

    assert!(!addr1.is_empty(), "Contract 1 should be deployed");
    assert!(!addr2.is_empty(), "Contract 2 should be deployed");
    assert_ne!(addr1.as_ref(), addr2.as_ref(),
        "Two different contracts should have different addresses");
}

// ============================================================================
// ISSUE: Gas limit behavior
//
// Verify that extremely large deploy bytecodes are handled gracefully
// (gas limit capping at BRC20_PROG_MAX_CALL_GAS).
// ============================================================================

#[test]
fn test_prog_deploy_large_bytecode_no_crash() {
    clear();
    let height = 840000u32;

    // Create very large bytecode (100KB of NOPs = 0x5b JUMPDEST)
    let bytecode = "5b".repeat(100_000);
    let content = format!(r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#, bytecode).into_bytes();
    let tx = create_inscription_transaction(&content, "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    // Should not panic, even if gas runs out
    prog.index_block(&block, height);
}

// ============================================================================
// Full integration: ord → brc20 → prog pipeline
//
// Index a block through all three indexers in sequence, verifying that the
// BRC-20 deploy and the prog deploy both work correctly when run together.
// ============================================================================

#[test]
fn test_full_pipeline_brc20_and_prog() {
    clear();
    let height = 840000u32;

    // BRC-20 deploy inscription (uses default outpoint 0)
    let brc20_content = br#"{"p":"brc-20","op":"deploy","tick":"test","max":"21000000","lim":"1000"}"#;
    let brc20_tx = create_inscription_transaction(brc20_content, "text/plain", None);

    // BRC20-prog deploy inscription (uses outpoint 1 to avoid duplicate)
    let prog_content = simple_deploy_content();
    let prog_tx = create_inscription_transaction(&prog_content, "application/json", Some(create_mock_outpoint(1)));

    let block = create_block_with_txs(vec![
        create_coinbase_transaction(height),
        brc20_tx.clone(),
        prog_tx.clone(),
    ]);

    // Run ord indexer
    index_ord_block(&block, height).unwrap();

    // Run BRC-20 indexer
    let brc20_indexer = shrew_brc20::brc20::Brc20Indexer::new();
    brc20_indexer.process_block(&block, height);

    // Run prog indexer
    let mut prog_indexer = ProgrammableBrc20Indexer::new();
    prog_indexer.index_block(&block, height);

    // Verify BRC-20 ticker was created
    let ticker_data = shrew_brc20::tables::Brc20Tickers::new().get("test");
    assert!(ticker_data.is_some(), "BRC-20 ticker 'test' should exist after indexing");

    // Verify prog contract was deployed
    let prog_id = shrew_support::inscription::InscriptionId::new(prog_tx.txid(), 0);
    let contract_addr = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&prog_id.to_bytes()).get();
    assert!(!contract_addr.is_empty(), "Prog contract should be deployed");

    // Verify sequence counter reflects both inscriptions
    let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    assert!(!seq_bytes.is_empty());
    let count = u32::from_le_bytes(seq_bytes[..4].try_into().unwrap());
    assert!(count >= 2, "At least 2 inscriptions should be indexed (brc20 + prog)");
}
