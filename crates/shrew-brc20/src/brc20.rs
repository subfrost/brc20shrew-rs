use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::tables::*;
use shrew_support::inscription::{InscriptionEntry, InscriptionId};
use shrew_support::utils::get_address_from_txout;
use shrew_support::constants::{BRC20_SELF_MINT_ENABLE_HEIGHT, BRC20_PROG_PHASE_ONE_HEIGHT};
use shrew_ord::tables::{
    INSCRIPTION_ID_TO_SEQUENCE, SEQUENCE_TO_INSCRIPTION_ENTRY,
    OUTPOINT_TO_INSCRIPTIONS, INSCRIPTION_CONTENT,
};
use bitcoin_hashes::Hash;
use bitcoin::{Block, Network, Transaction};
use metashrew_support::index_pointer::KeyValuePointer;
use std::str::FromStr;

/// Maximum representable BRC-20 amount: (2^64 - 1) * 10^18
/// Matches OPI reference: `pub const MAX_AMOUNT: u128 = (2u128.pow(64) - 1) * 10u128.pow(18);`
pub const MAX_AMOUNT: u128 = (u64::MAX as u128) * 1_000_000_000_000_000_000u128;

/// Fixed-point scale factor: all amounts are stored as value * 10^18
const FIXED_POINT_SCALE: u128 = 1_000_000_000_000_000_000u128; // 10^18

/// OP_RETURN pkscript prefix (bare OP_RETURN = 0x6a)
pub const OP_RETURN_PKSCRIPT: &str = "6a";

/// BRC20-PROG OP_RETURN pkscript: OP_RETURN OP_PUSH9 "BRC20PROG"
pub const BRC20_PROG_OP_RETURN_PKSCRIPT: &str = "6a09425243323050524f47";

/// BRC20-prog phase 2 (all tickers) — not yet finalized on mainnet
pub const BRC20_PROG_ALL_TICKERS_HEIGHT: u32 = 9999999;

