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
    let result = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], 1000, txid);
    assert!(result.is_some(), "Should dispatch to OP_RETURN_TXID precompile");
    let result = result.unwrap();
    assert!(result.success, "Should succeed with sufficient gas");
    assert_eq!(result.output, txid.to_vec(), "Output should be the B256 txid");
}

#[test]
fn test_op_return_txid_gas_cost() {
    let txid = B256::from([0x11; 32]);
    let result = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], 1000, txid).unwrap();
    assert_eq!(result.gas_used, GAS_OP_RETURN_TXID, "Gas used should be GAS_OP_RETURN_TXID");
}

#[test]
fn test_op_return_txid_insufficient_gas() {
    let txid = B256::from([0x22; 32]);
    // GAS_OP_RETURN_TXID is 100, pass only 50
    let result = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], 50, txid).unwrap();
    assert!(!result.success, "Should fail with insufficient gas");
    assert!(result.output.is_empty(), "Output should be empty on gas failure");
}

// ---------------------------------------------------------------------------
// BIP322 verify precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_bip322_verify_returns_false() {
    let txid = B256::ZERO;
    let result = execute_precompile(&PRECOMPILE_BIP322, &[0u8; 64], 100_000, txid).unwrap();
    assert!(result.success, "BIP322 verify should succeed (return the result)");
    assert_eq!(result.output.len(), 32, "Output should be 32 bytes");
    assert_eq!(result.output[31], 0, "Stub should return false (0)");
}

#[test]
fn test_bip322_verify_insufficient_gas() {
    let txid = B256::ZERO;
    // GAS_BIP322_VERIFY is 10_000, pass only 5000
    let result = execute_precompile(&PRECOMPILE_BIP322, &[0u8; 64], 5000, txid).unwrap();
    assert!(!result.success, "Should fail with insufficient gas");
    assert!(result.output.is_empty(), "Output should be empty on gas failure");
}

// ---------------------------------------------------------------------------
// BTC TX Details precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_btc_tx_details_short_input() {
    let txid = B256::ZERO;
    // Input shorter than 36 bytes should fail
    let short_input = vec![0u8; 20];
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &short_input, 100_000, txid).unwrap();
    assert!(!result.success, "Should fail with input < 36 bytes");
}

#[test]
fn test_btc_tx_details_valid_input() {
    let txid = B256::ZERO;
    // Input of 36+ bytes should succeed
    let valid_input = vec![0u8; 36];
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &valid_input, 100_000, txid).unwrap();
    assert!(result.success, "Should succeed with 36 byte input");
    assert_eq!(result.gas_used, GAS_BTC_RPC_CALL);
}

#[test]
fn test_btc_tx_details_insufficient_gas() {
    let txid = B256::ZERO;
    let valid_input = vec![0u8; 36];
    // GAS_BTC_RPC_CALL is 5000, pass only 100
    let result = execute_precompile(&PRECOMPILE_TX_DETAILS, &valid_input, 100, txid).unwrap();
    assert!(!result.success, "Should fail with insufficient gas");
}

// ---------------------------------------------------------------------------
// Last sat location precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_last_sat_location_placeholder() {
    let txid = B256::ZERO;
    let result = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &[], 100_000, txid).unwrap();
    assert!(result.success, "Last sat location stub should succeed");
    assert_eq!(result.gas_used, GAS_BTC_RPC_CALL);
    assert_eq!(result.output.len(), 32, "Output should be 32 bytes");
}

// ---------------------------------------------------------------------------
// Locked pkscript precompile tests
// ---------------------------------------------------------------------------

#[test]
fn test_locked_pkscript_placeholder() {
    let txid = B256::ZERO;
    let result = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &[], 100_000, txid).unwrap();
    assert!(result.success, "Locked pkscript stub should succeed");
    assert_eq!(result.gas_used, GAS_LOCKED_PKSCRIPT);
    assert_eq!(result.output.len(), 32, "Output should be 32 bytes");
}

// ---------------------------------------------------------------------------
// Dispatch tests
// ---------------------------------------------------------------------------

#[test]
fn test_execute_precompile_dispatch() {
    let txid = B256::from([0x99; 32]);
    let gas = 100_000u64;

    // Each precompile address should dispatch to Some(...)
    let r1 = execute_precompile(&PRECOMPILE_BIP322, &[], gas, txid);
    assert!(r1.is_some(), "BIP322 should dispatch");

    let r2 = execute_precompile(&PRECOMPILE_TX_DETAILS, &[0u8; 36], gas, txid);
    assert!(r2.is_some(), "TX_DETAILS should dispatch");

    let r3 = execute_precompile(&PRECOMPILE_LAST_SAT_LOC, &[], gas, txid);
    assert!(r3.is_some(), "LAST_SAT_LOC should dispatch");

    let r4 = execute_precompile(&PRECOMPILE_LOCKED_PKSCRIPT, &[], gas, txid);
    assert!(r4.is_some(), "LOCKED_PKSCRIPT should dispatch");

    let r5 = execute_precompile(&PRECOMPILE_OP_RETURN_TXID, &[], gas, txid);
    assert!(r5.is_some(), "OP_RETURN_TXID should dispatch");
}

#[test]
fn test_execute_precompile_unknown_address() {
    let txid = B256::ZERO;
    let unknown = Address::ZERO;
    let result = execute_precompile(&unknown, &[], 100_000, txid);
    assert!(result.is_none(), "Unknown address should return None");
}
