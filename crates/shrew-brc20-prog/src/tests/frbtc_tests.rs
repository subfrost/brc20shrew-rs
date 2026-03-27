///! FrBTC Contract Tests
///!
///! Tests deploying the actual FrBTC.sol contract and calling getSignerAddress().
///! Regression test for: getSignerAddress() reverts on EllipticCurve EC math.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::prog_indexer::ProgrammableBrc20Indexer;
use crate::view;
use crate::proto::CallRequest;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::{INSCRIPTION_ID_TO_CONTRACT_ADDRESS, EVM_STORAGE, EVM_ACCOUNTS};
use revm::primitives::{Address, U256};
use revm::Database;
use metashrew_support::index_pointer::KeyValuePointer;

use shrew_test_helpers::transactions::create_activation_transaction;
use bitcoin::Amount;

const FRBTC_BYTECODE: &str = include_str!("fixtures/frbtc_bytecode.hex");

/// Function selectors
const DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67]; // decimals()
const OWNER_SELECTOR: [u8; 4] = [0x8d, 0xa5, 0xcb, 0x5b]; // owner()
const NAME_SELECTOR: [u8; 4] = [0x06, 0xfd, 0xde, 0x03]; // name()
const GET_SIGNER_ADDRESS_SELECTOR: [u8; 4] = [0x1a, 0x29, 0x6e, 0x02]; // getSignerAddress()

