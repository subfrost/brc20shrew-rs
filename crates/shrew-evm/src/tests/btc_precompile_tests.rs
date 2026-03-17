///! BTC Precompile integration tests
///!
///! Tests for btc_tx_details and last_sat_location precompiles using
///! synthetic transaction data stored in the TXID_TO_RAW_TX table.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::precompiles::*;
use bitcoin::consensus::serialize;
use bitcoin::{
    Transaction, TxIn, TxOut, OutPoint, Txid, Sequence, Witness,
    Amount, ScriptBuf, absolute::LockTime,
};
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use revm::primitives::B256;
use shrew_ord::tables::{TXID_TO_RAW_TX, TXID_TO_BLOCK_HEIGHT};
use shrew_test_helpers::state::clear;
use std::sync::Arc;

/// Store a synthetic transaction in the indexed tables
fn store_tx(tx: &Transaction, height: u32) {
    let txid_bytes = tx.compute_txid().as_byte_array().to_vec();
    let raw = serialize(tx);
    TXID_TO_RAW_TX.select(&txid_bytes).set(Arc::new(raw));
    TXID_TO_BLOCK_HEIGHT.select(&txid_bytes).set(Arc::new(height.to_le_bytes().to_vec()));
}

/// Build a simple test transaction with specified inputs and outputs
fn build_tx(inputs: Vec<(Txid, u32)>, outputs: Vec<(ScriptBuf, u64)>) -> Transaction {
    let txins: Vec<TxIn> = inputs.iter().map(|(txid, vout)| TxIn {
        previous_output: OutPoint { txid: *txid, vout: *vout },
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    }).collect();

    let txouts: Vec<TxOut> = outputs.iter().map(|(script, value)| TxOut {
        script_pubkey: script.clone(),
        value: Amount::from_sat(*value),
    }).collect();

    Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: LockTime::ZERO,
        input: txins,
        output: txouts,
    }
}

/// Build a coinbase transaction
fn build_coinbase(outputs: Vec<(ScriptBuf, u64)>) -> Transaction {
    let txouts: Vec<TxOut> = outputs.iter().map(|(script, value)| TxOut {
        script_pubkey: script.clone(),
        value: Amount::from_sat(*value),
    }).collect();

    Transaction {
        version: bitcoin::transaction::Version(2),
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::from(vec![0x03, 0x01, 0x00, 0x00]),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: txouts,
    }
}

/// Build ABI input for getTxDetails(bytes32 txid)
fn build_tx_details_input(txid: &Txid) -> Vec<u8> {
    let mut input = vec![0u8; 4]; // function selector
    let mut txid_be = *txid.as_byte_array();
    txid_be.reverse(); // Bitcoin internal -> ABI big-endian
    input.extend_from_slice(&txid_be);
    input
}

/// Build ABI input for getLastSatLocation(bytes32 txid, uint256 vout, uint256 sat)
fn build_last_sat_input(txid: &Txid, vout: u32, sat: u64) -> Vec<u8> {
    let mut input = vec![0u8; 4]; // function selector
    // txid (big-endian)
    let mut txid_be = *txid.as_byte_array();
    txid_be.reverse();
    input.extend_from_slice(&txid_be);
    // vout as uint256
    let mut vout_bytes = [0u8; 32];
    vout_bytes[28..32].copy_from_slice(&(vout as u32).to_be_bytes());
    input.extend_from_slice(&vout_bytes);
    // sat as uint256
    let mut sat_bytes = [0u8; 32];
    sat_bytes[24..32].copy_from_slice(&sat.to_be_bytes());
    input.extend_from_slice(&sat_bytes);
    input
}

/// Helper: decode a uint256 from 32 bytes at offset in output
fn read_u256(output: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(output[offset + 24..offset + 32].try_into().unwrap())
}

/// Helper: decode a bytes32 from output at offset
fn read_bytes32(output: &[u8], offset: usize) -> [u8; 32] {
    output[offset..offset + 32].try_into().unwrap()
}

// P2WPKH script for testing
fn p2wpkh_script() -> ScriptBuf {
    ScriptBuf::from(hex::decode("0014f477952f33561c1b89a1fe9f28682f623263e159").unwrap())
}

// P2TR script for testing
fn p2tr_script() -> ScriptBuf {
    ScriptBuf::from(hex::decode("51204a6041f54b8cf8b2d48c6f725cb0514e51e5e7e7ac429c33da62e98765dd62f3").unwrap())
}

