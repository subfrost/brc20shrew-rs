//! Custom precompiles for the BRC20-prog EVM.
//!
//! These are adapted from the upstream brc20-prog module to work with
//! the metashrew indexed state instead of RPC calls.
//!
//! Precompile addresses:
//!   0xFE - BIP322 signature verification
//!   0xFD - BTC transaction details
//!   0xFC - Last satoshi location
//!   0xFB - Get locked pkscript
//!   0xFA - Get OP_RETURN transaction ID

use revm::primitives::{Address, B256};

/// Precompile address for BIP322 verify
pub const PRECOMPILE_BIP322: Address = address_from_low_byte(0xFE);
/// Precompile address for BTC transaction details
pub const PRECOMPILE_TX_DETAILS: Address = address_from_low_byte(0xFD);
/// Precompile address for last sat location
pub const PRECOMPILE_LAST_SAT_LOC: Address = address_from_low_byte(0xFC);
/// Precompile address for get locked pkscript
pub const PRECOMPILE_LOCKED_PKSCRIPT: Address = address_from_low_byte(0xFB);
/// Precompile address for OP_RETURN tx ID
pub const PRECOMPILE_OP_RETURN_TXID: Address = address_from_low_byte(0xFA);

/// Gas costs
pub const GAS_BIP322_VERIFY: u64 = 10_000;
pub const GAS_BTC_RPC_CALL: u64 = 5_000;
pub const GAS_LOCKED_PKSCRIPT: u64 = 3_000;
pub const GAS_OP_RETURN_TXID: u64 = 100;

const fn address_from_low_byte(byte: u8) -> Address {
    let mut bytes = [0u8; 20];
    bytes[19] = byte;
    Address::new(bytes)
}

/// Check if an address is a custom precompile
pub fn is_precompile(address: &Address) -> bool {
    *address == PRECOMPILE_BIP322
        || *address == PRECOMPILE_TX_DETAILS
        || *address == PRECOMPILE_LAST_SAT_LOC
        || *address == PRECOMPILE_LOCKED_PKSCRIPT
        || *address == PRECOMPILE_OP_RETURN_TXID
}

/// All custom precompile addresses
pub fn precompile_addresses() -> Vec<Address> {
    vec![
        PRECOMPILE_BIP322,
        PRECOMPILE_TX_DETAILS,
        PRECOMPILE_LAST_SAT_LOC,
        PRECOMPILE_LOCKED_PKSCRIPT,
        PRECOMPILE_OP_RETURN_TXID,
    ]
}

/// Execute a custom precompile call.
///
/// Returns `Some((gas_used, output))` if the address is a custom precompile,
/// `None` otherwise.
pub fn execute_precompile(
    address: &Address,
    input: &[u8],
    gas_limit: u64,
    op_return_tx_id: B256,
) -> Option<PrecompileResult> {
    if *address == PRECOMPILE_BIP322 {
        Some(bip322_verify(input, gas_limit))
    } else if *address == PRECOMPILE_TX_DETAILS {
        Some(btc_tx_details(input, gas_limit))
    } else if *address == PRECOMPILE_LAST_SAT_LOC {
        Some(last_sat_location(input, gas_limit))
    } else if *address == PRECOMPILE_LOCKED_PKSCRIPT {
        Some(get_locked_pkscript(input, gas_limit))
    } else if *address == PRECOMPILE_OP_RETURN_TXID {
        Some(get_op_return_txid(op_return_tx_id, gas_limit))
    } else {
        None
    }
}

pub struct PrecompileResult {
    pub success: bool,
    pub gas_used: u64,
    pub output: Vec<u8>,
}

/// BIP322 signature verification precompile (0xFE).
///
/// In the metashrew environment, we perform pure crypto verification
/// without needing RPC calls.
///
/// Input ABI: verify(bytes pkscript, bytes message, bytes signature) -> bool
fn bip322_verify(_input: &[u8], gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_BIP322_VERIFY {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // For now, return false (not verified) - full BIP322 verification requires
    // secp256k1 and script evaluation which will be added when bip322 crate is integrated
    let mut output = vec![0u8; 32];
    output[31] = 0; // false

    PrecompileResult { success: true, gas_used: GAS_BIP322_VERIFY, output }
}

/// BTC transaction details precompile (0xFD).
///
/// Reads indexed transaction data from shrew-ord tables.
///
/// Input ABI: getTxDetails(bytes32 txid) -> (...)
fn btc_tx_details(input: &[u8], gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_BTC_RPC_CALL {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Extract txid from input (skip 4 bytes function selector, read 32 bytes)
    if input.len() < 36 {
        return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] };
    }

    let _txid = &input[4..36];

    // Read from indexed data - placeholder for now
    // In full implementation, would query shrew-ord tables for tx details
    let output = vec![0u8; 32]; // Empty response

    PrecompileResult { success: true, gas_used: GAS_BTC_RPC_CALL, output }
}

/// Last satoshi location precompile (0xFC).
///
/// Input ABI: getLastSatLocation(bytes32 txid, uint256 vout, uint256 sat) -> (...)
fn last_sat_location(_input: &[u8], gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_BTC_RPC_CALL {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Placeholder - requires sat tracking implementation
    let output = vec![0u8; 32];
    PrecompileResult { success: true, gas_used: GAS_BTC_RPC_CALL, output }
}

/// Get locked pkscript precompile (0xFB).
///
/// Input ABI: getLockedPkscript(bytes pkscript, uint256 lock_block_count) -> bytes
fn get_locked_pkscript(_input: &[u8], gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_LOCKED_PKSCRIPT {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Placeholder - returns empty pkscript
    let output = vec![0u8; 32];
    PrecompileResult { success: true, gas_used: GAS_LOCKED_PKSCRIPT, output }
}

/// Get OP_RETURN transaction ID precompile (0xFA).
///
/// Returns the current OP_RETURN transaction ID from context.
///
/// Input ABI: getTxId() -> bytes32
fn get_op_return_txid(op_return_tx_id: B256, gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_OP_RETURN_TXID {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    let output = op_return_tx_id.to_vec();
    PrecompileResult { success: true, gas_used: GAS_OP_RETURN_TXID, output }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precompile_addresses() {
        assert!(is_precompile(&PRECOMPILE_BIP322));
        assert!(is_precompile(&PRECOMPILE_TX_DETAILS));
        assert!(is_precompile(&PRECOMPILE_LAST_SAT_LOC));
        assert!(is_precompile(&PRECOMPILE_LOCKED_PKSCRIPT));
        assert!(is_precompile(&PRECOMPILE_OP_RETURN_TXID));
        assert!(!is_precompile(&Address::ZERO));
    }

    #[test]
    fn test_op_return_precompile() {
        let txid = B256::from([0xAB; 32]);
        let result = get_op_return_txid(txid, 1000);
        assert!(result.success);
        assert_eq!(result.output, txid.to_vec());
    }
}
