///! Activation TX byte flow debug tests
///!
///! Traces the full byte flow from activation tx detection through
///! to the getTxDetails precompile lookup.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::prog_indexer::ProgrammableBrc20Indexer;
use crate::view;
use crate::proto::CallRequest;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::{create_inscription_transaction, create_activation_transaction, create_mock_outpoint};
use shrew_test_helpers::indexing::index_ord_block;
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::{INSCRIPTION_ID_TO_CONTRACT_ADDRESS, EVM_STORAGE};
use shrew_ord::tables::TXID_TO_RAW_TX;
use revm::primitives::{Address, U256};
use revm::Database;
use metashrew_support::index_pointer::KeyValuePointer;
use bitcoin::Amount;

const FRBTC_BYTECODE: &str = include_str!("fixtures/frbtc_bytecode.hex");

fn deploy_frbtc_at(height: u32) -> Address {
    let bytecode = FRBTC_BYTECODE.trim();
    let content = format!(r#"{{"p":"brc20-prog","op":"deploy","d":"{}"}}"#, bytecode);
    let coinbase = create_coinbase_transaction(height);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![coinbase, tx.clone()]);
    index_ord_block(&block, height).unwrap();
    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height);
    let inscription_id = shrew_support::inscription::InscriptionId::new(tx.txid(), 0);
    let addr_bytes = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.to_bytes()).get();
    assert!(!addr_bytes.is_empty(), "FrBTC should deploy");
    Address::from_slice(&addr_bytes)
}

fn view_call(to: &Address, calldata: &[u8]) -> crate::proto::CallResponse {
    let request = CallRequest { to: to.as_slice().to_vec(), data: calldata.to_vec(), from: None };
    view::call(&request).expect("view::call should not Err")
}

fn decode_uint256(result: &[u8]) -> U256 {
    if result.len() < 32 { return U256::ZERO; }
    U256::from_be_slice(&result[..32])
}

/// Step 1: Verify the activation mapping is stored correctly
#[test]
fn test_activation_mapping_stored() {
    clear();
    let frbtc = deploy_frbtc_at(912690);

    let height2 = 912691u32;
    let wrap_content = format!(
        r#"{{"p":"brc20-prog","op":"call","c":"0x{}","d":"0xd46eb119"}}"#,
        hex::encode(frbtc.as_slice())
    );
    let coinbase = create_coinbase_transaction(height2);
    let reveal_tx = create_inscription_transaction(wrap_content.as_bytes(), "application/json", Some(create_mock_outpoint(50)));
    let reveal_txid = reveal_tx.compute_txid();

    let activation_tx = create_activation_transaction(&reveal_txid, 0, vec![]);
    let activation_txid = activation_tx.compute_txid();

    let block = create_block_with_txs(vec![coinbase, reveal_tx, activation_tx]);
    index_ord_block(&block, height2).unwrap();
    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height2);

    // Check activation mapping
    let ptr = metashrew_core::index_pointer::IndexPointer::from_keyword("/prog/activation_map/")
        .select(&reveal_txid[..].to_vec());
    let stored = ptr.get();
    assert!(!stored.is_empty(), "Activation mapping should be stored");
    assert_eq!(stored.len(), 32, "Activation mapping should be 32 bytes (txid)");

    // The stored bytes should be the activation txid in LE (internal) order
    let stored_bytes: Vec<u8> = stored.to_vec();
    let expected_bytes: Vec<u8> = activation_txid[..].to_vec();
    assert_eq!(stored_bytes, expected_bytes,
        "Stored activation txid should match activation_tx.compute_txid() in LE order");
}

/// Step 2: Verify the activation txid is in TXID_TO_RAW_TX
#[test]
fn test_activation_tx_in_raw_tx_table() {
    clear();
    let frbtc = deploy_frbtc_at(912690);

    let height2 = 912691u32;
    let wrap_content = format!(
        r#"{{"p":"brc20-prog","op":"call","c":"0x{}","d":"0xd46eb119"}}"#,
        hex::encode(frbtc.as_slice())
    );
    let coinbase = create_coinbase_transaction(height2);
    let reveal_tx = create_inscription_transaction(wrap_content.as_bytes(), "application/json", Some(create_mock_outpoint(50)));
    let reveal_txid = reveal_tx.compute_txid();

    let activation_tx = create_activation_transaction(&reveal_txid, 0, vec![]);
    let activation_txid = activation_tx.compute_txid();

    let block = create_block_with_txs(vec![coinbase, reveal_tx, activation_tx]);
    index_ord_block(&block, height2).unwrap();

    // The ord indexer should have stored all txs including the activation tx
    let activation_raw = TXID_TO_RAW_TX.select(&activation_txid[..].to_vec()).get();
    assert!(!activation_raw.is_empty(),
        "Activation tx should be in TXID_TO_RAW_TX (stored by ord indexer)");

    // Also check the reveal tx is stored
    let reveal_raw = TXID_TO_RAW_TX.select(&reveal_txid[..].to_vec()).get();
    assert!(!reveal_raw.is_empty(),
        "Reveal tx should be in TXID_TO_RAW_TX");
}

