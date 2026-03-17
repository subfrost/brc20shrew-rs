use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::precompiles::*;
use revm::primitives::{Address, B256};

// ---------------------------------------------------------------------------
// Address recognition tests
// ---------------------------------------------------------------------------

#[test]
fn test_precompile_addresses_recognized() {
    assert!(is_precompile(&PRECOMPILE_BIP322), "BIP322 should be recognized");
    assert!(is_precompile(&PRECOMPILE_TX_DETAILS), "TX_DETAILS should be recognized");
    assert!(is_precompile(&PRECOMPILE_LAST_SAT_LOC), "LAST_SAT_LOC should be recognized");
    assert!(is_precompile(&PRECOMPILE_LOCKED_PKSCRIPT), "LOCKED_PKSCRIPT should be recognized");
    assert!(is_precompile(&PRECOMPILE_OP_RETURN_TXID), "OP_RETURN_TXID should be recognized");
}

#[test]
fn test_non_precompile_address() {
    assert!(!is_precompile(&Address::ZERO), "Address::ZERO should not be a precompile");

    // Also test an address that is close but not a precompile
    let mut bytes = [0u8; 20];
    bytes[19] = 0xF0; // not in the set 0xFA-0xFE
    let addr = Address::new(bytes);
    assert!(!is_precompile(&addr), "0xF0 should not be a precompile");
}

#[test]
fn test_precompile_list() {
    let addresses = precompile_addresses();
    assert_eq!(addresses.len(), 5, "Should have exactly 5 precompile addresses");
    assert!(addresses.contains(&PRECOMPILE_BIP322));
    assert!(addresses.contains(&PRECOMPILE_TX_DETAILS));
    assert!(addresses.contains(&PRECOMPILE_LAST_SAT_LOC));
    assert!(addresses.contains(&PRECOMPILE_LOCKED_PKSCRIPT));
    assert!(addresses.contains(&PRECOMPILE_OP_RETURN_TXID));
}

// ---------------------------------------------------------------------------
// OP_RETURN TXID precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_op_return_txid_precompile() {
    let txid = B256::from([0xAB; 32]);
    let result = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], 1000, txid, 840000);
    assert!(result.is_some(), "Should dispatch to OP_RETURN_TXID precompile");
    let result = result.unwrap();
    assert!(result.success, "Should succeed with sufficient gas");
    assert_eq!(result.output, txid.to_vec(), "Output should be the B256 txid");
}

#[test]
fn test_op_return_txid_gas_cost() {
    let txid = B256::from([0x11; 32]);
    let result = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], 1000, txid, 840000).unwrap();
    assert_eq!(result.gas_used, GAS_OP_RETURN_TXID, "Gas used should be GAS_OP_RETURN_TXID");
}

#[test]
fn test_op_return_txid_insufficient_gas() {
    let txid = B256::from([0x22; 32]);
    // GAS_OP_RETURN_TXID is 40, pass only 10
    let result = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], 10, txid, 840000).unwrap();
    assert!(!result.success, "Should fail with insufficient gas");
    assert!(result.output.is_empty(), "Output should be empty on gas failure");
}

// ---------------------------------------------------------------------------
// BIP322 verify precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_bip322_verify_returns_false() {
    let txid = B256::ZERO;
    // Need at least 100 bytes for valid ABI input
    let result = execute_precompile(&PRECOMPILE_BIP322, &[0u8; 100], 100_000, txid, 840000).unwrap();
    assert!(result.success, "BIP322 verify should succeed (return the result)");
    assert_eq!(result.output.len(), 32, "Output should be 32 bytes");
    assert_eq!(result.output[31], 0, "Stub should return false (0)");
}

#[test]
fn test_bip322_verify_short_input() {
    let txid = B256::ZERO;
    // Input too short (< 100 bytes)
    let result = execute_precompile(&PRECOMPILE_BIP322, &[0u8; 64], 100_000, txid, 840000).unwrap();
    assert!(!result.success, "Should fail with input < 100 bytes");
}

#[test]
fn test_bip322_verify_insufficient_gas() {
    let txid = B256::ZERO;
    let result = execute_precompile(&PRECOMPILE_BIP322, &[0u8; 100], 5000, txid, 840000).unwrap();
    assert!(!result.success, "Should fail with insufficient gas");
    assert!(result.output.is_empty(), "Output should be empty on gas failure");
}