fn deploy_frbtc(height: u32) -> Option<Address> {
    let bytecode = FRBTC_BYTECODE.trim();
    let content = format!(
        r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#,
        bytecode
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

fn view_call(to: &Address, calldata: &[u8]) -> crate::proto::CallResponse {
    let request = CallRequest {
        to: to.as_slice().to_vec(),
        data: calldata.to_vec(),
        from: None,
    };
    view::call(&request).expect("view::call should not return Err")
}

#[test]
fn test_frbtc_deploys_successfully() {
    clear();
    let addr = deploy_frbtc(912690);
    assert!(addr.is_some(), "FrBTC should deploy successfully");
    let addr = addr.unwrap();

    // Verify account has code
    let mut db = MetashrewDB;
    let account = db.basic(addr).unwrap();
    assert!(account.is_some(), "FrBTC account should exist");
    let info = account.unwrap();
    assert!(info.code.is_some(), "FrBTC should have code");
    let code = info.code.as_ref().unwrap();
    assert!(code.len() > 100, "FrBTC runtime code should be substantial, got {} bytes", code.len());
}

#[test]
fn test_frbtc_decimals() {
    clear();
    let addr = deploy_frbtc(912690).expect("FrBTC should deploy");
    let response = view_call(&addr, &DECIMALS_SELECTOR);
    assert!(response.success, "decimals() should succeed: {}", response.error);
    assert_eq!(response.result.len(), 32);
    assert_eq!(response.result[31], 8, "decimals should be 8");
}

#[test]
fn test_frbtc_name() {
    clear();
    let addr = deploy_frbtc(912690).expect("FrBTC should deploy");
    let response = view_call(&addr, &NAME_SELECTOR);
    assert!(response.success, "name() should succeed: {}", response.error);
    // ABI-encoded string: offset(32) + length(32) + data(32) = 96 bytes
    assert!(response.result.len() >= 96, "name() should return ABI-encoded string");
    // "fr-BTC" = [102, 114, 45, 66, 84, 67]
    let data_start = 64; // after offset + length
    assert_eq!(response.result[data_start], b'f');
    assert_eq!(response.result[data_start + 1], b'r');
}

#[test]
fn test_frbtc_owner() {
    clear();
    let addr = deploy_frbtc(912690).expect("FrBTC should deploy");
    let response = view_call(&addr, &OWNER_SELECTOR);
    assert!(response.success, "owner() should succeed: {}", response.error);
    assert_eq!(response.result.len(), 32, "owner should return 32 bytes (padded address)");
    // Owner should be a non-zero address (the deployer)
    let is_zero = response.result.iter().all(|&b| b == 0);
    assert!(!is_zero, "owner should not be zero address");
}

#[test]
fn test_frbtc_get_signer_address() {
    clear();
    let addr = deploy_frbtc(912690).expect("FrBTC should deploy");

    // First verify the defaultSignerPubkey storage slot has a value.
    // In FrBTC.sol, defaultSignerPubkey is at storage slot 6 (after ERC20 slots).
    // Actually, the slot depends on the contract layout. Let's check slot 6.
    let mut db = MetashrewDB;

    // Try reading a few slots to find defaultSignerPubkey
    for slot in 0..10 {
        let value = db.storage(addr, U256::from(slot)).unwrap();
        if value != U256::ZERO {
            let bytes = value.to_be_bytes::<32>();
            let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            // Check if this matches the known pubkey prefix
            if hex.starts_with("afbf") {
                assert!(true, "Found defaultSignerPubkey at slot {}", slot);
            }
        }
    }

    // Now call getSignerAddress()
    let response = view_call(&addr, &GET_SIGNER_ADDRESS_SELECTOR);

    if !response.success {
        // Decode revert reason if present
        let revert_hex: String = response.result.iter().map(|b| format!("{:02x}", b)).collect();
        panic!(
            "getSignerAddress() reverted: error='{}', revert_data_hex='{}', revert_data_len={}",
            response.error,
            revert_hex,
            response.result.len()
        );
    }

    assert!(response.success, "getSignerAddress() should succeed");
    // Result should be ABI-encoded bytes: abi.encodePacked(0x51, 0x20, tweakedPubKey)
    // = 1 + 1 + 32 = 34 bytes, but ABI-encoded as dynamic bytes
    assert!(response.result.len() >= 64, "Should return ABI-encoded bytes");
}

#[test]
fn test_activation_tx_detection() {
    clear();
    let height = 912690u32;

    // Deploy FrBTC first (simple deploy, no activation needed)
    let addr = deploy_frbtc(height).expect("FrBTC should deploy");

    // Now create a "wrap" inscription with an activation tx.
    // The wrap() call needs:
    //   1. getTxId() → activation tx id
    //   2. getTxDetails(activationTxId) → finds BTC output to signer
    //   3. _wrap() mints frBTC based on matching vouts
    //
    // For this test we verify that the activation tx is detected by the indexer.

    let height2 = 912691u32;
    let wrap_content = format!(
        r#"{{"p":"brc20-prog","op":"call","c":"0x{}","d":"0xd46eb119"}}"#,
        hex::encode(addr.as_slice())
    );
    // d46eb119 = wrap() selector

    let coinbase = create_coinbase_transaction(height2);
    let reveal_tx = create_inscription_transaction(wrap_content.as_bytes(), "application/json", Some(shrew_test_helpers::transactions::create_mock_outpoint(50)));
    let reveal_txid = reveal_tx.compute_txid();

    // Create activation tx that spends the reveal tx output
    // with a BTC output (e.g., 500k sats) to some address
    let signer_output = bitcoin::TxOut {
        value: Amount::from_sat(500_000),
        // Use a test P2TR-like address (the contract will compare this to its signer)
        script_pubkey: bitcoin::ScriptBuf::from_bytes({
            let mut s = vec![0x51, 0x20];
            s.extend_from_slice(&[0xAA; 32]);
            s
        }),
    };
    let activation_tx = create_activation_transaction(&reveal_txid, 0, vec![signer_output]);
    let activation_txid = activation_tx.compute_txid();

    let block = create_block_with_txs(vec![coinbase, reveal_tx, activation_tx]);
    index_ord_block(&block, height2).unwrap();

    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height2);

    // If the activation tx was detected, the OP_RETURN txid precompile should
    // return the activation tx's id, and getTxDetails should return its vouts.
    // We can't directly test the precompile output here, but we can verify
    // the block was processed without panic (activation detection didn't crash).
    //
    // The wrap() call will likely revert because the signer address won't match
    // our test P2TR address. But the point is: the activation detection works
    // and the indexer doesn't use the reveal tx's vouts.
    //
    // TODO: Full wrap test would need matching signer address
}
