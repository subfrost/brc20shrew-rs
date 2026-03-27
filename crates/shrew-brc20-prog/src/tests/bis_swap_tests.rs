///! BiS_Swap proxy + initialize() debugging tests
///!
///! Tests deploying the actual BiS_Swap bytecode (from production) and
///! calling initialize() through a MinimalProxy to identify why the
///! nested CREATE chain fails.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::prog_indexer::ProgrammableBrc20Indexer;
use crate::view;
use crate::proto::CallRequest;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::{create_inscription_transaction, create_mock_outpoint};
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::INSCRIPTION_ID_TO_CONTRACT_ADDRESS;
use revm::primitives::{Address, U256};
use revm::Database;
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::atomic::{AtomicU32, Ordering};

static OUTPOINT_N: AtomicU32 = AtomicU32::new(200);

const BIS_SWAP_HEX: &str = include_str!("fixtures/bis_swap.hex");
const MINIMAL_PROXY_HEX: &str = include_str!("fixtures/minimal_proxy.hex");

fn next_outpoint() -> bitcoin::OutPoint {
    create_mock_outpoint(OUTPOINT_N.fetch_add(1, Ordering::SeqCst))
}

fn deploy(bytecode_hex: &str, height: u32) -> Option<Address> {
    let hex = bytecode_hex.trim();
    let content = format!(r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#, hex);
    let coinbase = create_coinbase_transaction(height);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", Some(next_outpoint()));
    let block = create_block_with_txs(vec![coinbase, tx.clone()]);
    index_ord_block(&block, height).unwrap();
    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);
    let id = shrew_support::inscription::InscriptionId::new(tx.txid(), 0);
    let addr = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&id.to_bytes()).get();
    if addr.is_empty() { None } else { Some(Address::from_slice(&addr)) }
}

fn call_contract(contract: &Address, calldata_hex: &str, height: u32) {
    let content = format!(
        r#"{{"p":"brc20-prog","op":"call","c":"0x{}","d":"0x{}"}}"#,
        hex::encode(contract.as_slice()),
        calldata_hex,
    );
    let coinbase = create_coinbase_transaction(height);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", Some(next_outpoint()));
    let block = create_block_with_txs(vec![coinbase, tx.clone()]);
    index_ord_block(&block, height).unwrap();
    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);
}

fn view_call(to: &Address, calldata: &[u8]) -> crate::proto::CallResponse {
    let req = CallRequest { to: to.as_slice().to_vec(), data: calldata.to_vec(), from: None };
    view::call(&req).expect("view")
}

fn decode_address(result: &[u8]) -> Address {
    if result.len() < 32 { return Address::ZERO; }
    Address::from_slice(&result[12..32])
}

fn pad_address(addr: &Address) -> String {
    format!("{:0>64}", hex::encode(addr.as_slice()))
}

fn pad_uint256(val: u64) -> String {
    format!("{:0>64x}", val)
}

// ============================================================================
// Step 1: Deploy BiS_Swap implementation alone
// ============================================================================

#[test]
fn test_bis_swap_deploys() {
    clear();
    let addr = deploy(BIS_SWAP_HEX, 912690);
    assert!(addr.is_some(), "BiS_Swap should deploy");
    let addr = addr.unwrap();

    // Verify it has code
    let mut db = MetashrewDB;
    let account = db.basic(addr).unwrap();
    assert!(account.is_some(), "Account should exist");
    let code = account.unwrap().code;
    assert!(code.is_some() && !code.as_ref().unwrap().is_empty(), "Should have code");
}

// ============================================================================
// Step 2: Deploy MinimalProxy pointing to BiS_Swap impl
// ============================================================================

#[test]
fn test_minimal_proxy_deploys_for_bis_swap() {
    clear();

    // Deploy impl
    let impl_addr = deploy(BIS_SWAP_HEX, 912690).expect("impl deploy");

    // Build proxy bytecode with constructor args: (address _logic, bytes _data)
    // _data = empty bytes (we'll call initialize separately)
    let proxy_hex = MINIMAL_PROXY_HEX.trim();
    let constructor_args = format!(
        "{}{}{}",
        pad_address(&impl_addr),            // _logic
        pad_uint256(64),                     // offset to _data = 0x40
        pad_uint256(0),                      // _data length = 0
    );
    let full_hex = format!("{}{}", proxy_hex, constructor_args);

    let proxy_addr = deploy(&full_hex, 912691).expect("proxy deploy");

    // Proxy should have code
    let mut db = MetashrewDB;
    let account = db.basic(proxy_addr).unwrap();
    assert!(account.is_some(), "Proxy should have account");

    // Proxy should be different from impl
    assert_ne!(proxy_addr, impl_addr, "Proxy and impl should have different addresses");

    // Call BTC_UPSCALE() = 099e7b8d on proxy — should delegate to impl and return 0
    let resp = view_call(&proxy_addr, &hex::decode("099e7b8d").unwrap());
    assert!(resp.success, "BTC_UPSCALE view should succeed: {}", resp.error);
    let value = U256::from_be_slice(&resp.result);
    assert_eq!(value, U256::ZERO, "BTC_UPSCALE should be 0 (not initialized)");
}