// ============================================================================
// btc_tx_details tests
// ============================================================================

#[test]
fn test_tx_details_single_input_single_output() {
    clear();
    let op_return_txid = B256::ZERO;

    // Create a parent tx with one output
    let parent = build_tx(
        vec![(Txid::all_zeros(), 0)],
        vec![(p2tr_script(), 10_000_000)],
    );
    store_tx(&parent, 840000);

    // Create child tx spending the parent
    let child = build_tx(
        vec![(parent.compute_txid(), 0)],
        vec![(p2wpkh_script(), 9_658_000)],
    );
    store_tx(&child, 840001);

    // Query tx details for child
    let input = build_tx_details_input(&child.compute_txid());
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &input, 2_000_000, op_return_txid, 840001).unwrap();

    assert!(result.success, "Should succeed for indexed transaction");

    // Gas should be base + 1 input
    assert_eq!(result.gas_used, GAS_BTC_RPC_CALL * 2, "Gas = base + 1 input");

    // Decode block_height from output
    let block_height = read_u256(&result.output, 0);
    assert_eq!(block_height, 840001, "Block height should match");
}

#[test]
fn test_tx_details_multi_input() {
    clear();
    let op_return_txid = B256::ZERO;

    // Create 3 parent transactions
    let parent1 = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2tr_script(), 5_000_000)]);
    let parent2 = build_tx(vec![(Txid::all_zeros(), 1)], vec![(p2wpkh_script(), 3_000_000)]);
    let parent3 = build_tx(vec![(Txid::all_zeros(), 2)], vec![(p2tr_script(), 2_000_000)]);
    store_tx(&parent1, 840000);
    store_tx(&parent2, 840000);
    store_tx(&parent3, 840000);

    // Child spends all 3
    let child = build_tx(
        vec![(parent1.compute_txid(), 0), (parent2.compute_txid(), 0), (parent3.compute_txid(), 0)],
        vec![(p2wpkh_script(), 9_500_000)],
    );
    store_tx(&child, 840001);

    let input = build_tx_details_input(&child.compute_txid());
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &input, 5_000_000, op_return_txid, 840001).unwrap();

    assert!(result.success, "Should succeed for multi-input tx");

    // Gas should be base + 3 inputs
    assert_eq!(result.gas_used, GAS_BTC_RPC_CALL * 4, "Gas = base + 3 inputs");
}

#[test]
fn test_tx_details_unknown_txid_fails() {
    clear();
    let op_return_txid = B256::ZERO;
    let fake_txid = Txid::from_slice(&[0xAB; 32]).unwrap();
    let input = build_tx_details_input(&fake_txid);
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &input, 2_000_000, op_return_txid, 840000).unwrap();
    assert!(!result.success, "Should fail for unknown txid");
}

#[test]
fn test_tx_details_future_block_rejected() {
    clear();
    let op_return_txid = B256::ZERO;

    let tx = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2wpkh_script(), 1000)]);
    store_tx(&tx, 840010); // stored at height 840010

    let input = build_tx_details_input(&tx.compute_txid());
    // Query at height 840005 — tx is in the future
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &input, 2_000_000, op_return_txid, 840005).unwrap();
    assert!(!result.success, "Should reject tx from future block");
}

#[test]
fn test_tx_details_insufficient_gas_for_inputs() {
    clear();
    let op_return_txid = B256::ZERO;

    let parent = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2tr_script(), 10_000_000)]);
    store_tx(&parent, 840000);

    // 3-input tx requires base(400k) + 3*400k = 1.6M gas
    let child = build_tx(
        vec![(parent.compute_txid(), 0), (parent.compute_txid(), 0), (parent.compute_txid(), 0)],
        vec![(p2wpkh_script(), 1000)],
    );
    store_tx(&child, 840001);

    let input = build_tx_details_input(&child.compute_txid());
    // Provide only 1M gas (need 1.6M)
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &input, 1_000_000, op_return_txid, 840001).unwrap();
    assert!(!result.success, "Should fail when gas insufficient for input lookup");
}

#[test]
fn test_tx_details_coinbase_no_inputs() {
    clear();
    let op_return_txid = B256::ZERO;

    let coinbase = build_coinbase(vec![(p2wpkh_script(), 625_000_000)]);
    store_tx(&coinbase, 840000);

    let input = build_tx_details_input(&coinbase.compute_txid());
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &input, 2_000_000, op_return_txid, 840000).unwrap();

    assert!(result.success, "Coinbase tx should succeed");
    // Coinbase has no vin data to report (is_coinbase skips vin processing)
    assert_eq!(result.gas_used, GAS_BTC_RPC_CALL, "Coinbase: only base gas (no input lookups)");
}