/// Step 3: Verify the precompile 0xFA returns the right txid
/// and precompile 0xFD can look it up.
/// We do this indirectly by calling totalSupply after a wrap.
#[test]
fn test_wrap_mints_with_activation_tx() {
    clear();
    let frbtc = deploy_frbtc_at(912690);

    // First get the signer address from the contract
    // getSignerAddress() = 0x1a296e02
    let gsa_resp = view_call(&frbtc, &[0x1a, 0x29, 0x6e, 0x02]);
    assert!(gsa_resp.success, "getSignerAddress should succeed: {}", gsa_resp.error);
    assert!(gsa_resp.result.len() >= 64, "Should return ABI-encoded bytes");

    // Decode the signer script from the ABI response
    // ABI: offset(32) + length(32) + data(...)
    let offset = U256::from_be_slice(&gsa_resp.result[0..32]).to::<usize>();
    let length = U256::from_be_slice(&gsa_resp.result[offset..offset+32]).to::<usize>();
    let signer_script = gsa_resp.result[offset+32..offset+32+length].to_vec();

    // The signer script should be a P2TR script: 0x51 0x20 <32-byte-pubkey>
    assert_eq!(signer_script.len(), 34, "Signer script should be 34 bytes (P2TR)");
    assert_eq!(signer_script[0], 0x51, "P2TR version byte");
    assert_eq!(signer_script[1], 0x20, "P2TR push 32 bytes");

    // Now create a wrap inscription with an activation tx
    // The activation tx needs an output matching the signer script
    let height2 = 912691u32;
    let wrap_content = format!(
        r#"{{"p":"brc20-prog","op":"call","c":"0x{}","d":"0xd46eb119"}}"#,
        hex::encode(frbtc.as_slice())
    );
    let coinbase = create_coinbase_transaction(height2);
    let reveal_tx = create_inscription_transaction(wrap_content.as_bytes(), "application/json", Some(create_mock_outpoint(50)));
    let reveal_txid = reveal_tx.compute_txid();

    // Create activation tx with output matching signer
    let signer_output = bitcoin::TxOut {
        value: Amount::from_sat(500_000),
        script_pubkey: bitcoin::ScriptBuf::from_bytes(signer_script.clone()),
    };
    let activation_tx = create_activation_transaction(&reveal_txid, 0, vec![signer_output]);

    let block = create_block_with_txs(vec![coinbase, reveal_tx, activation_tx]);
    index_ord_block(&block, height2).unwrap();
    let mut prog = ProgrammableBrc20Indexer::new();
    prog.index_block(&block, height2);

    // Check totalSupply — should be 500,000 if wrap minted correctly
    // totalSupply() = 0x18160ddd
    let supply_resp = view_call(&frbtc, &[0x18, 0x16, 0x0d, 0xdd]);
    assert!(supply_resp.success, "totalSupply should succeed: {}", supply_resp.error);
    let supply = decode_uint256(&supply_resp.result);

    if supply == U256::ZERO {
        // Debug: check the activation mapping
        let map_ptr = metashrew_core::index_pointer::IndexPointer::from_keyword("/prog/activation_map/")
            .select(&reveal_txid[..].to_vec());
        let stored_activation = map_ptr.get();
        let has_mapping = !stored_activation.is_empty();

        // Check if the stored activation txid matches what's in TXID_TO_RAW_TX
        let activation_in_raw = if stored_activation.len() == 32 {
            !TXID_TO_RAW_TX.select(&stored_activation.to_vec()).get().is_empty()
        } else {
            false
        };

        // Check what resolve_op_return_tx_id would return
        // It reads the mapping, reverses bytes LE→BE, and returns B256
        let reversed_stored: Vec<u8> = if stored_activation.len() == 32 {
            let mut b = stored_activation.to_vec();
            b.reverse();
            b
        } else {
            vec![]
        };

        // The precompile receives this BE value from Solidity, reverses to LE, and looks up
        // So it should look up the ORIGINAL stored bytes (before reversal)
        let precompile_lookup_key = stored_activation.to_vec();
        let precompile_finds_tx = !TXID_TO_RAW_TX.select(&precompile_lookup_key).get().is_empty();

        panic!(
            "totalSupply is 0 — wrap() failed to mint!\n\
             Debug info:\n\
             - has_activation_mapping: {}\n\
             - stored_activation_len: {}\n\
             - activation_in_TXID_TO_RAW_TX: {}\n\
             - precompile_would_find_tx: {}\n\
             - stored_hex: {}\n\
             - reversed_hex: {}",
            has_mapping,
            stored_activation.len(),
            activation_in_raw,
            precompile_finds_tx,
            hex::encode(&stored_activation.to_vec()),
            hex::encode(&reversed_stored),
        );
    }

    assert!(supply > U256::ZERO, "totalSupply should be > 0 after wrap");
    assert_eq!(supply, U256::from(500_000), "totalSupply should be 500,000 sats");
}
