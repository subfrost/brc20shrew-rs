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
use bitcoin::consensus::deserialize;
use bitcoin::key::UntweakedPublicKey;
use bitcoin::taproot::TaprootBuilder;
use bitcoin::{ScriptBuf, Transaction};
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use revm::primitives::{Address, B256};
use shrew_ord::tables::{TXID_TO_RAW_TX, TXID_TO_BLOCK_HEIGHT};

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

/// Gas costs (aligned with canonical brc20-prog implementation)
pub const GAS_BIP322_VERIFY: u64 = 20_000;
pub const GAS_BTC_RPC_CALL: u64 = 400_000;
pub const GAS_LOCKED_PKSCRIPT: u64 = 20_000;
pub const GAS_OP_RETURN_TXID: u64 = 40;

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
/// Returns `Some(PrecompileResult)` if the address is a custom precompile,
/// `None` otherwise.
pub fn execute_precompile(
    address: &Address,
    input: &[u8],
    gas_limit: u64,
    op_return_tx_id: B256,
    current_height: u32,
) -> Option<PrecompileResult> {
    if *address == PRECOMPILE_BIP322 {
        Some(bip322_verify(input, gas_limit))
    } else if *address == PRECOMPILE_TX_DETAILS {
        Some(btc_tx_details(input, gas_limit, current_height))
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

// ============================================================================
// Helper: look up a transaction by txid bytes (32 bytes, internal byte order)
// ============================================================================

/// Look up a raw transaction from the TXID_TO_RAW_TX table.
/// Returns the deserialized Transaction and its block height.
fn lookup_tx(txid_bytes: &[u8]) -> Option<(Transaction, u32)> {
    let raw = TXID_TO_RAW_TX.select(&txid_bytes.to_vec()).get();
    if raw.is_empty() { return None; }
    let height_bytes = TXID_TO_BLOCK_HEIGHT.select(&txid_bytes.to_vec()).get();
    let height = if height_bytes.len() >= 4 {
        u32::from_le_bytes(height_bytes[..4].try_into().unwrap_or([0; 4]))
    } else {
        0
    };
    let tx: Transaction = deserialize(&raw).ok()?;
    Some((tx, height))
}

// ============================================================================
// Helper: ABI encoding utilities
// ============================================================================

/// Encode a u256 value (u64 in low bytes) into 32 bytes big-endian
fn encode_u256(val: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..32].copy_from_slice(&val.to_be_bytes());
    buf
}

/// Encode a bytes32 value
fn encode_bytes32(data: &[u8; 32]) -> [u8; 32] {
    *data
}

/// Encode dynamic bytes with offset/length/data pattern
fn encode_dynamic_bytes(data: &[u8]) -> Vec<u8> {
    let padded_len = (data.len() + 31) / 32 * 32;
    let mut result = vec![0u8; 32 + padded_len]; // length + padded data
    // Length
    let len_bytes = (data.len() as u64).to_be_bytes();
    result[24..32].copy_from_slice(&len_bytes);
    // Data
    result[32..32 + data.len()].copy_from_slice(data);
    result
}

/// Encode a dynamic array of bytes32 values
fn encode_bytes32_array(items: &[[u8; 32]]) -> Vec<u8> {
    let mut result = vec![0u8; 32]; // length
    let len_bytes = (items.len() as u64).to_be_bytes();
    result[24..32].copy_from_slice(&len_bytes);
    for item in items {
        result.extend_from_slice(item);
    }
    result
}

/// Encode a dynamic array of u256 values
fn encode_u256_array(items: &[u64]) -> Vec<u8> {
    let mut result = vec![0u8; 32]; // length
    let len_bytes = (items.len() as u64).to_be_bytes();
    result[24..32].copy_from_slice(&len_bytes);
    for item in items {
        result.extend_from_slice(&encode_u256(*item));
    }
    result
}

/// Encode a dynamic array of dynamic bytes values
fn encode_bytes_array(items: &[Vec<u8>]) -> Vec<u8> {
    // First: length of array
    let mut header = vec![0u8; 32];
    let len_bytes = (items.len() as u64).to_be_bytes();
    header[24..32].copy_from_slice(&len_bytes);

    // Offsets for each item (relative to start of data area after offsets)
    let offsets_size = items.len() * 32;
    let mut offsets = Vec::with_capacity(offsets_size);
    let mut data_parts = Vec::new();
    let mut current_offset = offsets_size;

    for item in items {
        offsets.extend_from_slice(&encode_u256(current_offset as u64));
        let encoded = encode_dynamic_bytes(item);
        current_offset += encoded.len();
        data_parts.push(encoded);
    }

    let mut result = header;
    result.extend(offsets);
    for part in data_parts {
        result.extend(part);
    }
    result
}

// ============================================================================
// BIP322 signature verification precompile (0xFE)
// ============================================================================

/// BIP322 signature verification precompile.
///
/// ABI: verify(bytes pkscript, bytes message, bytes signature) -> bool
///
/// NOTE: Full BIP322 verification requires the `bip322` crate which needs
/// bitcoin 0.32+. Currently we have bitcoin 0.30.2.
/// This implementation decodes the ABI parameters correctly but returns false
/// for all verification attempts until the bitcoin crate is upgraded.
pub fn bip322_verify(input: &[u8], gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_BIP322_VERIFY {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Validate minimum input size (4 selector + 3*32 offsets = 100 bytes minimum)
    if input.len() < 100 {
        return PrecompileResult { success: false, gas_used: GAS_BIP322_VERIFY, output: vec![] };
    }

    // Validate input size limit (32KB per OPI spec)
    if input.len() > 32768 {
        return PrecompileResult { success: false, gas_used: GAS_BIP322_VERIFY, output: vec![] };
    }

    // TODO: Full BIP322 verification when bitcoin crate is upgraded to 0.32+
    // For now, return false (verification failed) — this is the safe default
    // since a false negative is better than a false positive for signature verification.
    let mut output = vec![0u8; 32];
    output[31] = 0; // false

    PrecompileResult { success: true, gas_used: GAS_BIP322_VERIFY, output }
}

// ============================================================================
// BTC transaction details precompile (0xFD)
// ============================================================================

/// BTC transaction details precompile.
///
/// ABI: getTxDetails(bytes32 txid) -> (
///     uint256 block_height,
///     bytes32[] vin_txids,
///     uint256[] vin_vouts,
///     bytes[] vin_scriptPubKeys,
///     uint256[] vin_values,
///     bytes[] vout_scriptPubKeys,
///     uint256[] vout_values
/// )
pub fn btc_tx_details(input: &[u8], gas_limit: u64, current_height: u32) -> PrecompileResult {
    if gas_limit < GAS_BTC_RPC_CALL {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Extract txid from input (skip 4 bytes function selector, read 32 bytes)
    if input.len() < 36 {
        return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] };
    }

    // txid is in the ABI as big-endian bytes32, but Bitcoin uses little-endian internally
    let mut txid_bytes = [0u8; 32];
    txid_bytes.copy_from_slice(&input[4..36]);
    // Reverse to get Bitcoin internal byte order
    txid_bytes.reverse();

    // Look up the transaction
    let (tx, tx_height) = match lookup_tx(&txid_bytes) {
        Some(v) => v,
        None => return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] },
    };

    // Validate tx is not in the future
    if tx_height > current_height {
        return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] };
    }

    let is_coinbase = tx.input.len() == 1 && tx.input[0].previous_output.is_null();

    // Calculate gas: base + per-input for looking up previous txs (coinbase has 0 input lookups)
    let vin_count = if is_coinbase { 0 } else { tx.input.len() as u64 };
    let input_gas = vin_count * GAS_BTC_RPC_CALL;
    let total_gas = GAS_BTC_RPC_CALL + input_gas;
    if gas_limit < total_gas {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Build vin data by looking up each input's previous output
    let mut vin_txids: Vec<[u8; 32]> = Vec::new();
    let mut vin_vouts: Vec<u64> = Vec::new();
    let mut vin_scripts: Vec<Vec<u8>> = Vec::new();
    let mut vin_values: Vec<u64> = Vec::new();

    if !is_coinbase {
        for txin in &tx.input {
            let prev_txid = txin.previous_output.txid;
            let prev_vout = txin.previous_output.vout;

            // Reverse txid for ABI encoding (back to big-endian)
            let mut prev_txid_be = *prev_txid.as_byte_array();
            prev_txid_be.reverse();
            vin_txids.push(prev_txid_be);
            vin_vouts.push(prev_vout as u64);

            // Look up the previous transaction to get scriptPubKey and value
            if let Some((prev_tx, _)) = lookup_tx(prev_txid.as_byte_array()) {
                if let Some(prev_output) = prev_tx.output.get(prev_vout as usize) {
                    vin_scripts.push(prev_output.script_pubkey.as_bytes().to_vec());
                    vin_values.push(prev_output.value);
                } else {
                    vin_scripts.push(vec![]);
                    vin_values.push(0);
                }
            } else {
                vin_scripts.push(vec![]);
                vin_values.push(0);
            }
        }
    }

    // Build vout data
    let mut vout_scripts: Vec<Vec<u8>> = Vec::new();
    let mut vout_values: Vec<u64> = Vec::new();
    for txout in &tx.output {
        vout_scripts.push(txout.script_pubkey.as_bytes().to_vec());
        vout_values.push(txout.value);
    }

    // ABI-encode the response as a tuple of 7 elements
    // Each dynamic element gets an offset in the header, then data appended
    let header_size = 7 * 32; // 7 fields, each 32 bytes (value or offset)

    // Encode each dynamic array
    let vin_txids_enc = encode_bytes32_array(&vin_txids);
    let vin_vouts_enc = encode_u256_array(&vin_vouts);
    let vin_scripts_enc = encode_bytes_array(&vin_scripts);
    let vin_values_enc = encode_u256_array(&vin_values);
    let vout_scripts_enc = encode_bytes_array(&vout_scripts);
    let vout_values_enc = encode_u256_array(&vout_values);

    // Calculate offsets (relative to start of tuple encoding)
    let mut current_offset = header_size;
    let offsets = [
        // block_height is inline (not an offset)
        0u64, // placeholder
        current_offset as u64, // vin_txids
        { current_offset += vin_txids_enc.len(); current_offset as u64 }, // vin_vouts
        { current_offset += vin_vouts_enc.len(); current_offset as u64 }, // vin_scripts
        { current_offset += vin_scripts_enc.len(); current_offset as u64 }, // vin_values
        { current_offset += vin_values_enc.len(); current_offset as u64 }, // vout_scripts
        { current_offset += vout_scripts_enc.len(); current_offset as u64 }, // vout_values
    ];

    let mut output = Vec::with_capacity(header_size + vin_txids_enc.len() + vin_vouts_enc.len()
        + vin_scripts_enc.len() + vin_values_enc.len() + vout_scripts_enc.len() + vout_values_enc.len());

    // Header: block_height (inline) + 6 offsets
    output.extend_from_slice(&encode_u256(tx_height as u64));
    for i in 1..7 {
        output.extend_from_slice(&encode_u256(offsets[i]));
    }

    // Data
    output.extend(&vin_txids_enc);
    output.extend(&vin_vouts_enc);
    output.extend(&vin_scripts_enc);
    output.extend(&vin_values_enc);
    output.extend(&vout_scripts_enc);
    output.extend(&vout_values_enc);

    PrecompileResult { success: true, gas_used: total_gas, output }
}

