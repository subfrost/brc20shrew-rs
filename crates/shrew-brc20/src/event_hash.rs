///! BRC-20 Event Hash v3 Calculation
///!
///! Implements the OPI event hash format for verifying BRC-20 indexer state.
///! See: https://github.com/bestinslot-xyz/OPI#brc-20-indexer--api

use sha2::{Sha256, Digest};

const EVENT_SEPARATOR: char = '|';

/// A BRC-20 event that contributes to the block event hash.
#[derive(Debug, Clone)]
pub enum Brc20Event {
    DeployInscribe {
        inscription_id: String,
        deployer_pkscript: String,
        ticker_lowercase: String,
        ticker_original: String,
        max_supply: String,
        decimals: u8,
        limit_per_mint: String,
        is_self_mint: bool,
    },
    MintInscribe {
        inscription_id: String,
        minter_pkscript: String,
        ticker_lowercase: String,
        ticker_original: String,
        amount: String,
        parent_id: Option<String>,
    },
    TransferInscribe {
        inscription_id: String,
        source_pkscript: String,
        ticker_lowercase: String,
        ticker_original: String,
        amount: String,
    },
    TransferTransfer {
        inscription_id: String,
        source_pkscript: String,
        sent_pkscript: String,
        ticker_lowercase: String,
        ticker_original: String,
        amount: String,
    },
}

impl Brc20Event {
    /// Format the event as an OPI v3 event string.
    pub fn to_event_string(&self) -> String {
        match self {
            Brc20Event::DeployInscribe {
                inscription_id, deployer_pkscript, ticker_lowercase, ticker_original,
                max_supply, decimals, limit_per_mint, is_self_mint,
            } => {
                format!(
                    "deploy-inscribe;{};{};{};{};{};{};{};{}",
                    inscription_id, deployer_pkscript, ticker_lowercase, ticker_original,
                    max_supply, decimals, limit_per_mint,
                    if *is_self_mint { "true" } else { "false" }
                )
            }
            Brc20Event::MintInscribe {
                inscription_id, minter_pkscript, ticker_lowercase, ticker_original,
                amount, parent_id,
            } => {
                format!(
                    "mint-inscribe;{};{};{};{};{};{}",
                    inscription_id, minter_pkscript, ticker_lowercase, ticker_original,
                    amount, parent_id.as_deref().unwrap_or("")
                )
            }
            Brc20Event::TransferInscribe {
                inscription_id, source_pkscript, ticker_lowercase, ticker_original, amount,
            } => {
                format!(
                    "transfer-inscribe;{};{};{};{};{}",
                    inscription_id, source_pkscript, ticker_lowercase, ticker_original, amount
                )
            }
            Brc20Event::TransferTransfer {
                inscription_id, source_pkscript, sent_pkscript,
                ticker_lowercase, ticker_original, amount,
            } => {
                format!(
                    "transfer-transfer;{};{};{};{};{};{}",
                    inscription_id, source_pkscript, sent_pkscript,
                    ticker_lowercase, ticker_original, amount
                )
            }
        }
    }
}

/// Computes per-block and cumulative event hashes per OPI v3.
pub struct EventHasher {
    events: Vec<String>,
}

impl EventHasher {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Add a BRC-20 event to this block's hash computation.
    pub fn add_event(&mut self, event: &Brc20Event) {
        self.events.push(event.to_event_string());
    }

    /// Compute the block event hash: sha256_hex of pipe-joined event strings.
    /// Returns empty string if no events in this block.
    pub fn compute_block_hash(&self) -> String {
        if self.events.is_empty() {
            return String::new();
        }
        let block_str = self.events.join(&EVENT_SEPARATOR.to_string());
        sha256_hex(&block_str)
    }

    /// Compute the cumulative hash: sha256_hex(last_cumulative + block_hash).
    /// For the first block, last_cumulative should be "".
    pub fn compute_cumulative_hash(last_cumulative: &str, block_hash: &str) -> String {
        let input = format!("{}{}", last_cumulative, block_hash);
        sha256_hex(&input)
    }
}

/// Format a fixed-point u128 amount (scaled by 10^18) as a decimal string
/// with the given number of decimal places.
///
/// Per OPI: decimal count matches the ticker's decimals.
/// No trailing dot if decimals is 0.
pub fn format_amount(value: u128, decimals: u8) -> String {
    let scale: u128 = 10u128.pow(18);
    if decimals == 0 {
        // Integer only, no dot
        return (value / scale).to_string();
    }

    let display_scale: u128 = 10u128.pow(decimals as u32);
    // Convert from 18-decimal fixed point to the ticker's decimal precision
    let rescaled = value / 10u128.pow(18 - decimals as u32);
    let integer_part = rescaled / display_scale;
    let fractional_part = rescaled % display_scale;

    format!(
        "{}.{:0>width$}",
        integer_part,
        fractional_part,
        width = decimals as usize
    )
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
