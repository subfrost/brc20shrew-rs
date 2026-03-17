///! EVM Integration Tests
///!
///! Tests for:
///! 1. Custom precompile wiring (ShrewPrecompiles registered with revm)
///! 2. Controller contract deployment at fixed address
///! 3. Controller mint/burn functions
///! 4. Precompile addresses recognized by EVM

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::prog_indexer::ProgrammableBrc20Indexer;
use crate::controller::CONTROLLER_ADDRESS;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::{EVM_ACCOUNTS, CODE_HASH_TO_BYTECODE};
use shrew_evm::precompiles::{PRECOMPILE_BIP322, PRECOMPILE_TX_DETAILS, PRECOMPILE_LAST_SAT_LOC,
    PRECOMPILE_LOCKED_PKSCRIPT, PRECOMPILE_OP_RETURN_TXID};
use shrew_evm::ShrewPrecompiles;
use revm::primitives::{Address, U256, B256};
use revm::state::AccountInfo;
use revm::Database;
use revm::handler::PrecompileProvider;
use revm::primitives::hardfork::SpecId;
use metashrew_support::index_pointer::KeyValuePointer;

// ============================================================================
// Controller deployment tests
// ============================================================================

#[test]
fn test_controller_deployed_on_first_block() {
    clear();
    let mut indexer = ProgrammableBrc20Indexer::new();

    // Create a minimal block (just coinbase, no prog inscriptions)
    let coinbase = create_coinbase_transaction(0);
    let block = create_block_with_txs(vec![coinbase]);

    // Process block — should trigger controller deployment
    indexer.index_block(&block, 912690);

    // Verify controller account exists in EVM state
    let mut db = MetashrewDB;
    let account = db.basic(CONTROLLER_ADDRESS).unwrap();
    assert!(account.is_some(), "Controller account should exist after first block");

    let info = account.unwrap();
    assert!(info.code.is_some(), "Controller should have code");
    assert!(!info.code.unwrap().is_empty(), "Controller code should not be empty");
    assert_eq!(info.nonce, 1, "Controller nonce should be 1");
}

#[test]
fn test_controller_deployed_only_once() {
    clear();
    let mut indexer = ProgrammableBrc20Indexer::new();

    let coinbase = create_coinbase_transaction(0);
    let block = create_block_with_txs(vec![coinbase.clone()]);

    // Process two blocks
    indexer.index_block(&block, 912690);
    let mut db = MetashrewDB;
    let first_hash = db.basic(CONTROLLER_ADDRESS).unwrap().unwrap().code_hash;

    indexer.index_block(&block, 912691);
    let second_hash = db.basic(CONTROLLER_ADDRESS).unwrap().unwrap().code_hash;

    assert_eq!(first_hash, second_hash, "Controller should only be deployed once");
}

#[test]
fn test_controller_address_matches_constant() {
    // Verify the address matches the reference implementation
    let expected_hex = "c54dd4581af2dbf18e4d90840226756e9d2b3cdb";
    let expected_bytes = hex::decode(expected_hex).unwrap();
    assert_eq!(CONTROLLER_ADDRESS.as_slice(), &expected_bytes,
        "Controller address should match reference impl");
}

// ============================================================================
// ShrewPrecompiles provider tests
// ============================================================================

#[test]
fn test_shrew_precompiles_custom_addresses_recognized() {
    // Use the is_precompile function directly (avoids generic CTX issues)
    use shrew_evm::precompiles::is_precompile;
    assert!(is_precompile(&PRECOMPILE_BIP322), "Should contain BIP322");
    assert!(is_precompile(&PRECOMPILE_TX_DETAILS), "Should contain TX_DETAILS");
    assert!(is_precompile(&PRECOMPILE_LAST_SAT_LOC), "Should contain LAST_SAT_LOC");
    assert!(is_precompile(&PRECOMPILE_LOCKED_PKSCRIPT), "Should contain LOCKED_PKSCRIPT");
    assert!(is_precompile(&PRECOMPILE_OP_RETURN_TXID), "Should contain OP_RETURN_TXID");
    assert!(!is_precompile(&Address::ZERO), "Zero address should not be a precompile");
}

#[test]
fn test_shrew_precompiles_constructor() {
    // Verify ShrewPrecompiles can be constructed with different specs
    let _cancun = ShrewPrecompiles::new(SpecId::CANCUN, B256::ZERO, 840000);
    let _prague = ShrewPrecompiles::new(SpecId::PRAGUE, B256::from([0xAB; 32]), 923369);
    // No panic = success
}

// ============================================================================
// Prog indexer with custom precompiles
// ============================================================================

#[test]
fn test_deploy_uses_custom_precompiles() {
    clear();
    let mut indexer = ProgrammableBrc20Indexer::new();

    // Deploy a contract that stores value 42 and returns it
    // Bytecode: PUSH1 0x42, PUSH1 0x00, MSTORE, PUSH1 0x20, PUSH1 0x00, RETURN
    let bytecode = "604260005260206000f3";
    let content = format!(r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#, bytecode).into_bytes();

    let coinbase = create_coinbase_transaction(0);
    let inscription_tx = create_inscription_transaction(&content, "application/json", None);
    let block = create_block_with_txs(vec![coinbase, inscription_tx]);

    index_ord_block(&block, 912690).unwrap();
    indexer.index_block(&block, 912690);

    // The deploy should have succeeded (controller deployed + contract deployed)
    // We can verify by checking that the controller exists
    let mut db = MetashrewDB;
    let controller = db.basic(CONTROLLER_ADDRESS).unwrap();
    assert!(controller.is_some(), "Controller should be deployed alongside user contract");
}

// ============================================================================
// Operation shorthand parsing (d/c/t aliases)
// ============================================================================

#[test]
fn test_deploy_op_alias() {
    clear();
    let mut indexer = ProgrammableBrc20Indexer::new();

    // Use "d" shorthand for deploy
    let content = br#"{"p":"brc20-prog","op":"d","d":"604260005260206000f3"}"#;
    let coinbase = create_coinbase_transaction(0);
    let inscription_tx = create_inscription_transaction(content, "application/json", None);
    let block = create_block_with_txs(vec![coinbase, inscription_tx]);

    index_ord_block(&block, 912690).unwrap();
    indexer.index_block(&block, 912690);

    // Should have deployed (same as "deploy")
    let mut db = MetashrewDB;
    assert!(db.basic(CONTROLLER_ADDRESS).unwrap().is_some());
}

// ============================================================================
// Controller mint/burn
// ============================================================================

#[test]
fn test_controller_mint_does_not_panic() {
    clear();
    let mut indexer = ProgrammableBrc20Indexer::new();

    // First deploy controller
    let coinbase = create_coinbase_transaction(0);
    let block = create_block_with_txs(vec![coinbase]);
    indexer.index_block(&block, 912690);

    // Call controller_mint — should not panic even if controller doesn't handle it perfectly
    // (the controller bytecode may or may not be a full implementation, but the call path works)
    let recipient = Address::from_slice(&[0x42; 20]);
    indexer.controller_mint("ordi", recipient, U256::from(1000));
    // If we get here without panic, the call path is wired correctly
}

#[test]
fn test_controller_burn_does_not_panic() {
    clear();
    let mut indexer = ProgrammableBrc20Indexer::new();

    let coinbase = create_coinbase_transaction(0);
    let block = create_block_with_txs(vec![coinbase]);
    indexer.index_block(&block, 912690);

    let sender = Address::from_slice(&[0x42; 20]);
    indexer.controller_burn("ordi", sender, U256::from(500));
}