// ============================================================================
// Last satoshi location precompile (0xFC)
// ============================================================================

/// Last satoshi location precompile.
///
/// ABI: getLastSatLocation(bytes32 txid, uint256 vout, uint256 sat) -> (
///     bytes32 last_txid,
///     uint256 last_vout,
///     uint256 last_sat,
///     bytes old_pkscript,
///     bytes new_pkscript
/// )
///
/// Traces a specific satoshi backwards through the transaction chain.
pub fn last_sat_location(input: &[u8], gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_BTC_RPC_CALL {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Need at least 4 (selector) + 32 (txid) + 32 (vout) + 32 (sat) = 100 bytes
    if input.len() < 100 {
        return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] };
    }

    // Decode inputs
    let mut txid_bytes = [0u8; 32];
    txid_bytes.copy_from_slice(&input[4..36]);
    txid_bytes.reverse(); // Big-endian to Bitcoin internal

    let vout = {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&input[60..68]); // low 8 bytes of uint256
        u64::from_be_bytes(buf) as u32
    };

    let sat = {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&input[92..100]); // low 8 bytes of uint256
        u64::from_be_bytes(buf)
    };

    // Look up the transaction
    let (tx, _) = match lookup_tx(&txid_bytes) {
        Some(v) => v,
        None => return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] },
    };

    // Reject coinbase transactions
    let is_coinbase = tx.input.len() == 1 && tx.input[0].previous_output.is_null();
    if is_coinbase {
        return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] };
    }

    // Validate vout index
    if vout as usize >= tx.output.len() {
        return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] };
    }

    // Validate sat within output value
    let output_value = tx.output[vout as usize].value;
    if sat >= output_value {
        return PrecompileResult { success: false, gas_used: GAS_BTC_RPC_CALL, output: vec![] };
    }

    // Get new_pkscript (the output script at vout)
    let new_pkscript = tx.output[vout as usize].script_pubkey.as_bytes().to_vec();

    // Calculate absolute sat position through outputs
    let mut total_vout_sats: u64 = 0;
    for i in 0..vout as usize {
        total_vout_sats += tx.output[i].value;
    }
    total_vout_sats += sat;

    // Calculate gas for input traversal
    let input_gas = tx.input.len() as u64 * GAS_BTC_RPC_CALL;
    let total_gas = GAS_BTC_RPC_CALL + input_gas;
    if gas_limit < total_gas {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    // Walk through inputs to find which one contains our satoshi
    let mut accumulated_sats: u64 = 0;
    let mut found = false;
    let mut last_txid_be = [0u8; 32];
    let mut last_vout: u64 = 0;
    let mut last_sat: u64 = 0;
    let mut old_pkscript: Vec<u8> = Vec::new();

    for txin in &tx.input {
        let prev_txid = txin.previous_output.txid;
        let prev_vout = txin.previous_output.vout;

        let prev_output_value = if let Some((prev_tx, _)) = lookup_tx(prev_txid.as_byte_array()) {
            if let Some(prev_out) = prev_tx.output.get(prev_vout as usize) {
                old_pkscript = prev_out.script_pubkey.as_bytes().to_vec();
                prev_out.value
            } else {
                0
            }
        } else {
            0
        };

        accumulated_sats += prev_output_value;

        if accumulated_sats > total_vout_sats {
            // This input contains our satoshi
            let mut prev_txid_be = *prev_txid.as_byte_array();
            prev_txid_be.reverse();
            last_txid_be = prev_txid_be;
            last_vout = prev_vout as u64;
            // Calculate sat offset within this input
            last_sat = total_vout_sats - (accumulated_sats - prev_output_value);
            found = true;
            break;
        }
    }

    if !found {
        return PrecompileResult { success: false, gas_used: total_gas, output: vec![] };
    }

    // ABI-encode response: (bytes32, uint256, uint256, bytes, bytes)
    // Header: 5 slots (first 3 inline, last 2 offsets)
    let header_size = 5 * 32;
    let old_pk_enc = encode_dynamic_bytes(&old_pkscript);
    let new_pk_enc = encode_dynamic_bytes(&new_pkscript);

    let old_pk_offset = header_size;
    let new_pk_offset = old_pk_offset + old_pk_enc.len();

    let mut output = Vec::with_capacity(header_size + old_pk_enc.len() + new_pk_enc.len());
    output.extend_from_slice(&encode_bytes32(&last_txid_be));   // last_txid
    output.extend_from_slice(&encode_u256(last_vout));           // last_vout
    output.extend_from_slice(&encode_u256(last_sat));            // last_sat
    output.extend_from_slice(&encode_u256(old_pk_offset as u64)); // offset to old_pkscript
    output.extend_from_slice(&encode_u256(new_pk_offset as u64)); // offset to new_pkscript
    output.extend(&old_pk_enc);
    output.extend(&new_pk_enc);

    PrecompileResult { success: true, gas_used: total_gas, output }
}

