///! View Call Tests
///!
///! Tests that view::call() correctly executes read-only EVM calls
///! against contracts deployed via the indexer.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::prog_indexer::ProgrammableBrc20Indexer;
use crate::view;
use crate::proto::CallRequest;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::{INSCRIPTION_ID_TO_CONTRACT_ADDRESS, EVM_STORAGE};
use revm::primitives::{Address, U256};
use revm::Database;
use metashrew_support::index_pointer::KeyValuePointer;

/// Helper: wrap runtime bytecode in a simple constructor.
/// Constructor: PUSH1 <len>, DUP1, PUSH1 <offset>, PUSH1 0, CODECOPY, PUSH1 0, RETURN
/// Constructor is always 11 bytes, so runtime starts at offset 11 (0x0b).
fn wrap_runtime(runtime_hex: &str) -> String {
    let runtime_bytes = runtime_hex.len() / 2;
    assert!(runtime_bytes <= 255, "Runtime must be <= 255 bytes");
    format!("60{:02x}80600b6000396000f3{}", runtime_bytes, runtime_hex)
}

/// Helper: deploy a contract via brc20-prog inscription and return its address.
fn deploy_contract(bytecode_hex: &str, height: u32) -> Option<Address> {
    let content = format!(
        r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#,
        bytecode_hex
    );
    let coinbase = create_coinbase_transaction(height);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![coinbase, tx.clone()]);
    index_ord_block(&block, height).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);

    let inscription_id = shrew_support::inscription::InscriptionId::new(tx.txid(), 0);
    let addr_bytes = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.to_bytes()).get();
    if addr_bytes.is_empty() {
        None
    } else {
        Some(Address::from_slice(&addr_bytes))
    }
}

/// Helper: make a view call to a contract.
fn view_call(to: &Address, calldata: &[u8]) -> crate::proto::CallResponse {
    let request = CallRequest {
        to: to.as_slice().to_vec(),
        data: calldata.to_vec(),
        from: None,
    };
    view::call(&request).expect("view::call should not return Err")
}

// ============================================================================
// Basic view call tests
// ============================================================================

#[test]
fn test_view_call_simple_return() {
    clear();
    // Runtime: PUSH1 0x2a, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    let runtime = "602a60005260206000f3"; // 10 bytes
    let bytecode = wrap_runtime(runtime);

    let addr = deploy_contract(&bytecode, 912690);
    assert!(addr.is_some(), "Contract should deploy");
    let addr = addr.unwrap();

    let mut db = MetashrewDB;
    let account = db.basic(addr).unwrap();
    assert!(account.is_some(), "Account should exist");

    let response = view_call(&addr, &[0u8; 4]);
    assert!(response.success, "Should succeed: {}", response.error);
    assert_eq!(response.result.len(), 32);
    assert_eq!(response.result[31], 0x2a, "Should return 42");
}

#[test]
fn test_view_call_with_exp_opcode() {
    clear();
    // EXP: µ_s[0]^µ_s[1] → a^b where a=top, b=second
    // For 2^10: PUSH1 10, PUSH1 2, EXP → top=2, second=10 → 2^10 = 1024
    // Runtime: PUSH1 10, PUSH1 2, EXP, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    let runtime = "600a60020a60005260206000f3"; // 13 bytes
    let bytecode = wrap_runtime(runtime);

    let addr = deploy_contract(&bytecode, 912690);
    assert!(addr.is_some(), "EXP contract should deploy");
    let addr = addr.unwrap();

    let response = view_call(&addr, &[0u8; 4]);
    assert!(response.success, "EXP should succeed: {}", response.error);
    assert_eq!(response.result.len(), 32);
    // 2^10 = 1024 = 0x0400
    assert_eq!(response.result[30], 0x04);
    assert_eq!(response.result[31], 0x00);
}

#[test]
fn test_view_call_with_mulmod_and_exp() {
    clear();
    // Test the pattern from EllipticCurve.expMod:
    //   mulmod(exp(7,1), exp(7,0), 97) = mulmod(7, 1, 97) = 7
    //
    // EXP: top^second. For 7^0: PUSH 0, PUSH 7, EXP → 7^0 = 1
    //                  For 7^1: PUSH 1, PUSH 7, EXP → 7^1 = 7
    // MULMOD: (top * second) % third
    //   Need stack top→bottom: [7, 1, 97] → (7*1)%97 = 7
    //
    // Runtime:
    //   PUSH1 97, PUSH1 0, PUSH1 7, EXP,  ; stack: [97, 7^0=1]
    //                                       ; wait, EXP pops top=7, second=0 → 7^0=1
    //   PUSH1 1, PUSH1 7, EXP,            ; EXP: top=7, second=1 → 7^1=7
    //   MULMOD                              ; (7*1)%97 = 7
    //   PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    let runtime = "606160006007_0a60016007_0a_09_60005260206000f3"
        .replace("_", "");
    // 60 61=2, 60 00=2, 60 07=2, 0a=1, 60 01=2, 60 07=2, 0a=1, 09=1, 60 00=2, 52=1, 60 20=2, 60 00=2, f3=1
    // = 21 bytes
    let bytecode = wrap_runtime(&runtime);

    let addr = deploy_contract(&bytecode, 912690);
    assert!(addr.is_some(), "MULMOD+EXP should deploy");
    let addr = addr.unwrap();

    let response = view_call(&addr, &[0u8; 4]);
    assert!(response.success, "MULMOD+EXP should succeed: {}", response.error);
    assert_eq!(response.result.len(), 32);
    assert_eq!(response.result[31], 7, "mulmod(7, 1, 97) should be 7");
}