/// Transfer destination type — determines how a transfer-transfer is resolved
#[derive(Debug, Clone, PartialEq)]
pub enum TransferDestination {
    /// Normal wallet-to-wallet transfer
    Wallet(String),
    /// OP_RETURN output — tokens are burned
    Burn,
    /// BRC20-PROG OP_RETURN — tokens deposited to programmable module
    Brc20ProgDeposit,
    /// Inscription spent as fee — tokens returned to sender
    SentAsFee,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Brc20Operation {
    Deploy {
        ticker: String,
        max_supply: u128,
        limit_per_mint: u128,
        decimals: u8,
        self_mint: bool,
    },
    Mint {
        ticker: String,
        amount: u128,
    },
    Transfer {
        ticker: String,
        amount: u128,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ticker {
    pub name: String,
    pub max_supply: u128,
    pub current_supply: u128,
    pub limit_per_mint: u128,
    pub decimals: u8,
    pub deploy_inscription_id: String,
    #[serde(default)]
    pub is_self_mint: bool,
    #[serde(default)]
    pub burned_supply: u128,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Balance {
    pub ticker: String,
    pub total_balance: u128,
    pub available_balance: u128,
}

impl Balance {
    pub fn new(ticker: String) -> Self {
        Self { ticker, total_balance: 0, available_balance: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferInfo {
    pub ticker: String,
    pub amount: u128,
    pub sender: String,
}

pub struct Brc20Indexer;

impl Brc20Indexer {
    pub fn new() -> Self { Self }

    /// Process an entire block for BRC20 operations.
    pub fn process_block(&self, block: &Block, height: u32) {
        let network = Network::Bitcoin;
        for tx in &block.txdata {
            // Check for BRC20 transfer claims (spending transferable inscriptions)
            self.process_brc20_transfers(tx, network);

            // Check for new BRC20 operations in inscriptions
            self.process_brc20_inscriptions(tx, network, height);
        }
    }

    fn process_brc20_inscriptions(&self, tx: &Transaction, network: Network, height: u32) {
        for (input_idx, _input) in tx.input.iter().enumerate() {
            let inscription_id = InscriptionId::new(tx.txid(), input_idx as u32);
            let seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.to_bytes()).get();
            if seq_bytes.is_empty() { continue; }

            let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
            if entry_bytes.is_empty() { continue; }

            let entry = match InscriptionEntry::from_bytes(&entry_bytes) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let _content_type = match &entry.content_type {
                Some(ct) if ct.starts_with("text/plain") || ct.starts_with("application/json") => ct.clone(),
                _ => continue,
            };

            let inscription_id_str = inscription_id.to_string();
            let content_bytes = INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).get();
            if content_bytes.is_empty() { continue; }

            if let Some(operation) = self.parse_operation(&content_bytes, height) {
                // For self-mint mints, validate that the parent inscription matches the deploy inscription
                if let Brc20Operation::Mint { ref ticker, .. } = operation {
                    let ticker_lower = ticker.to_lowercase();
                    if let Some(ticker_data) = Brc20Tickers::new().get(&ticker_lower) {
                        if let Ok(ticker_entry) = serde_json::from_slice::<Ticker>(&ticker_data) {
                            if ticker_entry.is_self_mint {
                                // Self-mint mint requires parent inscription ID == deploy inscription ID
                                let deploy_id = InscriptionId::from_str(&ticker_entry.deploy_inscription_id).ok();
                                let has_valid_parent = match (&entry.parent, &deploy_id) {
                                    (Some(parent), Some(deploy)) => parent == deploy,
                                    _ => false,
                                };
                                if !has_valid_parent {
                                    continue; // Skip: self-mint mint without valid parent
                                }
                            }
                        }
                    }
                }

                if let Some(first_output) = tx.output.get(0) {
                    if let Some(address) = get_address_from_txout(first_output, network) {
                        let _ = self.process_operation(&operation, &inscription_id_str, &address.to_string());
                    }
                }
            }
        }
    }

    fn process_brc20_transfers(&self, tx: &Transaction, network: Network) {
        for input in &tx.input {
            let outpoint_bytes = input.previous_output.txid.as_byte_array()
                .iter().chain(input.previous_output.vout.to_le_bytes().iter()).copied().collect::<Vec<u8>>();
            let inscription_sequences = OUTPOINT_TO_INSCRIPTIONS.select(&outpoint_bytes).get_list();
            if inscription_sequences.is_empty() { continue; }

            for seq_bytes in inscription_sequences {
                let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
                if entry_bytes.is_empty() { continue; }
                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    let inscription_id_str = entry.id.to_string();
                    if let Some(transfer_info_bytes) = Brc20TransferableInscriptions::new().get(&inscription_id_str) {
                        if let Ok(transfer_info) = serde_json::from_slice::<TransferInfo>(&transfer_info_bytes) {
                            if let Some(first_output) = tx.output.get(0) {
                                if let Some(new_owner) = get_address_from_txout(first_output, network) {
                                    let _ = self.claim_transfer(&new_owner.to_string(), &transfer_info);
                                    Brc20TransferableInscriptions::new().delete(&inscription_id_str);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Parse a BRC-20 amount string into 18-decimal fixed-point u128.
    ///
    /// Matches OPI `get_amount_value()`:
    /// - Integer amounts: "1000" -> 1000 * 10^18
    /// - Decimal amounts: "1000.5" -> 1000.5 * 10^18
    /// - Rejects negative, leading dot, trailing dot, multiple dots
    /// - Rejects amounts with more than 18 decimal places
    /// - Rejects amounts exceeding MAX_AMOUNT
    fn parse_amount(s: &str) -> Option<u128> {
        // Must be a valid positive decimal number
        if s.is_empty() { return None; }

        // Reject negative numbers
        if s.starts_with('-') { return None; }

        // Reject leading dot (e.g. ".5")
        if s.starts_with('.') { return None; }

        // Reject trailing dot (e.g. "100.")
        if s.ends_with('.') { return None; }

        // All chars must be digits or a single dot
        let dot_count = s.chars().filter(|c| *c == '.').count();
        if dot_count > 1 { return None; }
        if !s.chars().all(|c| c.is_ascii_digit() || c == '.') { return None; }

        let result: u128;
        if let Some(dot_index) = s.find('.') {
            let integer_part = &s[..dot_index];
            let decimal_part = &s[dot_index + 1..];

            if decimal_part.len() > 18 { return None; }

            // Build the scaled integer: integer_part + decimal_part + padding zeros
            let mut combined = String::new();
            combined.push_str(integer_part);
            combined.push_str(decimal_part);
            for _ in decimal_part.len()..18 {
                combined.push('0');
            }
            result = combined.parse::<u128>().ok()?;
        } else {
            // Integer amount: multiply by 10^18
            let integer_val = s.parse::<u128>().ok()?;
            result = integer_val.checked_mul(FIXED_POINT_SCALE)?;
        }

        // Reject zero
        // Note: zero rejection is handled by callers for mint/transfer,
        // but deploy max_supply=0 has special handling. Return the value.

        // Reject amounts exceeding MAX_AMOUNT
        if result > MAX_AMOUNT { return None; }

        Some(result)
    }

    /// Check if a string contains only alphanumeric characters or dashes.
    fn is_alphanumeric_or_dash(s: &str) -> bool {
        s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }

    /// Validate a ticker string per OPI rules:
    /// - Must be 4, 5, or 6 bytes
    /// - No null bytes
    /// - 6-byte tickers must be alphanumeric or dash only
    /// - Normalized to lowercase for storage
    fn validate_ticker(ticker: &str) -> Option<String> {
        let ticker_bytes = ticker.as_bytes();

        // Reject null bytes
        if ticker_bytes.contains(&0x00) { return None; }

        let len = ticker_bytes.len();

        match len {
            4 => {} // Standard BRC-20 ticker
            5 => {} // Self-mint ticker (height validation done elsewhere)
            6 => {
                // Predeploy ticker: must be alphanumeric or dash only
                if !Self::is_alphanumeric_or_dash(ticker) { return None; }
            }
            _ => return None,
        }

        // Normalize to lowercase (OPI: `original_ticker.to_lowercase()`)
        Some(ticker.to_lowercase())
    }

    pub fn parse_operation(&self, content: &[u8], height: u32) -> Option<Brc20Operation> {
        let content_str = std::str::from_utf8(content).ok()?;
        let json: serde_json::Value = serde_json::from_str(content_str).ok()?;

        // Validate protocol field: must be "brc-20"
        let protocol = json.get("p")?.as_str()?;
        if protocol != "brc-20" { return None; }

        let op = json.get("op")?.as_str()?;
        let raw_ticker = json.get("tick")?.as_str()?;

        // Validate and normalize ticker
        let ticker = Self::validate_ticker(raw_ticker)?;
        let ticker_byte_len = raw_ticker.as_bytes().len();

        // Height-based validation for extended tickers
        match ticker_byte_len {
            5 => {
                if height < BRC20_SELF_MINT_ENABLE_HEIGHT { return None; }
            }
            6 => {
                if height < BRC20_PROG_PHASE_ONE_HEIGHT { return None; }
            }
            _ => {} // 4-byte tickers always allowed
        }

        match op {
            "deploy" => {
                // Determine if this is a self-mint deploy
                let is_self_mint = ticker_byte_len == 5;

                // 5-byte tickers require "self_mint": "true" in the JSON
                if is_self_mint {
                    let self_mint_val = json.get("self_mint").and_then(|v| v.as_str());
                    if self_mint_val != Some("true") { return None; }
                }

                let max_supply_raw = Self::parse_amount(json.get("max")?.as_str()?)?;

                // For self-mint: max_supply of 0 defaults to MAX_AMOUNT
                let max_supply = if is_self_mint && max_supply_raw == 0 {
                    MAX_AMOUNT
                } else {
                    if max_supply_raw == 0 { return None; } // Reject zero max_supply for non-self-mint
                    max_supply_raw
                };

                // lim defaults to max_supply when absent (OPI behavior)
                let limit_per_mint = if let Some(lim_val) = json.get("lim") {
                    let lim_str = lim_val.as_str()?;
                    let lim = Self::parse_amount(lim_str)?;
                    // For self-mint: lim of 0 defaults to MAX_AMOUNT
                    if is_self_mint && lim == 0 {
                        MAX_AMOUNT
                    } else {
                        if lim == 0 { return None; } // zero lim rejected for non-self-mint
                        lim
                    }
                } else {
                    max_supply
                };

                let decimals = json.get("dec")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u8>().ok())
                    .unwrap_or(18);

                // Decimals must be <= 18
                if decimals > 18 { return None; }

                Some(Brc20Operation::Deploy { ticker, max_supply, limit_per_mint, decimals, self_mint: is_self_mint })
            }
            "mint" => {
                let amount = Self::parse_amount(json.get("amt")?.as_str()?)?;
                if amount == 0 { return None; } // reject zero mint
                Some(Brc20Operation::Mint { ticker, amount })
            }
            "transfer" => {
                let amount = Self::parse_amount(json.get("amt")?.as_str()?)?;
                if amount == 0 { return None; } // reject zero transfer
                Some(Brc20Operation::Transfer { ticker, amount })
            }
            _ => None,
        }
    }

    pub fn process_operation(&self, operation: &Brc20Operation, inscription_id: &str, owner: &str) -> Result<()> {
        match operation {
            Brc20Operation::Deploy { ticker, max_supply, limit_per_mint, decimals, self_mint } => {
                // Normalize ticker to lowercase for storage/lookup
                let ticker = ticker.to_lowercase();
                let tickers_table = Brc20Tickers::new();
                if tickers_table.get(&ticker).is_some() { return Ok(()); }
                let new_ticker = Ticker {
                    name: ticker.clone(), max_supply: *max_supply, current_supply: 0,
                    limit_per_mint: *limit_per_mint, decimals: *decimals,
                    deploy_inscription_id: inscription_id.to_string(),
                    is_self_mint: *self_mint,
                    burned_supply: 0,
                };
                tickers_table.set(&ticker, &serde_json::to_vec(&new_ticker)?);
            }
            Brc20Operation::Mint { ticker, amount } => {
                let ticker = ticker.to_lowercase();
                // Reject zero amount
                if *amount == 0 { return Ok(()); }
                let tickers_table = Brc20Tickers::new();
                if let Some(ticker_data) = tickers_table.get(&ticker) {
                    let mut ticker_entry: Ticker = serde_json::from_slice(&ticker_data)?;
                    if *amount > ticker_entry.limit_per_mint { return Ok(()); }
                    if ticker_entry.current_supply >= ticker_entry.max_supply { return Ok(()); }

                    // Clamp amount to remaining supply (OPI partial mint behavior)
                    let remaining = ticker_entry.max_supply - ticker_entry.current_supply;
                    let mint_amount = (*amount).min(remaining);

                    ticker_entry.current_supply += mint_amount;
                    tickers_table.set(&ticker, &serde_json::to_vec(&ticker_entry)?);
                    let balances_table = Brc20Balances::new();
                    let mut balance = balances_table.get(owner, &ticker)
                        .and_then(|d| serde_json::from_slice(&d).ok())
                        .unwrap_or_else(|| Balance::new(ticker.clone()));
                    balance.total_balance += mint_amount;
                    balance.available_balance += mint_amount;
                    balances_table.set(owner, &ticker, &serde_json::to_vec(&balance)?);
                }
            }
            Brc20Operation::Transfer { ticker, amount } => {
                let ticker = ticker.to_lowercase();
                // Reject zero amount
                if *amount == 0 { return Ok(()); }
                let balances_table = Brc20Balances::new();
                let balance_data = match balances_table.get(owner, &ticker) {
                    Some(data) => data,
                    None => return Ok(()),
                };
                let mut balance: Balance = match serde_json::from_slice(&balance_data) {
                    Ok(b) => b,
                    Err(_) => return Ok(()),
                };
                if balance.available_balance < *amount { return Ok(()); }
                balance.available_balance -= amount;
                balances_table.set(owner, &ticker, &serde_json::to_vec(&balance)?);
                let transfer_info = TransferInfo { ticker: ticker.clone(), amount: *amount, sender: owner.to_string() };
                Brc20TransferableInscriptions::new().set(inscription_id, &serde_json::to_vec(&transfer_info)?);
            }
        }
        Ok(())
    }

    /// Claim a transfer — simple wallet-to-wallet (backwards-compatible).
    pub fn claim_transfer(&self, new_owner: &str, transfer_info: &TransferInfo) -> Result<()> {
        self.resolve_transfer(TransferDestination::Wallet(new_owner.to_string()), transfer_info, 0)
    }

    /// Resolve a transfer based on destination type (OPI-compatible).
    ///
    /// Handles four cases per OPI reference:
    /// - Wallet: normal transfer to recipient
    /// - Burn: OP_RETURN output, tokens destroyed
    /// - Brc20ProgDeposit: BRC20-PROG OP_RETURN, tokens deposited to prog module
    /// - SentAsFee: inscription spent as tx fee, tokens returned to sender
    pub fn resolve_transfer(&self, destination: TransferDestination, transfer_info: &TransferInfo, height: u32) -> Result<()> {
        let balances_table = Brc20Balances::new();
        let tickers_table = Brc20Tickers::new();
        let ticker = &transfer_info.ticker;

        match destination {
            TransferDestination::Wallet(ref new_owner) => {
                // Add to recipient's balance
                let mut new_owner_balance = balances_table.get(new_owner, ticker)
                    .and_then(|d| serde_json::from_slice(&d).ok())
                    .unwrap_or_else(|| Balance::new(ticker.clone()));
                new_owner_balance.total_balance += transfer_info.amount;
                new_owner_balance.available_balance += transfer_info.amount;
                balances_table.set(new_owner, ticker, &serde_json::to_vec(&new_owner_balance)?);

                // Deduct from sender's total_balance (available was already reduced at inscribe)
                if let Some(sender_data) = balances_table.get(&transfer_info.sender, ticker) {
                    let mut sender_balance: Balance = serde_json::from_slice(&sender_data)?;
                    sender_balance.total_balance -= transfer_info.amount;
                    balances_table.set(&transfer_info.sender, ticker, &serde_json::to_vec(&sender_balance)?);
                }
            }
            TransferDestination::Burn => {
                // OP_RETURN: reduce sender's total_balance, increment ticker's burned_supply
                if let Some(sender_data) = balances_table.get(&transfer_info.sender, ticker) {
                    let mut sender_balance: Balance = serde_json::from_slice(&sender_data)?;
                    sender_balance.total_balance -= transfer_info.amount;
                    balances_table.set(&transfer_info.sender, ticker, &serde_json::to_vec(&sender_balance)?);
                }
                if let Some(ticker_data) = tickers_table.get(ticker) {
                    let mut ticker_entry: Ticker = serde_json::from_slice(&ticker_data)?;
                    ticker_entry.burned_supply += transfer_info.amount;
                    tickers_table.set(ticker, &serde_json::to_vec(&ticker_entry)?);
                }
            }
            TransferDestination::Brc20ProgDeposit => {
                // BRC20-PROG OP_RETURN: phase-gated deposit
                // Before phase 1 or for tickers < 6 bytes before phase 2: treat as burn
                let ticker_len = ticker.as_bytes().len();
                let should_burn = height < BRC20_PROG_PHASE_ONE_HEIGHT
                    || (ticker_len < 6 && height < BRC20_PROG_ALL_TICKERS_HEIGHT);

                if should_burn {
                    // Burn: same as OP_RETURN
                    if let Some(sender_data) = balances_table.get(&transfer_info.sender, ticker) {
                        let mut sender_balance: Balance = serde_json::from_slice(&sender_data)?;
                        sender_balance.total_balance -= transfer_info.amount;
                        balances_table.set(&transfer_info.sender, ticker, &serde_json::to_vec(&sender_balance)?);
                    }
                    if let Some(ticker_data) = tickers_table.get(ticker) {
                        let mut ticker_entry: Ticker = serde_json::from_slice(&ticker_data)?;
                        ticker_entry.burned_supply += transfer_info.amount;
                        tickers_table.set(ticker, &serde_json::to_vec(&ticker_entry)?);
                    }
                } else {
                    // Deposit to BRC20-PROG: move tokens to the prog address balance
                    let mut prog_balance = balances_table.get(BRC20_PROG_OP_RETURN_PKSCRIPT, ticker)
                        .and_then(|d| serde_json::from_slice(&d).ok())
                        .unwrap_or_else(|| Balance::new(ticker.clone()));
                    prog_balance.total_balance += transfer_info.amount;
                    prog_balance.available_balance += transfer_info.amount;
                    balances_table.set(BRC20_PROG_OP_RETURN_PKSCRIPT, ticker, &serde_json::to_vec(&prog_balance)?);

                    // Deduct from sender
                    if let Some(sender_data) = balances_table.get(&transfer_info.sender, ticker) {
                        let mut sender_balance: Balance = serde_json::from_slice(&sender_data)?;
                        sender_balance.total_balance -= transfer_info.amount;
                        balances_table.set(&transfer_info.sender, ticker, &serde_json::to_vec(&sender_balance)?);
                    }
                }
            }
            TransferDestination::SentAsFee => {
                // Inscription spent as fee: return tokens to sender's available_balance
                if let Some(sender_data) = balances_table.get(&transfer_info.sender, ticker) {
                    let mut sender_balance: Balance = serde_json::from_slice(&sender_data)?;
                    sender_balance.available_balance += transfer_info.amount;
                    balances_table.set(&transfer_info.sender, ticker, &serde_json::to_vec(&sender_balance)?);
                }
                // Note: total_balance is unchanged (it was never deducted at inscribe,
                // only available_balance was reduced)
            }
        }
        Ok(())
    }

    /// Determine transfer destination from a pkscript hex string.
    /// Used by process_brc20_transfers to classify the output.
    pub fn classify_destination(pkscript_hex: &str, sent_as_fee: bool) -> TransferDestination {
        if sent_as_fee {
            return TransferDestination::SentAsFee;
        }
        if pkscript_hex == BRC20_PROG_OP_RETURN_PKSCRIPT {
            return TransferDestination::Brc20ProgDeposit;
        }
        if pkscript_hex.starts_with(OP_RETURN_PKSCRIPT) {
            return TransferDestination::Burn;
        }
        // Empty pkscript (should not happen, but treat as fee return)
        if pkscript_hex.is_empty() {
            return TransferDestination::SentAsFee;
        }
        // Normal address
        TransferDestination::Wallet(pkscript_hex.to_string())
    }
}
