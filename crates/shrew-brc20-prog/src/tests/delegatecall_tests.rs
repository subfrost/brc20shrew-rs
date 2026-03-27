///! Delegatecall state persistence tests
///!
///! Tests that state changes made via DELEGATECALL during contract
///! construction are properly committed to MetashrewDB.
///!
///! This isolates the issue where BiS_Swap's proxy pattern fails:
///! MinimalProxy constructor delegatecalls initialize() on the impl,
///! which sets storage slots, but the state doesn't persist.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::prog_indexer::ProgrammableBrc20Indexer;
use crate::view;
use crate::proto::CallRequest;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::INSCRIPTION_ID_TO_CONTRACT_ADDRESS;
use revm::primitives::{Address, U256};
use revm::Database;
use metashrew_support::index_pointer::KeyValuePointer;

use shrew_test_helpers::transactions::create_mock_outpoint;
use std::sync::atomic::{AtomicU32, Ordering};
static OUTPOINT_COUNTER: AtomicU32 = AtomicU32::new(100);

fn deploy_contract(bytecode_hex: &str, height: u32) -> Option<Address> {
    let content = format!(
        r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#,
        bytecode_hex
    );
    let outpoint_n = OUTPOINT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let coinbase = create_coinbase_transaction(height);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", Some(create_mock_outpoint(outpoint_n)));
    let block = create_block_with_txs(vec![coinbase, tx.clone()]);
    index_ord_block(&block, height).unwrap();
    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);
    let inscription_id = shrew_support::inscription::InscriptionId::new(tx.txid(), 0);
    let addr_bytes = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.to_bytes()).get();
    if addr_bytes.is_empty() { None } else { Some(Address::from_slice(&addr_bytes)) }
}

fn view_call(to: &Address, calldata: &[u8]) -> crate::proto::CallResponse {
    let request = CallRequest { to: to.as_slice().to_vec(), data: calldata.to_vec(), from: None };
    view::call(&request).expect("view::call should not Err")
}

fn wrap_runtime(runtime_hex: &str) -> String {
    let runtime_bytes = runtime_hex.len() / 2;
    assert!(runtime_bytes <= 255);
    format!("60{:02x}80600b6000396000f3{}", runtime_bytes, runtime_hex)
}

/// Test 1: Simple contract that stores value in constructor, read via view
/// This verifies basic constructor SSTORE → view SLOAD works.
#[test]
fn test_constructor_sstore_persists() {
    clear();
    // Constructor: SSTORE(0, 0x42), then deploy runtime that reads slot 0
    // Runtime (11 bytes): PUSH1 0, SLOAD, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    let bytecode = "604260005560_0b80601060003960_00f3_6000546000526020_6000f3"
        .replace("_", "");
    let addr = deploy_contract(&bytecode, 912690).expect("deploy");
    let resp = view_call(&addr, &[0u8; 4]);
    assert!(resp.success, "view should succeed: {}", resp.error);
    assert_eq!(resp.result[31], 0x42, "should read stored value");
}