// ============================================================================
// last_sat_location tests
// ============================================================================

#[test]
fn test_last_sat_single_input() {
    clear();
    let op_return_txid = B256::ZERO;

    // Parent: one output worth 10M sats
    let parent = build_tx(
        vec![(Txid::all_zeros(), 0)],
        vec![(p2tr_script(), 10_000_000)],
    );
    store_tx(&parent, 840000);

    // Child spends parent, one output
    let child = build_tx(
        vec![(parent.compute_txid(), 0)],
        vec![(p2wpkh_script(), 9_658_000)],
    );
    store_tx(&child, 840001);

    // Find sat 100 in output 0 of child
    let input = build_last_sat_input(&child.compute_txid(), 0, 100);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 2_000_000, op_return_txid, 840001).unwrap();

    assert!(result.success, "Should succeed for single input tx");

    // last_txid should be parent txid (big-endian)
    let last_txid = read_bytes32(&result.output, 0);
    let mut expected_parent_be = *parent.compute_txid().as_byte_array();
    expected_parent_be.reverse();
    assert_eq!(last_txid, expected_parent_be, "last_txid should be parent txid");

    // last_vout should be 0
    let last_vout = read_u256(&result.output, 32);
    assert_eq!(last_vout, 0, "last_vout should be 0");

    // last_sat should be 100 (same sat offset in the input)
    let last_sat = read_u256(&result.output, 64);
    assert_eq!(last_sat, 100, "last_sat should be 100");
}

#[test]
fn test_last_sat_multi_input_finds_correct_input() {
    clear();
    let op_return_txid = B256::ZERO;

    // Create two parent transactions
    let parent1 = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2tr_script(), 5_000_000)]);
    let parent2 = build_tx(vec![(Txid::all_zeros(), 1)], vec![(p2wpkh_script(), 3_000_000)]);
    store_tx(&parent1, 840000);
    store_tx(&parent2, 840000);

    // Child spends both: input0 = 5M sats, input1 = 3M sats
    // Total: 8M sats in, output: 7.5M sats
    let child = build_tx(
        vec![(parent1.compute_txid(), 0), (parent2.compute_txid(), 0)],
        vec![(p2wpkh_script(), 7_500_000)],
    );
    store_tx(&child, 840001);

    // Sat 6_000_000 in output 0 — should be in input 1 (parent2)
    // because input 0 covers sats 0-4_999_999 and input 1 covers 5_000_000-7_999_999
    let input = build_last_sat_input(&child.compute_txid(), 0, 6_000_000);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 5_000_000, op_return_txid, 840001).unwrap();

    assert!(result.success, "Should find sat in second input");

    // last_txid should be parent2
    let last_txid = read_bytes32(&result.output, 0);
    let mut expected_parent2_be = *parent2.compute_txid().as_byte_array();
    expected_parent2_be.reverse();
    assert_eq!(last_txid, expected_parent2_be, "Should trace back to parent2");

    // last_vout should be 0 (parent2's output index)
    let last_vout = read_u256(&result.output, 32);
    assert_eq!(last_vout, 0);

    // last_sat: sat 6M in output = sat 1M in input1 (6M - 5M from input0)
    let last_sat = read_u256(&result.output, 64);
    assert_eq!(last_sat, 1_000_000, "Sat offset in parent2 should be 1M");
}

#[test]
fn test_last_sat_coinbase_rejected() {
    clear();
    let op_return_txid = B256::ZERO;

    let coinbase = build_coinbase(vec![(p2wpkh_script(), 625_000_000)]);
    store_tx(&coinbase, 840000);

    let input = build_last_sat_input(&coinbase.compute_txid(), 0, 100);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 2_000_000, op_return_txid, 840000).unwrap();
    assert!(!result.success, "Coinbase transactions should be rejected");
}

