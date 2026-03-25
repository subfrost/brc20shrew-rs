///! BRC20-prog Trace Hash Calculation
///!
///! Computes per-block and cumulative trace hashes matching the OPI format.
///! Trace entries are formatted as semicolon-delimited OPI strings, joined
///! by pipe separator per block, then sha256 hashed.

use sha2::{Sha256, Digest};

const TRACE_SEPARATOR: &str = "|";

/// A single EVM execution trace entry.
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Call type: "CALL", "CREATE", "STATICCALL", "DELEGATECALL", etc.
    pub tx_type: String,
    /// Sender address (lowercase hex, no 0x prefix)
    pub from: String,
    /// Recipient address (lowercase hex, no 0x prefix), None for CREATE
    pub to: Option<String>,
    /// Gas allocated
    pub gas: u64,
    /// Gas actually used
    pub gas_used: u64,
    /// Input data (hex lowercase, no 0x prefix)
    pub input: String,
    /// Output data (hex lowercase, no 0x prefix)
    pub output: String,
    /// Nested internal calls
    pub calls: Vec<TraceEntry>,
}

impl TraceEntry {
    /// Format as an OPI trace string:
    /// TYPE;from;to;gas;gasUsed;input;output;[nested,calls]
    pub fn to_opi_string(&self) -> String {
        let to_str = self.to.as_deref().unwrap_or("");
        let calls_str = self.calls.iter()
            .map(|c| c.to_opi_string())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{};{};{};{};{};{};{};[{}]",
            self.tx_type, self.from, to_str,
            self.gas, self.gas_used,
            self.input, self.output,
            calls_str,
        )
    }
}

/// Computes per-block and cumulative trace hashes.
pub struct TraceHasher {
    traces: Vec<String>,
}

impl TraceHasher {
    pub fn new() -> Self {
        Self { traces: Vec::new() }
    }

    /// Add a trace entry for one transaction.
    pub fn add_trace(&mut self, trace: &TraceEntry) {
        self.traces.push(trace.to_opi_string());
    }

    /// Compute the block trace hash: sha256_hex of pipe-joined trace strings.
    /// Returns empty string if no traces.
    pub fn compute_block_hash(&self) -> String {
        if self.traces.is_empty() {
            return String::new();
        }
        let block_str = self.traces.join(TRACE_SEPARATOR);
        sha256_hex(&block_str)
    }

    /// Compute the cumulative hash: sha256_hex(last_cumulative + block_hash).
    pub fn compute_cumulative_hash(last_cumulative: &str, block_hash: &str) -> String {
        let input = format!("{}{}", last_cumulative, block_hash);
        sha256_hex(&input)
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