/// Test 2: Contract A deploys Contract B in constructor (nested CREATE)
/// Then reads B's address from storage.
/// This tests that nested CREATEs during deployment persist.
#[test]
fn test_nested_create_in_constructor() {
    clear();
    // We need a contract whose constructor does:
    //   1. CREATE a child contract (simple: just stores 0x99 at slot 0)
    //   2. SSTORE the child address at slot 0
    //   3. Deploy runtime that reads slot 0 (the child address)
    //
    // Child initcode: PUSH1 0x99, PUSH1 0, SSTORE, STOP
    //   6099 6000 55 00 = 6 bytes
    //
    // Parent constructor:
    //   PUSH6 <child_initcode>  65 609960005500   (7 bytes, pushes 6 bytes)
    //   PUSH1 0                 6000              (store child code at mem[0])
    //   MSTORE                  52
    //   PUSH1 6                 6006              (child code size)
    //   PUSH1 26               601a              (mem offset: 32-6=26)
    //   PUSH1 0                 6000              (value: 0 wei)
    //   CREATE                  f0                (creates child, pushes address)
    //   PUSH1 0                 6000              (slot 0)
    //   SSTORE                  55                (store child address at slot 0)
    //   <then deploy runtime>
    //
    // Runtime (11 bytes): PUSH1 0, SLOAD, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    //   6000 54 6000 52 6020 6000 f3
    //
    // Constructor = 7+2+1+2+2+2+1+2+1 = 20 bytes
    // Runtime starts at offset 20+11 = 31... let me compute carefully.
    //
    // Actually, let me build this more carefully with hex:
    // Child initcode (6 bytes): 60 99 60 00 55 00
    // Push it as bytes to memory:
    //   PUSH6 609960005500 → 65 60 99 60 00 55 00  (7 bytes)
    //   PUSH1 0  → 60 00 (2 bytes)
    //   MSTORE   → 52    (1 byte)  ← stores at mem[0..32], right-aligned: mem[26..32] = child
    //   PUSH1 6  → 60 06 (2 bytes) ← child size
    //   PUSH1 26 → 60 1a (2 bytes) ← mem offset (32-6=26)
    //   PUSH1 0  → 60 00 (2 bytes) ← value
    //   CREATE   → f0    (1 byte)  ← creates child
    //   PUSH1 0  → 60 00 (2 bytes) ← slot
    //   SSTORE   → 55    (1 byte)  ← store child addr
    //   --- deploy runtime (11 bytes) ---
    //   PUSH1 11 → 60 0b (2 bytes) ← runtime size
    //   DUP1     → 80    (1 byte)
    //   PUSH1 ?? → 60 ?? (2 bytes) ← offset = total constructor size
    //   PUSH1 0  → 60 00 (2 bytes)
    //   CODECOPY → 39    (1 byte)
    //   PUSH1 0  → 60 00 (2 bytes)
    //   RETURN   → f3    (1 byte)
    //
    // Constructor size = 7+2+1+2+2+2+1+2+1 + 2+1+2+2+1+2+1 = 20+11 = 31
    // Wait no, the runtime deploy code is part of the constructor:
    // Constructor prefix (store child addr): 20 bytes
    // Constructor suffix (deploy runtime): 11 bytes
    // Total constructor: 31 bytes
    // Runtime: 11 bytes
    // Runtime starts at offset 31
    //
    // Constructor suffix: PUSH1 0x0b, DUP1, PUSH1 0x1f(=31), PUSH1 0, CODECOPY, PUSH1 0, RETURN
    //   60 0b 80 60 1f 60 00 39 60 00 f3

    let bytecode = "65609960005500600052600660\
1a6000f060005560\
0b80601f60003960\
00f360005460005260206000f3";

    let addr = deploy_contract(&bytecode, 912690).expect("deploy");

    // Read slot 0 — should be the child contract address (non-zero)
    let resp = view_call(&addr, &[0u8; 4]);
    assert!(resp.success, "view should succeed: {}", resp.error);

    // The child address should be in the last 20 bytes of the 32-byte return
    let is_zero = resp.result[12..32].iter().all(|&b| b == 0);
    assert!(!is_zero, "Child address should be non-zero (nested CREATE worked)");
}

/// Test 3: DELEGATECALL that writes storage
/// Deploy an implementation that has a function to write slot 0.
/// Deploy a proxy that DELEGATECALLs that function in its constructor.
/// Read slot 0 via view — should see the value written by delegatecall.
#[test]
fn test_delegatecall_in_constructor_persists() {
    clear();

    // Step 1: Deploy "implementation" — a contract whose fallback writes 0xBEEF to slot 0
    // Runtime: PUSH2 0xBEEF, PUSH1 0, SSTORE, STOP
    //   61 beef 60 00 55 00 = 6 bytes
    let impl_runtime = "61beef60005500";
    let impl_bytecode = wrap_runtime(impl_runtime);
    let impl_addr = deploy_contract(&impl_bytecode, 912690).expect("impl deploy");

    // Step 2: Deploy "proxy" — constructor DELEGATECALLs impl (which writes 0xBEEF to slot 0)
    //   then deploys runtime that reads slot 0
    //
    // Constructor:
    //   PUSH1 0     6000   ← retSize (0, we don't care about return data)
    //   PUSH1 0     6000   ← retOffset
    //   PUSH1 0     6000   ← argsSize (0, no calldata needed — fallback triggers)
    //   PUSH1 0     6000   ← argsOffset
    //   PUSH20 <impl_addr> ← 20-byte address
    //   PUSH3 0xFFFFFF     ← gas
    //   DELEGATECALL f4
    //   POP          50    ← pop success boolean
    //   <deploy runtime>
    //
    // DELEGATECALL: gas, addr, argsOff, argsSize, retOff, retSize
    // Stack order (top to bottom): [gas, addr, argsOff, argsSize, retOff, retSize]
    // Push in reverse: retSize, retOff, argsSize, argsOff, addr, gas

    // Build proxy constructor bytecode manually.
    // DELEGATECALL(gas, addr, argsOffset, argsSize, retOffset, retSize)
    // In EVM: pops gas(top), addr, argsOff, argsSize, retOff, retSize
    // Push in reverse order: retSize, retOff, argsSize, argsOff, addr, gas
    let impl_hex = hex::encode(impl_addr.as_slice());

    // Constructor prefix (calls DELEGATECALL with no calldata):
    //   PUSH1 0        6000  retSize
    //   PUSH1 0        6000  retOff
    //   PUSH1 0        6000  argsSize
    //   PUSH1 0        6000  argsOff
    //   PUSH20 <addr>  73XX..XX
    //   GAS            5a    (all remaining gas)
    //   DELEGATECALL   f4
    //   POP            50
    let constructor_prefix = format!(
        "60006000600060007{addr}5af450",
        addr = format!("3{}", impl_hex),
    );
    // Bytes: 2+2+2+2+1+20+1+1+1 = 32 bytes

    // Runtime: reads slot 0 and returns it
    //   PUSH1 0, SLOAD, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    let runtime = "60005460005260206000f3"; // 11 bytes

    // Constructor suffix: copy runtime and return it
    //   PUSH1 0x0b (11), DUP1, PUSH1 <offset>, PUSH1 0, CODECOPY, PUSH1 0, RETURN
    // offset = constructor_prefix (32) + suffix (11) = 43 = 0x2b
    let suffix = "600b80602b6000396000f3"; // 11 bytes

    let proxy_bytecode = format!("{}{}{}", constructor_prefix, suffix, runtime);

    let proxy_addr = deploy_contract(
        &proxy_bytecode,
        912691, // different height to avoid inscription conflict
    ).expect("proxy deploy");

    // Read slot 0 from proxy — should be 0xBEEF (written by delegatecall to impl)
    let resp = view_call(&proxy_addr, &[0u8; 4]);
    assert!(resp.success, "view should succeed: {}", resp.error);
    assert_eq!(resp.result.len(), 32);

    let value = u16::from_be_bytes([resp.result[30], resp.result[31]]);
    assert_eq!(value, 0xBEEF, "Delegatecall should have written 0xBEEF to proxy's slot 0");
}