// ============================================================================
// Get locked pkscript precompile (0xFB) — already implemented
// ============================================================================

/// Get locked pkscript precompile (0xFB).
///
/// Input ABI: getLockedPkscript(bytes pkscript, uint256 lock_block_count) -> bytes
///
/// Builds a P2TR output script with a CSV timelock wrapping the given pkscript.
pub fn get_locked_pkscript(input: &[u8], gas_limit: u64) -> PrecompileResult {
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
        match u256_to_usize(buf) {
            Some(o) => o,
            None => return fail(),
        }
    };

    // Decode lock_block_count (second 32 bytes of data), use as u64
    let lock_block_count = {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&data[32..64]);
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
    for &b in &bytes[..24] {
        if b != 0 {
            return None;
        }
    }
    let val = u64::from_be_bytes(bytes[24..32].try_into().unwrap());
    usize::try_from(val).ok()
}

// ============================================================================
// Get OP_RETURN transaction ID precompile (0xFA) — already implemented
// ============================================================================

/// Get OP_RETURN transaction ID precompile (0xFA).
///
/// Returns the current OP_RETURN transaction ID from context.
///
/// Input ABI: getTxId() -> bytes32
pub fn get_op_return_txid(op_return_tx_id: B256, gas_limit: u64) -> PrecompileResult {
    if gas_limit < GAS_OP_RETURN_TXID {
        return PrecompileResult { success: false, gas_used: gas_limit, output: vec![] };
    }

    let output = op_return_tx_id.to_vec();
    PrecompileResult { success: true, gas_used: GAS_OP_RETURN_TXID, output }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test as test;

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