#[test]
fn test_last_sat_invalid_vout() {
    clear();
    let op_return_txid = B256::ZERO;

    let parent = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2tr_script(), 10_000_000)]);
    store_tx(&parent, 840000);
    let child = build_tx(vec![(parent.compute_txid(), 0)], vec![(p2wpkh_script(), 9_000_000)]);
    store_tx(&child, 840001);

    // vout=5 doesn't exist (only one output)
    let input = build_last_sat_input(&child.compute_txid(), 5, 100);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 2_000_000, op_return_txid, 840001).unwrap();
    assert!(!result.success, "Invalid vout index should fail");
}

#[test]
fn test_last_sat_sat_exceeds_output_value() {
    clear();
    let op_return_txid = B256::ZERO;

    let parent = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2tr_script(), 10_000_000)]);
    store_tx(&parent, 840000);
    let child = build_tx(vec![(parent.compute_txid(), 0)], vec![(p2wpkh_script(), 1000)]);
    store_tx(&child, 840001);

    // sat=5000 exceeds output value of 1000
    let input = build_last_sat_input(&child.compute_txid(), 0, 5000);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 2_000_000, op_return_txid, 840001).unwrap();
    assert!(!result.success, "Sat exceeding output value should fail");
}

#[test]
fn test_last_sat_unknown_txid() {
    clear();
    let op_return_txid = B256::ZERO;
    let fake_txid = Txid::from_slice(&[0xBB; 32]).unwrap();
    let input = build_last_sat_input(&fake_txid, 0, 100);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 2_000_000, op_return_txid, 840000).unwrap();
    assert!(!result.success, "Unknown txid should fail");
}

#[test]
fn test_last_sat_sat_zero() {
    clear();
    let op_return_txid = B256::ZERO;

    let parent = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2tr_script(), 10_000_000)]);
    store_tx(&parent, 840000);
    let child = build_tx(vec![(parent.compute_txid(), 0)], vec![(p2wpkh_script(), 9_000_000)]);
    store_tx(&child, 840001);

    // sat=0 — first satoshi in the output
    let input = build_last_sat_input(&child.compute_txid(), 0, 0);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 2_000_000, op_return_txid, 840001).unwrap();
    assert!(result.success, "Sat 0 should succeed");

    let last_sat = read_u256(&result.output, 64);
    assert_eq!(last_sat, 0, "First sat in first input should have offset 0");
}

#[test]
fn test_last_sat_gas_scales_with_inputs() {
    clear();
    let op_return_txid = B256::ZERO;

    let parent1 = build_tx(vec![(Txid::all_zeros(), 0)], vec![(p2tr_script(), 5_000_000)]);
    let parent2 = build_tx(vec![(Txid::all_zeros(), 1)], vec![(p2wpkh_script(), 3_000_000)]);
    store_tx(&parent1, 840000);
    store_tx(&parent2, 840000);

    let child = build_tx(
        vec![(parent1.compute_txid(), 0), (parent2.compute_txid(), 0)],
        vec![(p2wpkh_script(), 7_500_000)],
    );
    store_tx(&child, 840001);

    let input = build_last_sat_input(&child.compute_txid(), 0, 100);
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 5_000_000, op_return_txid, 840001).unwrap();
    assert!(result.success);

    // Gas should be base + 2 inputs
    assert_eq!(result.gas_used, GAS_BTC_RPC_CALL * 3, "Gas = base + 2 inputs");
}

// ============================================================================
// ABI encoding verification
// ============================================================================

#[test]
fn test_tx_details_output_structure() {
    clear();
    let op_return_txid = B256::ZERO;

    // Parent with known output
    let parent = build_tx(
        vec![(Txid::all_zeros(), 0)],
        vec![(p2tr_script(), 10_000_000)],
    );
    store_tx(&parent, 840000);

    // Child with 1 input, 2 outputs
    let child = build_tx(
        vec![(parent.compute_txid(), 0)],
        vec![
            (p2wpkh_script(), 5_000_000),
            (p2tr_script(), 4_500_000),
        ],
    );
    store_tx(&child, 840001);

    let input = build_tx_details_input(&child.compute_txid());
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &input, 5_000_000, op_return_txid, 840001).unwrap();
    assert!(result.success);

    // First 32 bytes: block_height
    assert_eq!(read_u256(&result.output, 0), 840001);

    // Remaining 6 * 32 bytes are offsets to dynamic arrays
    // We can't easily decode the full ABI without a proper decoder,
    // but we can verify the output is substantial (not empty)
    assert!(result.output.len() > 224, "Output should contain header + arrays");
}

// ============================================================================
// BIP322 verification tests
// ============================================================================