#[test]
fn test_bip322_verify_oversized_input() {
    let txid = B256::ZERO;
    // Input exceeds 32KB limit
    let result = execute_precompile(&PRECOMPILE_BIP322, &[0u8; 33000], 100_000, txid, 840000).unwrap();
    assert!(!result.success, "Should fail with input > 32KB");
}

// ---------------------------------------------------------------------------
// BTC TX Details precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_btc_tx_details_short_input() {
    let txid = B256::ZERO;
    // Input shorter than 36 bytes should fail
    let short_input = vec![0u8; 20];
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &short_input, 100_000, txid, 840000).unwrap();
    assert!(!result.success, "Should fail with input < 36 bytes");
}

#[test]
fn test_btc_tx_details_valid_input_no_tx_found() {
    let txid = B256::ZERO;
    // Input of 36+ bytes with a zero txid — won't be found in test DB
    let valid_input = vec![0u8; 36];
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &valid_input, 500_000, txid, 840000).unwrap();
    assert!(!result.success, "Should fail when tx not found in indexed data");
}

#[test]
fn test_btc_tx_details_insufficient_gas() {
    let txid = B256::ZERO;
    let valid_input = vec![0u8; 36];
    // GAS_BTC_RPC_CALL is 400_000, pass only 100
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &valid_input, 100, txid, 840000).unwrap();
    assert!(!result.success, "Should fail with insufficient gas");
}

// ---------------------------------------------------------------------------
// Last sat location precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_last_sat_location_short_input() {
    let txid = B256::ZERO;
    // Empty input should fail (needs 100 bytes minimum)
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &[], 500_000, txid, 840000).unwrap();
    assert!(!result.success, "Last sat location with empty input should fail");
}

#[test]
fn test_last_sat_location_no_tx_found() {
    let txid = B256::ZERO;
    // Valid-length input but zero txid won't be found
    let input = vec![0u8; 100];
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &input, 500_000, txid, 840000).unwrap();
    assert!(!result.success, "Last sat location with unknown txid should fail");
}

// ---------------------------------------------------------------------------
// Locked pkscript precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_locked_pkscript_empty_input_fails() {
    let txid = B256::ZERO;
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &[], 100_000, txid, 840000).unwrap();
    assert!(!result.success, "Locked pkscript with empty input should fail (insufficient ABI data)");
}

// ---------------------------------------------------------------------------
// Locked pkscript precompile — valid ABI input tests
// ---------------------------------------------------------------------------

/// Build a valid ABI-encoded input for getLockedPkscript(bytes pkscript, uint256 lock_block_count)
fn build_locked_pkscript_input(pkscript: &[u8], lock_block_count: u64) -> Vec<u8> {
    let mut input = Vec::new();
    // 4-byte function selector (arbitrary, precompile ignores it)
    input.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    // offset to pkscript data (0x40 = 64 bytes from start of data)
    let mut offset = [0u8; 32];
    offset[31] = 0x40;
    input.extend_from_slice(&offset);
    // lock_block_count as uint256 (big-endian)
    let mut lock = [0u8; 32];
    lock[24..32].copy_from_slice(&lock_block_count.to_be_bytes());
    input.extend_from_slice(&lock);
    // pkscript length as uint256
    let mut len = [0u8; 32];
    len[24..32].copy_from_slice(&(pkscript.len() as u64).to_be_bytes());
    input.extend_from_slice(&len);
    // pkscript data (padded to 32 bytes)
    let padded_len = (pkscript.len() + 31) / 32 * 32;
    let mut padded = vec![0u8; padded_len];
    padded[..pkscript.len()].copy_from_slice(pkscript);
    input.extend_from_slice(&padded);
    input
}

#[test]
fn test_locked_pkscript_valid_small_lock() {
    let txid = B256::ZERO;
    // Simple 33-byte compressed pubkey as pkscript
    let pkscript = vec![0x02; 33];
    let input = build_locked_pkscript_input(&pkscript, 6);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100_000, txid, 840000).unwrap();
    assert!(result.success, "Should succeed with valid input and lock=6");
    assert_eq!(result.gas_used, GAS_LOCKED_PKSCRIPT);
    // Output is ABI-encoded bytes: offset(32) + length(32) + data
    assert!(result.output.len() >= 64, "Output should contain ABI header");
    // P2TR script should be 34 bytes (OP_1 <32-byte-key>)
    let data_len = u64::from_be_bytes(result.output[56..64].try_into().unwrap()) as usize;
    assert_eq!(data_len, 34, "P2TR script should be 34 bytes");
    // First byte should be OP_1 (0x51) for P2TR
    assert_eq!(result.output[64], 0x51, "P2TR script should start with OP_1");
    // Second byte should be 0x20 (push 32 bytes)
    assert_eq!(result.output[65], 0x20, "P2TR script should push 32-byte key");
}

