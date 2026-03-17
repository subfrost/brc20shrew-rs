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

use bitcoin::blockdata::opcodes;
use bitcoin::blockdata::script::Builder;
use bitcoin::key::UntweakedPublicKey;
use bitcoin::taproot::TaprootBuilder;
use bitcoin::ScriptBuf;
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
///
/// Builds a P2TR output script with a CSV timelock wrapping the given pkscript.
fn get_locked_pkscript(input: &[u8], gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_LOCKED_PKSCRIPT {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    let fail = || PrecompileResult { success: false, gas_used: GAS_LOCKED_PKSCRIPT, output: vec![] };

    // Need at least 4 (selector) + 32 (offset) + 32 (lock_block_count) = 68 bytes
    if input.len() < 68 {
        return fail();
    }

    // Skip 4-byte function selector
    let data = &input[4..];

    // Decode pkscript offset (first 32 bytes of data)
    let pkscript_offset = {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&data[0..32]);
        // offset is in bytes from start of data area
        let offset = u256_to_usize(buf);
        match offset {
            Some(o) => o,
            None => return fail(),
        }
    };

    // Decode lock_block_count (second 32 bytes of data), use as u64
    let lock_block_count = {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&data[32..64]);
        // Take low 8 bytes as u64
        let val = u64::from_be_bytes(buf[24..32].try_into().unwrap());
        val
    };

    // Validate lock_block_count: 1 <= lock_block_count <= 65535
    if lock_block_count < 1 || lock_block_count > 65535 {
        return fail();
    }

    // Decode pkscript bytes from the dynamic offset
    if data.len() < pkscript_offset + 32 {
        return fail();
    }
    let pkscript_len = {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&data[pkscript_offset..pkscript_offset + 32]);
        match u256_to_usize(buf) {
            Some(l) => l,
            None => return fail(),
        }
    };
    if data.len() < pkscript_offset + 32 + pkscript_len {
        return fail();
    }
    let pkscript = &data[pkscript_offset + 32..pkscript_offset + 32 + pkscript_len];

    // Build the lock script:
    // <lock_block_count> OP_CSV OP_DROP <pkscript_bytes> OP_CHECKSIG
    let lock_script = {
        let builder = Builder::new()
            .push_int(lock_block_count as i64)
            .push_opcode(opcodes::all::OP_CSV)
            .push_opcode(opcodes::all::OP_DROP);
        let mut script_bytes = builder.into_script().into_bytes();
        script_bytes.extend_from_slice(pkscript);
        script_bytes.push(opcodes::all::OP_CHECKSIG.to_u8());
        ScriptBuf::from(script_bytes)
    };

    // Unspendable internal key (nothing-up-my-sleeve point)
    let internal_key_bytes: [u8; 32] = [
        0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54,
        0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9, 0x7a, 0x5e,
        0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5,
        0x47, 0xbf, 0xee, 0x9a, 0xce, 0x80, 0x3a, 0xc0,
    ];
    let internal_key = match UntweakedPublicKey::from_slice(&internal_key_bytes) {
        Ok(k) => k,
        Err(_) => return fail(),
    };

    // Build taproot with one leaf at depth 0
    let secp = bitcoin::secp256k1::Secp256k1::verification_only();
    let taproot_builder = TaprootBuilder::new()
        .add_leaf(0, lock_script)
        .expect("single leaf at depth 0 should not fail");
    let taproot_spend_info = match taproot_builder.finalize(&secp, internal_key) {
        Ok(info) => info,
        Err(_) => return fail(),
    };

    // Get the P2TR script pubkey (OP_1 <32-byte-tweaked-key>)
    let output_key = taproot_spend_info.output_key();
    let p2tr_script = ScriptBuf::new_v1_p2tr_tweaked(output_key);
    let result_bytes = p2tr_script.into_bytes();

    // ABI-encode as bytes: offset (32) + length (32) + data (padded to 32)
    let padded_len = (result_bytes.len() + 31) / 32 * 32;
    let mut output = vec![0u8; 64 + padded_len];
    // Offset to data (always 0x20 = 32)
    output[31] = 0x20;
    // Length of data
    let len_bytes = (result_bytes.len() as u64).to_be_bytes();
    output[56..64].copy_from_slice(&len_bytes);
    // Data
    output[64..64 + result_bytes.len()].copy_from_slice(&result_bytes);

    PrecompileResult { success: true, gas_used: GAS_LOCKED_PKSCRIPT, output }
}

/// Convert a big-endian 256-bit value to usize, returning None if it overflows.
fn u256_to_usize(bytes: [u8; 32]) -> Option<usize> {
    // Check that the high bytes are all zero
    for &b in &bytes[..24] {
        if b != 0 {
            return None;
        }
    }
    let val = u64::from_be_bytes(bytes[24..32].try_into().unwrap());
    usize::try_from(val).ok()
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