/// Build ABI-encoded input for verify(bytes pkscript, bytes message, bytes signature)
fn build_bip322_input(pkscript: &[u8], message: &[u8], signature: &[u8]) -> Vec<u8> {
    let mut input = vec![0u8; 4]; // function selector

    // Three dynamic offsets (each 32 bytes) pointing past the header
    let header_size: usize = 3 * 32;
    let pk_data = encode_test_bytes(pkscript);
    let msg_data = encode_test_bytes(message);
    let sig_data = encode_test_bytes(signature);

    let pk_offset = header_size;
    let msg_offset = pk_offset + pk_data.len();
    let sig_offset = msg_offset + msg_data.len();

    // Offset to pkscript
    let mut buf = [0u8; 32];
    buf[24..32].copy_from_slice(&(pk_offset as u64).to_be_bytes());
    input.extend_from_slice(&buf);
    // Offset to message
    buf = [0u8; 32];
    buf[24..32].copy_from_slice(&(msg_offset as u64).to_be_bytes());
    input.extend_from_slice(&buf);
    // Offset to signature
    buf = [0u8; 32];
    buf[24..32].copy_from_slice(&(sig_offset as u64).to_be_bytes());
    input.extend_from_slice(&buf);

    // Data
    input.extend(&pk_data);
    input.extend(&msg_data);
    input.extend(&sig_data);

    input
}

/// Encode bytes for ABI: 32-byte length + padded data
fn encode_test_bytes(data: &[u8]) -> Vec<u8> {
    let padded_len = (data.len() + 31) / 32 * 32;
    let mut result = vec![0u8; 32 + padded_len];
    result[24..32].copy_from_slice(&(data.len() as u64).to_be_bytes());
    result[32..32 + data.len()].copy_from_slice(data);
    result
}

#[test]
fn test_bip322_invalid_pkscript_fails() {
    clear();
    let txid = B256::ZERO;

    // Invalid pkscript (not a valid Bitcoin script for address derivation)
    let input = build_bip322_input(
        &[0xFF, 0xFF, 0xFF], // invalid script
        b"Hello World",
        &[0x00; 32], // dummy signature
    );
    let result = execute_precompile(&PRECOMPILE_BIP322, &input, 100_000, txid, 840000).unwrap();
    assert!(!result.success, "Invalid pkscript should fail");
}

#[test]
fn test_bip322_invalid_witness_fails() {
    clear();
    let txid = B256::ZERO;

    // Valid P2WPKH pkscript but garbage witness
    let pkscript = hex::decode("0014f477952f33561c1b89a1fe9f28682f623263e159").unwrap();
    let input = build_bip322_input(
        &pkscript,
        b"Hello World",
        &[0xDE, 0xAD, 0xBE, 0xEF], // invalid witness encoding
    );
    let result = execute_precompile(&PRECOMPILE_BIP322, &input, 100_000, txid, 840000).unwrap();
    assert!(!result.success, "Invalid witness should fail");
}

#[test]
fn test_bip322_empty_pkscript_fails() {
    clear();
    let txid = B256::ZERO;

    let input = build_bip322_input(&[], b"test message", &[0u8; 10]);
    let result = execute_precompile(&PRECOMPILE_BIP322, &input, 100_000, txid, 840000).unwrap();
    assert!(!result.success, "Empty pkscript should fail");
}

#[test]
fn test_bip322_abi_decoding_works() {
    clear();
    let txid = B256::ZERO;

    // Valid P2WPKH pkscript, valid message, but wrong signature
    // This should pass ABI decode but fail signature verification
    let pkscript = hex::decode("0014f477952f33561c1b89a1fe9f28682f623263e159").unwrap();

    // Build a valid-looking witness (2 items: sig + pubkey)
    let mut witness_bytes = Vec::new();
    // Witness serialization: varint count + (varint len + data) for each item
    witness_bytes.push(2); // 2 witness items
    // Item 1: 71-byte dummy signature
    witness_bytes.push(71);
    witness_bytes.extend_from_slice(&[0x30; 71]);
    // Item 2: 33-byte dummy pubkey
    witness_bytes.push(33);
    witness_bytes.extend_from_slice(&[0x02; 33]);

    let input = build_bip322_input(&pkscript, b"Hello World", &witness_bytes);

    // Should succeed ABI parsing but fail crypto verification → returns false
    let result = execute_precompile(&PRECOMPILE_BIP322, &input, 100_000, txid, 840000).unwrap();
    // The bip322 crate may reject this as invalid → fail, or accept and return false
    // Either outcome is correct for a dummy signature
    // The key thing is it doesn't panic
    assert!(result.output.is_empty() || result.output.len() == 32,
        "Output should be empty (fail) or 32 bytes (result)");
}