#[test]
fn test_locked_pkscript_lock_1() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input = build_locked_pkscript_input(&pkscript, 1);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100_000, txid, 840000).unwrap();
    assert!(result.success, "lock_block_count=1 should succeed");
}

#[test]
fn test_locked_pkscript_lock_16() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input = build_locked_pkscript_input(&pkscript, 16);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100_000, txid, 840000).unwrap();
    assert!(result.success, "lock_block_count=16 should succeed (OP_16)");
}

#[test]
fn test_locked_pkscript_lock_17() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input = build_locked_pkscript_input(&pkscript, 17);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100_000, txid, 840000).unwrap();
    assert!(result.success, "lock_block_count=17 should succeed (first value beyond OP_16)");
}

#[test]
fn test_locked_pkscript_lock_max() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input = build_locked_pkscript_input(&pkscript, 65535);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100_000, txid, 840000).unwrap();
    assert!(result.success, "lock_block_count=65535 should succeed (max valid)");
}

#[test]
fn test_locked_pkscript_lock_zero_rejected() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input = build_locked_pkscript_input(&pkscript, 0);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100_000, txid, 840000).unwrap();
    assert!(!result.success, "lock_block_count=0 should fail");
}

#[test]
fn test_locked_pkscript_lock_too_large_rejected() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input = build_locked_pkscript_input(&pkscript, 65536);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100_000, txid, 840000).unwrap();
    assert!(!result.success, "lock_block_count=65536 should fail (> 65535)");
}

#[test]
fn test_locked_pkscript_insufficient_gas() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input = build_locked_pkscript_input(&pkscript, 6);
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input, 100, txid, 840000).unwrap();
    assert!(!result.success, "Should fail with insufficient gas");
}

#[test]
fn test_locked_pkscript_different_locks_produce_different_outputs() {
    let txid = B256::ZERO;
    let pkscript = vec![0x03; 33];
    let input6 = build_locked_pkscript_input(&pkscript, 6);
    let input100 = build_locked_pkscript_input(&pkscript, 100);
    let result6 = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input6, 100_000, txid, 840000).unwrap();
    let result100 = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input100, 100_000, txid, 840000).unwrap();
    assert!(result6.success && result100.success);
    assert_ne!(result6.output, result100.output,
        "Different lock counts should produce different P2TR outputs");
}

#[test]
fn test_locked_pkscript_different_pkscripts_produce_different_outputs() {
    let txid = B256::ZERO;
    let pk1 = vec![0x02; 33];
    let pk2 = vec![0x03; 33];
    let input1 = build_locked_pkscript_input(&pk1, 6);
    let input2 = build_locked_pkscript_input(&pk2, 6);
    let result1 = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input1, 100_000, txid, 840000).unwrap();
    let result2 = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &input2, 100_000, txid, 840000).unwrap();
    assert!(result1.success && result2.success);
    assert_ne!(result1.output, result2.output,
        "Different pkscripts should produce different P2TR outputs");
}

// ---------------------------------------------------------------------------
// Dispatch tests
// ---------------------------------------------------------------------------

#[test]
fn test_execute_precompile_dispatch() {
    let txid = B256::from([0x99; 32]);
    let gas = 100_000u64;

    // Each precompile address should dispatch to Some(...)
    let r1 = execute_precompile(&PRECOMPILE_BIP322, &[], gas, txid, 840000);
    assert!(r1.is_some(), "BIP322 should dispatch");

    let r2 = execute_precompile(&PRECOMPILE_TX_DETAILS, &[0u8; 36], gas, txid, 840000);
    assert!(r2.is_some(), "TX_DETAILS should dispatch");

    let r3 = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &[], gas, txid, 840000);
    assert!(r3.is_some(), "LAST_SAT_LOC should dispatch");

    let r4 = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &[], gas, txid, 840000);
    assert!(r4.is_some(), "LOCKED_PKSCRIPT should dispatch");

    let r5 = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], gas, txid, 840000);
    assert!(r5.is_some(), "OP_RETURN_TXID should dispatch");
}

#[test]
fn test_execute_precompile_unknown_address() {
    let txid = B256::ZERO;
    let unknown = Address::ZERO;
    let result = execute_precompile(&unknown, &[], 100_000, txid, 840000);
    assert!(result.is_none(), "Unknown address should return None");
}