// ============================================================================
// Step 3: Call initialize() on proxy — THE KEY TEST
// ============================================================================

#[test]
fn test_initialize_on_proxy() {
    clear();

    // Deploy impl
    let impl_addr = deploy(BIS_SWAP_HEX, 912690).expect("impl deploy");

    // Deploy proxy with empty init
    let proxy_hex = MINIMAL_PROXY_HEX.trim();
    let constructor_args = format!("{}{}{}", pad_address(&impl_addr), pad_uint256(64), pad_uint256(0));
    let proxy_addr = deploy(&format!("{}{}", proxy_hex, constructor_args), 912691).expect("proxy deploy");

    // Build initialize() calldata
    // initialize(address,address,address,address,uint256,address) = 53c425c1
    // Use a test admin address (we'll use the deployer's address)
    let admin = Address::from_slice(&[0xAA; 20]);
    let frbtc = Address::from_slice(&[0xBB; 20]);

    let init_calldata = format!(
        "53c425c1{}{}{}{}{}{}",
        pad_address(&admin),   // _depositSignerWallet
        pad_address(&admin),   // _batchExecutorAddress
        pad_address(&admin),   // _feeTo
        pad_address(&frbtc),   // _wrappedBTCAddress
        pad_uint256(1),        // _btcUpscale
        pad_address(&admin),   // _adminWallet
    );

    // Call initialize on the proxy
    call_contract(&proxy_addr, &init_calldata, 912692);

    // Check owner() = 8da5cb5b
    let owner_resp = view_call(&proxy_addr, &hex::decode("8da5cb5b").unwrap());
    if owner_resp.success {
        let owner = decode_address(&owner_resp.result);
        if owner == Address::ZERO {
            // initialize() reverted — check which part failed
            // Try reading BTC_UPSCALE to see if ANY state was set
            let upscale_resp = view_call(&proxy_addr, &hex::decode("099e7b8d").unwrap());
            let upscale = if upscale_resp.success { U256::from_be_slice(&upscale_resp.result) } else { U256::ZERO };

            // Check batchExecutorAddress
            let batch_resp = view_call(&proxy_addr, &hex::decode("33e748b9").unwrap());
            let batch = if batch_resp.success { decode_address(&batch_resp.result) } else { Address::ZERO };

            // Check uniswapRouter
            let router_resp = view_call(&proxy_addr, &hex::decode("735de9f7").unwrap());
            let router = if router_resp.success { decode_address(&router_resp.result) } else { Address::ZERO };

            panic!(
                "initialize() did not set owner!\n\
                 Debug state on proxy:\n\
                 - owner: {:?}\n\
                 - BTC_UPSCALE: {}\n\
                 - batchExecutorAddress: {:?}\n\
                 - uniswapRouter: {:?}\n\
                 This means initialize() reverted. Likely cause:\n\
                 - new UniswapV2Router01() failed (nested CREATE)\n\
                 - or __ReentrancyGuardTransient_init() failed (needs TSTORE/PRAGUE)",
                owner, upscale, batch, router,
            );
        }
        assert_ne!(owner, Address::ZERO, "Owner should be set after initialize");
        assert_eq!(owner, admin, "Owner should be the admin we passed");
    } else {
        panic!("owner() view failed: {}", owner_resp.error);
    }

    // Check uniswapRouter was created
    let router_resp = view_call(&proxy_addr, &hex::decode("735de9f7").unwrap());
    assert!(router_resp.success, "uniswapRouter view should succeed");
    let router = decode_address(&router_resp.result);
    assert_ne!(router, Address::ZERO, "uniswapRouter should be created by initialize()");

    // Check wrappedBTCAddress
    let wbtc_resp = view_call(&proxy_addr, &hex::decode("240cfb28").unwrap());
    assert!(wbtc_resp.success);
    let wbtc = decode_address(&wbtc_resp.result);
    assert_eq!(wbtc, frbtc, "wrappedBTCAddress should be frBTC");
}

