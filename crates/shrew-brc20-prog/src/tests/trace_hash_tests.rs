///! BRC20-prog Trace Hash Tests
///!
///! Per OPI spec, the trace hash is computed from EVM execution traces:
///! - Per-tx: TraceED formatted as OPI string (semicolon-delimited)
///! - Per-block: tx traces joined by '|' separator, sorted by tx index
///! - block_trace_hash = sha256(block_trace_string)
///! - cumulative_trace_hash = sha256(prev_cumulative + block_trace_hash)
///!
///! OPI trace string format:
///!   TYPE;from;to;gas;gasUsed;input;output;[nested_calls]
///!   - TYPE: CALL, CREATE, etc. (uppercase)
///!   - from/to: 20-byte addresses lowercase hex no 0x prefix
///!   - gas/gasUsed: decimal integers
///!   - input/output: hex lowercase no 0x prefix
///!   - nested calls: comma-separated recursive OPI strings inside []

use crate::trace_hash::{TraceEntry, TraceHasher};
use sha2::{Sha256, Digest};
use wasm_bindgen_test::wasm_bindgen_test;

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ============================================================================
// TEST 1: Simple CALL trace OPI string format
// ============================================================================

#[wasm_bindgen_test]
fn test_trace_entry_simple_call_format() {
    let trace = TraceEntry {
        tx_type: "CALL".to_string(),
        from: "0000000000000000000000000000000000000000".to_string(),
        to: Some("0101010101010101010101010101010101010101".to_string()),
        gas: 21000,
        gas_used: 21001,
        input: "6000".to_string(),
        output: "20".to_string(),
        calls: vec![],
    };

    let opi_str = trace.to_opi_string();
    assert_eq!(
        opi_str,
        "CALL;0000000000000000000000000000000000000000;0101010101010101010101010101010101010101;21000;21001;6000;20;[]"
    );
}

// ============================================================================
// TEST 2: CREATE trace with no 'to' address
// ============================================================================

#[wasm_bindgen_test]
fn test_trace_entry_create_no_to() {
    let trace = TraceEntry {
        tx_type: "CREATE".to_string(),
        from: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        to: None,
        gas: 100000,
        gas_used: 50000,
        input: "608060405234".to_string(),
        output: "".to_string(),
        calls: vec![],
    };

    let opi_str = trace.to_opi_string();
    assert_eq!(
        opi_str,
        "CREATE;aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa;;100000;50000;608060405234;;[]"
    );
}

// ============================================================================
// TEST 3: Nested calls
// ============================================================================

#[wasm_bindgen_test]
fn test_trace_entry_nested_calls() {
    let inner = TraceEntry {
        tx_type: "CALL".to_string(),
        from: "0202020202020202020202020202020202020202".to_string(),
        to: Some("0303030303030303030303030303030303030303".to_string()),
        gas: 21000,
        gas_used: 21000,
        input: "6000".to_string(),
        output: "00".to_string(),
        calls: vec![],
    };

    let outer = TraceEntry {
        tx_type: "CALL".to_string(),
        from: "0000000000000000000000000000000000000000".to_string(),
        to: Some("0101010101010101010101010101010101010101".to_string()),
        gas: 21000,
        gas_used: 21001,
        input: "6000".to_string(),
        output: "20".to_string(),
        calls: vec![inner],
    };

    let opi_str = outer.to_opi_string();
    assert_eq!(
        opi_str,
        "CALL;0000000000000000000000000000000000000000;0101010101010101010101010101010101010101;21000;21001;6000;20;[CALL;0202020202020202020202020202020202020202;0303030303030303030303030303030303030303;21000;21000;6000;00;[]]"
    );
}

// ============================================================================
// TEST 4: Block trace hash — single tx
// ============================================================================

#[wasm_bindgen_test]
fn test_block_trace_hash_single_tx() {
    let mut hasher = TraceHasher::new();

    let trace = TraceEntry {
        tx_type: "CALL".to_string(),
        from: "0000000000000000000000000000000000000000".to_string(),
        to: Some("0101010101010101010101010101010101010101".to_string()),
        gas: 21000,
        gas_used: 21000,
        input: "".to_string(),
        output: "".to_string(),
        calls: vec![],
    };

    hasher.add_trace(&trace);
    let block_hash = hasher.compute_block_hash();

    let expected_str = "CALL;0000000000000000000000000000000000000000;0101010101010101010101010101010101010101;21000;21000;;;[]";
    let expected = sha256_hex(expected_str);
    assert_eq!(block_hash, expected);
}

// ============================================================================
// TEST 5: Block trace hash — multiple txs joined by pipe
// ============================================================================

#[wasm_bindgen_test]
fn test_block_trace_hash_multiple_txs() {
    let mut hasher = TraceHasher::new();

    let trace1 = TraceEntry {
        tx_type: "CALL".to_string(),
        from: "aaaa".to_string(),
        to: Some("bbbb".to_string()),
        gas: 100,
        gas_used: 50,
        input: "ff".to_string(),
        output: "00".to_string(),
        calls: vec![],
    };
    let trace2 = TraceEntry {
        tx_type: "CREATE".to_string(),
        from: "cccc".to_string(),
        to: None,
        gas: 200,
        gas_used: 100,
        input: "6080".to_string(),
        output: "".to_string(),
        calls: vec![],
    };

    hasher.add_trace(&trace1);
    hasher.add_trace(&trace2);
    let block_hash = hasher.compute_block_hash();

    let expected_str = "CALL;aaaa;bbbb;100;50;ff;00;[]|CREATE;cccc;;200;100;6080;;[]";
    let expected = sha256_hex(expected_str);
    assert_eq!(block_hash, expected);
}

// ============================================================================
// TEST 6: Cumulative trace hash chaining
// ============================================================================

#[wasm_bindgen_test]
fn test_cumulative_trace_hash_chaining() {
    let mut hasher1 = TraceHasher::new();
    let trace1 = TraceEntry {
        tx_type: "CALL".to_string(),
        from: "aaaa".to_string(),
        to: Some("bbbb".to_string()),
        gas: 100,
        gas_used: 50,
        input: "ff".to_string(),
        output: "00".to_string(),
        calls: vec![],
    };
    hasher1.add_trace(&trace1);
    let block_hash_1 = hasher1.compute_block_hash();

    // First block: cumulative = sha256("" + block_hash)
    let cumulative_1 = TraceHasher::compute_cumulative_hash("", &block_hash_1);
    let expected_1 = sha256_hex(&block_hash_1);
    assert_eq!(cumulative_1, expected_1);

    let mut hasher2 = TraceHasher::new();
    let trace2 = TraceEntry {
        tx_type: "CREATE".to_string(),
        from: "cccc".to_string(),
        to: None,
        gas: 200,
        gas_used: 100,
        input: "6080".to_string(),
        output: "".to_string(),
        calls: vec![],
    };
    hasher2.add_trace(&trace2);
    let block_hash_2 = hasher2.compute_block_hash();

    // Second block: cumulative = sha256(cumulative_1 + block_hash_2)
    let cumulative_2 = TraceHasher::compute_cumulative_hash(&cumulative_1, &block_hash_2);
    let expected_2 = sha256_hex(&format!("{}{}", cumulative_1, block_hash_2));
    assert_eq!(cumulative_2, expected_2);
}

// ============================================================================
// TEST 7: Empty block trace hash
// ============================================================================

#[wasm_bindgen_test]
fn test_empty_block_trace_hash() {
    let hasher = TraceHasher::new();
    let block_hash = hasher.compute_block_hash();
    assert!(block_hash.is_empty(), "Empty block should produce empty trace hash");
}