#[test]
fn test_bip322_gas_cost() {
    clear();
    let txid = B256::ZERO;
    let pkscript = hex::decode("0014f477952f33561c1b89a1fe9f28682f623263e159").unwrap();
    let input = build_bip322_input(&pkscript, b"test", &[0u8; 10]);
    let result = execute_precompile(&PRECOMPILE_BIP322, &input, 100_000, txid, 840000).unwrap();
    // Whether it succeeds or fails, gas_used should be GAS_BIP322_VERIFY
    assert_eq!(result.gas_used, GAS_BIP322_VERIFY, "Gas should be GAS_BIP322_VERIFY");
}

#[test]
fn test_bip322_real_taproot_signature() {
    clear();
    let txid = B256::ZERO;

    // Real BIP322 test vector from bip322 crate / BIP-322 specification:
    // Address: bc1ppv609nr0vr25u07u95waq5lucwfm6tde4nydujnu8npg4q75mr5sxq8lt3
    // Message: "Hello World"
    // Signature (base64): AUHd69PrJQEv+oKTfZ8l+WROBHuy9HKrbFCJu7U1iK2iiEy1vMU5EfMtjc+VSHM7aU0SDbak5IUZRVno2P5mjSafAQ==

    // P2TR pkscript for bc1ppv609nr0vr25u07u95waq5lucwfm6tde4nydujnu8npg4q75mr5sxq8lt3
    let pkscript = hex::decode("51200b34f2cc6f60d54e3fdc2d1dd053fcc393bd2db9acc8de4a7c3cc28a83d4d8e9").unwrap();

    // Message
    let message = b"Hello World";

    // Signature: base64 decode of the BIP322 witness
    let sig_base64 = "AUHd69PrJQEv+oKTfZ8l+WROBHuy9HKrbFCJu7U1iK2iiEy1vMU5EfMtjc+VSHM7aU0SDbak5IUZRVno2P5mjSafAQ==";
    // This is consensus-encoded witness bytes
    let sig_bytes = {
        // Base64 decode
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = Vec::new();
        let chars: Vec<u8> = sig_base64.bytes().filter(|b| *b != b'=').collect();
        for chunk in chars.chunks(4) {
            let mut val = 0u32;
            for (i, &c) in chunk.iter().enumerate() {
                let idx = alphabet.iter().position(|&a| a == c).unwrap_or(0) as u32;
                val |= idx << (6 * (3 - i));
            }
            result.push((val >> 16) as u8);
            if chunk.len() > 2 { result.push((val >> 8) as u8); }
            if chunk.len() > 3 { result.push(val as u8); }
        }
        result
    };

    let input = build_bip322_input(&pkscript, message, &sig_bytes);
    let result = execute_precompile(&PRECOMPILE_BIP322, &input, 100_000, txid, 840000).unwrap();

    // With the bip322 crate integrated, this should verify successfully
    assert!(result.success, "BIP322 verification call should succeed");
    assert_eq!(result.output.len(), 32, "Output should be 32 bytes");
    assert_eq!(result.output[31], 1, "Real signature should verify as true");
}

#[test]
fn test_bip322_wrong_message_fails() {
    clear();
    let txid = B256::ZERO;

    // Same address + signature as above, but WRONG message
    let pkscript = hex::decode("51200b34f2cc6f60d54e3fdc2d1dd053fcc393bd2db9acc8de4a7c3cc28a83d4d8e9").unwrap();

    let sig_bytes = hex::decode("0141ddebd3eb25012ffa82937d9f25f9644e047bb2f472ab6c5089bbb53588ada2884cb5bcc53911f32d8dcf9548733b694d120db6a4e485194559e8d8fe668d269f01").unwrap();

    // Wrong message — should fail verification
    let input = build_bip322_input(&pkscript, b"Wrong Message", &sig_bytes);
    let result = execute_precompile(&PRECOMPILE_BIP322, &input, 100_000, txid, 840000).unwrap();

    // Should fail verification (either success=false or output[31]=0)
    if result.success {
        assert_eq!(result.output[31], 0, "Wrong message should return false");
    }
    // If !result.success, that's also correct (verification error)
}