#[test]
fn test_view_call_sha256_precompile() {
    clear();
    // STATICCALL to sha256 precompile (0x02) with "test" as input
    //
    // STATICCALL takes 6 args: gas, addr, argsOff, argsSize, retOff, retSize
    // (NO value parameter unlike CALL)
    //
    // Runtime:
    //   PUSH4 "test"            63 74657374   ; store "test" in memory
    //   PUSH1 0, MSTORE         6000 52       ; mem[28..32] = "test"
    //   PUSH1 32                6020          ; retSize
    //   PUSH1 32                6020          ; retOffset
    //   PUSH1 4                 6004          ; argsSize
    //   PUSH1 28                601c          ; argsOffset (32-4=28)
    //   PUSH1 2                 6002          ; addr (sha256)
    //   PUSH2 0xFFFF            61ffff        ; gas
    //   STATICCALL              fa
    //   POP                     50
    //   PUSH1 32, PUSH1 32, RETURN  6020 6020 f3
    //
    // 5+2+1+2+2+2+2+2+3+1+1+2+2+1 = 28 bytes
    let runtime = "6374657374600052602060206004601c600261fffffa5060206020f3";
    let bytecode = wrap_runtime(runtime);

    let addr = deploy_contract(&bytecode, 912690);
    assert!(addr.is_some(), "SHA256 contract should deploy");
    let addr = addr.unwrap();

    let response = view_call(&addr, &[0u8; 4]);
    assert!(response.success, "SHA256 should succeed: {}", response.error);
    assert_eq!(response.result.len(), 32, "Should return 32 bytes");
    // sha256("test") = 0x9f86d081...
    assert_eq!(response.result[0], 0x9f, "SHA256 first byte mismatch");
}

#[test]
fn test_view_call_nonexistent_contract() {
    clear();
    let fake_addr = Address::from_slice(&[0xDE; 20]);
    let response = view_call(&fake_addr, &[0u8; 4]);
    assert!(response.success, "Nonexistent should succeed: {}", response.error);
}

#[test]
fn test_view_call_deploy_then_read_storage() {
    clear();
    // Constructor stores 0x42 at slot 0, then deploys runtime that reads slot 0.
    //
    // Constructor prefix (5 bytes): PUSH1 0x42, PUSH1 0, SSTORE
    //   60 42 60 00 55
    //
    // Runtime (11 bytes = 0x0b): PUSH1 0, SLOAD, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    //   60 00 54 60 00 52 60 20 60 00 f3
    //
    // Constructor suffix (11 bytes): PUSH1 0x0b, DUP1, PUSH1 0x10, PUSH1 0, CODECOPY, PUSH1 0, RETURN
    //   60 0b 80 60 10 60 00 39 60 00 f3
    //
    // Full: [prefix][suffix][runtime]
    let bytecode = "604260005560_0b80601060003960_00f3_6000546000526020_6000f3"
        .replace("_", "");

    let addr = deploy_contract(&bytecode, 912690);
    assert!(addr.is_some(), "Storage contract should deploy");
    let addr = addr.unwrap();

    // Verify storage was committed by reading raw table
    let mut key = addr.to_vec();
    key.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    let stored = EVM_STORAGE.select(&key).get();
    assert!(!stored.is_empty(), "Storage slot 0 should be committed by indexer");
    let stored_value = U256::from_be_slice(&stored);
    assert_eq!(stored_value, U256::from(0x42), "Stored value should be 0x42");

    // Also verify the account exists via MetashrewDB
    let mut db = MetashrewDB;
    let storage_val = db.storage(addr, U256::ZERO).unwrap();
    assert_eq!(storage_val, U256::from(0x42), "MetashrewDB.storage() should read 0x42");

    // Now verify the account has code
    let account = db.basic(addr).unwrap();
    assert!(account.is_some(), "Account should exist");
    let info = account.unwrap();
    assert!(info.code.is_some(), "Account should have code");
    let code = info.code.as_ref().unwrap();
    assert!(!code.is_empty(), "Code should not be empty");
    // Runtime should be: 60005460005260206000f3 (11 bytes)
    assert_eq!(code.len(), 11, "Runtime code should be 11 bytes");

    // View call to read the value
    let response = view_call(&addr, &[0u8; 4]);

    // Check if it reported an error
    if !response.error.is_empty() {
        panic!("View call error: '{}', success={}, result_len={}",
            response.error, response.success, response.result.len());
    }
    assert!(response.success, "Should succeed");
    assert_eq!(response.result.len(), 32, "Should return 32 bytes, got {}", response.result.len());
    assert_eq!(response.result[31], 0x42, "Should read stored value 0x42");
}

#[test]
fn test_view_call_sload_from_zero() {
    clear();
    // Deploy a contract that reads storage slot 0 (which is empty/zero).
    // This tests SLOAD without any prior SSTORE.
    // Runtime: PUSH1 0, SLOAD, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    let runtime = "6000546000526020_6000f3".replace("_", "");
    let bytecode = wrap_runtime(&runtime);

    let addr = deploy_contract(&bytecode, 912690);
    assert!(addr.is_some(), "SLOAD contract should deploy");
    let addr = addr.unwrap();

    let response = view_call(&addr, &[0u8; 4]);
    if !response.error.is_empty() {
        panic!("SLOAD error: '{}', success={}", response.error, response.success);
    }
    assert!(response.success, "SLOAD should succeed");
    assert_eq!(response.result.len(), 32, "SLOAD should return 32 bytes, got {}", response.result.len());
    // Slot 0 was never written, so should be 0
    assert_eq!(response.result[31], 0, "Uninitialized slot should be 0");
}