/// Test 4: DELEGATECALL that does nested CREATE
/// Implementation's fallback creates a child contract and stores its address.
/// Proxy delegatecalls this during construction.
/// This matches the BiS_Swap pattern where initialize() does `new UniswapV2Router01()`.
#[test]
fn test_delegatecall_with_nested_create() {
    clear();

    // Implementation runtime: when called, CREATE a child and SSTORE its address at slot 0
    // Child initcode: PUSH1 0xDD, PUSH1 0, SSTORE, STOP (stores 0xDD at slot 0)
    //   60 DD 60 00 55 00 = 6 bytes
    //
    // Impl runtime:
    //   PUSH6 <child_initcode>  65 60DD60005500  (7 bytes)
    //   PUSH1 0                 6000
    //   MSTORE                  52
    //   PUSH1 6                 6006  (child code size)
    //   PUSH1 26               601a  (32-6=26)
    //   PUSH1 0                 6000  (value)
    //   CREATE                  f0
    //   PUSH1 0                 6000  (slot)
    //   SSTORE                  55
    //   STOP                    00
    //
    // = 7+2+1+2+2+2+1+2+1+1 = 21 bytes
    let impl_runtime = "6560dd60005500600052600660_1a6000f060005500"
        .replace("_", "");
    let impl_bytecode = wrap_runtime(&impl_runtime);
    let impl_addr = deploy_contract(&impl_bytecode, 912690).expect("impl deploy");

    // Proxy: DELEGATECALL impl in constructor, then deploy runtime that reads slot 0
    let impl_hex = hex::encode(impl_addr.as_slice());
    let constructor_prefix = format!(
        "60006000600060007{addr}5af450",
        addr = format!("3{}", impl_hex),
    );
    let runtime = "60005460005260206000f3";
    let prefix_len = constructor_prefix.len() / 2; // should be 32
    let suffix_offset = prefix_len + 11; // 43 = 0x2b
    let suffix = format!("600b8060{:02x}6000396000f3", suffix_offset);
    let proxy_bytecode = format!("{}{}{}", constructor_prefix, suffix, runtime);

    let proxy_addr = deploy_contract(&proxy_bytecode, 912691).expect("proxy deploy");

    // Read slot 0 from proxy — should be the child address (non-zero)
    let resp = view_call(&proxy_addr, &[0u8; 4]);
    assert!(resp.success, "view should succeed: {}", resp.error);

    // Child address should be in the last 20 bytes
    let child_addr_bytes = &resp.result[12..32];
    let is_zero = child_addr_bytes.iter().all(|&b| b == 0);
    assert!(!is_zero,
        "DELEGATECALL + nested CREATE should produce non-zero child address in proxy storage");
    let child_hex: String = child_addr_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    // Note: the child was created by the PROXY (via delegatecall), so its address is
    // keccak256(rlp([proxy_addr, proxy_nonce])). This proves nested CREATE inside
    // delegatecall works and state is committed to the proxy's storage.
}