/// Test 3b: Minimal initializable + proxy test
/// Deploy a simple contract with an initializer that just writes to storage,
/// then call it through a proxy. No nested CREATEs.
#[test]
fn test_simple_initializable_through_proxy() {
    clear();

    // "Implementation": a contract whose fallback uses CALLDATALOAD to read
    // first arg as a value and SSTORE it to slot 0, slot 1 = msg.sender
    // Runtime:
    //   PUSH1 0, CALLDATALOAD, PUSH1 0, SSTORE   (store calldata[0:32] at slot 0)
    //   CALLER, PUSH1 1, SSTORE                    (store msg.sender at slot 1)
    //   STOP
    //
    // Bytecode:
    //   6000 35 6000 55 33 6001 55 00
    //   = 10 bytes
    let impl_runtime = "600035600055336001_5500".replace("_", "");
    let impl_hex = format!("60{:02x}80600b6000396000f3{}", impl_runtime.len()/2, impl_runtime);
    let impl_addr = deploy(&impl_hex, 912690).expect("impl deploy");

    // Proxy: delegates to impl
    let proxy_hex = MINIMAL_PROXY_HEX.trim();
    let constructor_args = format!("{}{}{}", pad_address(&impl_addr), pad_uint256(64), pad_uint256(0));
    let proxy_addr = deploy(&format!("{}{}", proxy_hex, constructor_args), 912691).expect("proxy deploy");

    // Verify proxy has code
    let mut db_check = MetashrewDB;
    let proxy_account = db_check.basic(proxy_addr).unwrap();
    assert!(proxy_account.is_some(), "Proxy should have account");
    let proxy_code = proxy_account.unwrap().code;
    let code_len = proxy_code.as_ref().map(|c| c.len()).unwrap_or(0);
    assert!(code_len > 0, "Proxy should have runtime code, got {} bytes", code_len);

    // Call proxy with calldata = 0xDEADBEEF (padded to 32 bytes)
    // This should delegatecall to impl which stores 0xDEADBEEF at slot 0
    let calldata = format!("{:0>64}", "deadbeef");
    call_contract(&proxy_addr, &calldata, 912692);

    // Read slot 0 from proxy — should be 0xDEADBEEF
    // Runtime for reading: PUSH1 0, SLOAD, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
    // But we need to call the proxy, which delegates to impl's fallback...
    // Actually, the proxy only has a fallback. Any call gets delegated.
    // The impl's fallback stores calldata[0:32] at slot 0. But when we
    // want to READ, we need a different function.
    //
    // Let's deploy a different impl that has two modes:
    //   - If calldata[0:4] == 0x01: SSTORE(0, calldata[4:36])
    //   - If calldata[0:4] == 0x02: SLOAD(0), MSTORE, RETURN
    //
    // Actually, let me just verify by directly reading storage from MetashrewDB
    // Read debug info from commit
    let proxy_debug_key = format!("/debug/commit/{}", hex::encode(proxy_addr.as_slice()));
    let debug_ptr = metashrew_core::index_pointer::IndexPointer::from_keyword(&proxy_debug_key);
    let debug_val = debug_ptr.get();
    let debug_str = String::from_utf8_lossy(&debug_val);

    let impl_debug_key = format!("/debug/commit/{}", hex::encode(impl_addr.as_slice()));
    let impl_debug_ptr = metashrew_core::index_pointer::IndexPointer::from_keyword(&impl_debug_key);
    let impl_debug_val = impl_debug_ptr.get();
    let impl_debug_str = String::from_utf8_lossy(&impl_debug_val);

    let mut db = MetashrewDB;
    let val = db.storage(proxy_addr, U256::ZERO).unwrap();

    if val == U256::ZERO {
        panic!(
            "Proxy storage slot 0 is ZERO after call!\n\
             Proxy commit debug: '{}'\n\
             Impl commit debug: '{}'\n\
             This means the call through proxy → delegatecall → impl\n\
             did not persist storage changes. Possible causes:\n\
             - transact_commit() returned Revert (call failed)\n\
             - Proxy account not in changes map\n\
             - Storage not marked as changed",
            debug_str, impl_debug_str,
        );
    }

    let expected = U256::from_be_slice(&hex::decode(format!("{:0>64}", "deadbeef")).unwrap());
    assert_eq!(val, expected, "Slot 0 should contain 0xDEADBEEF");
}

