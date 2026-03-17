use crate::tables::*;
use shrew_support::inscription::InscriptionEntry;
use shrew_support::utils::get_address_from_txout;
use shrew_ord::tables::{
    SEQUENCE_TO_INSCRIPTION_ENTRY, INSCRIPTION_CONTENT, GLOBAL_SEQUENCE_COUNTER,
    OUTPOINT_TO_INSCRIPTIONS,
};
use bitcoin::{Block, Network, Transaction};
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

const POW20_STARTING_BLOCK: u32 = shrew_support::constants::POW20_STARTING_BLOCK;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pow20Ticker {
    pub name: String,
    pub max_supply: u64,
    pub current_supply: u64,
    pub limit_per_mint: u64,
    pub decimals: u8,
    pub difficulty: u32,
    pub starting_block_height: u32,
    pub deploy_inscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pow20Balance {
    pub ticker: String,
    pub total_balance: u64,
    pub available_balance: u64,
}

impl Pow20Balance {
    pub fn new(ticker: String) -> Self {
        Self { ticker, total_balance: 0, available_balance: 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pow20TransferInfo {
    pub ticker: String,
    pub amount: u64,
    pub sender: String,
}

pub struct Pow20Indexer;

impl Pow20Indexer {
    pub fn new() -> Self { Self }

    pub fn index_block(&self, block: &Block, height: u32) {
        let network = Network::Bitcoin;

        // Process transfers first
        for tx in &block.txdata {
            self.process_pow20_transfers(tx, network);
        }

        // Then process new inscriptions
        let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
        if seq_bytes.is_empty() { return; }
        let max_seq = u32::from_le_bytes(seq_bytes[..4].try_into().unwrap_or([0; 4]));

        for seq in 1..=max_seq {
            let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq.to_le_bytes().to_vec()).get();
            if entry_bytes.is_empty() { continue; }
            let entry = match InscriptionEntry::from_bytes(&entry_bytes) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.height != height { continue; }
            if entry.number < 0 { continue; }

            match &entry.content_type {
                Some(ct) if ct.starts_with("text/plain") || ct.starts_with("application/json") => {}
                _ => continue,
            }

            let inscription_id_str = entry.id.to_string();
            let content_bytes = INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).get();
            if content_bytes.is_empty() { continue; }

            let content_str = match std::str::from_utf8(&content_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let json: serde_json::Value = match serde_json::from_str(content_str) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let _p = match json.get("p").and_then(|v| v.as_str()) {
                Some("pow-20") => "pow-20",
                _ => continue,
            };

            let op = match json.get("op").and_then(|v| v.as_str()) {
                Some(op) => op,
                None => continue,
            };

            // Get owner address from first output of the transaction
            // We need the tx - find it in the block
            let owner = block.txdata.iter()
                .find(|tx| tx.txid() == entry.id.txid)
                .and_then(|tx| tx.output.get(0))
                .and_then(|out| get_address_from_txout(out, network))
                .map(|a| a.to_string());

            let owner = match owner {
                Some(o) => o,
                None => continue,
            };

            match op {
                "deploy" => self.process_deploy(&json, &inscription_id_str, height),
                "mint" => self.process_mint(&json, &inscription_id_str, &owner, height),
                "transfer" => self.process_transfer(&json, &inscription_id_str, &owner),
                _ => {}
            }
        }
    }

    fn process_deploy(&self, json: &serde_json::Value, inscription_id: &str, _height: u32) {
        let ticker = match json.get("tick").and_then(|v| v.as_str()) {
            Some(t) if t.len() <= 4 => t,
            _ => return,
        };
        let max_supply = match json.get("max").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()) {
            Some(n) => n,
            None => return,
        };
        let limit = match json.get("lim").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()) {
            Some(n) => n,
            None => return,
        };
        let decimals = json.get("dec").and_then(|v| v.as_str()).and_then(|s| s.parse::<u8>().ok()).unwrap_or(18);
        let difficulty = match json.get("diff").and_then(|v| v.as_str()).and_then(|s| s.parse::<u32>().ok()) {
            Some(d) => d,
            None => return,
        };
        let starting_block = json.get("start").and_then(|v| v.as_str()).and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(POW20_STARTING_BLOCK);

        // First deploy wins
        let existing = POW20_TICKERS.select(&ticker.to_lowercase().as_bytes().to_vec()).get();
        if !existing.is_empty() { return; }

        let ticker_entry = Pow20Ticker {
            name: ticker.to_lowercase(),
            max_supply, current_supply: 0, limit_per_mint: limit, decimals, difficulty,
            starting_block_height: starting_block,
            deploy_inscription_id: inscription_id.to_string(),
        };

        let bytes = serde_json::to_vec(&ticker_entry).unwrap_or_default();
        POW20_TICKERS.select(&ticker.to_lowercase().as_bytes().to_vec()).set(Arc::new(bytes));
    }

    fn process_mint(&self, json: &serde_json::Value, inscription_id: &str, owner: &str, height: u32) {
        let ticker = match json.get("tick").and_then(|v| v.as_str()) {
            Some(t) => t.to_lowercase(),
            None => return,
        };
        let amount = match json.get("amt").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()) {
            Some(n) => n,
            None => return,
        };
        let nonce = match json.get("nonce").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return,
        };

        // Load ticker
        let ticker_data = POW20_TICKERS.select(&ticker.as_bytes().to_vec()).get();
        if ticker_data.is_empty() { return; }
        let mut ticker_entry: Pow20Ticker = match serde_json::from_slice(&ticker_data) {
            Ok(t) => t,
            Err(_) => return,
        };

        // Check starting block
        if height < ticker_entry.starting_block_height { return; }

        // Check limits
        if amount > ticker_entry.limit_per_mint { return; }
        if ticker_entry.current_supply + amount > ticker_entry.max_supply { return; }

        // Verify proof of work: SHA256(inscription_id + nonce) must have `difficulty` leading zero bits
        let pow_input = format!("{}{}", inscription_id, nonce);
        let hash = Sha256::digest(pow_input.as_bytes());
        if !check_leading_zero_bits(&hash, ticker_entry.difficulty) { return; }

        // Mint successful
        ticker_entry.current_supply += amount;
        let ticker_bytes = serde_json::to_vec(&ticker_entry).unwrap_or_default();
        POW20_TICKERS.select(&ticker.as_bytes().to_vec()).set(Arc::new(ticker_bytes));

        // Update balance
        let key = format!("{}:{}", owner, ticker);
        let balance_data = POW20_BALANCES.select(&key.as_bytes().to_vec()).get();
        let mut balance: Pow20Balance = if balance_data.is_empty() {
            Pow20Balance::new(ticker.clone())
        } else {
            serde_json::from_slice(&balance_data).unwrap_or_else(|_| Pow20Balance::new(ticker.clone()))
        };
        balance.total_balance += amount;
        balance.available_balance += amount;
        let balance_bytes = serde_json::to_vec(&balance).unwrap_or_default();
        POW20_BALANCES.select(&key.as_bytes().to_vec()).set(Arc::new(balance_bytes));
    }

    fn process_transfer(&self, json: &serde_json::Value, inscription_id: &str, owner: &str) {
        let ticker = match json.get("tick").and_then(|v| v.as_str()) {
            Some(t) => t.to_lowercase(),
            None => return,
        };
        let amount = match json.get("amt").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()) {
            Some(n) => n,
            None => return,
        };

        let key = format!("{}:{}", owner, ticker);
        let balance_data = POW20_BALANCES.select(&key.as_bytes().to_vec()).get();
        if balance_data.is_empty() { return; }
        let mut balance: Pow20Balance = match serde_json::from_slice(&balance_data) {
            Ok(b) => b,
            Err(_) => return,
        };
        if balance.available_balance < amount { return; }

        balance.available_balance -= amount;
        let balance_bytes = serde_json::to_vec(&balance).unwrap_or_default();
        POW20_BALANCES.select(&key.as_bytes().to_vec()).set(Arc::new(balance_bytes));

        let transfer_info = Pow20TransferInfo { ticker, amount, sender: owner.to_string() };
        let transfer_bytes = serde_json::to_vec(&transfer_info).unwrap_or_default();
        POW20_TRANSFERABLE.select(&inscription_id.as_bytes().to_vec()).set(Arc::new(transfer_bytes));
    }

    fn process_pow20_transfers(&self, tx: &Transaction, network: Network) {
        for input in &tx.input {
            let outpoint_bytes: Vec<u8> = input.previous_output.txid.as_byte_array()
                .iter().chain(input.previous_output.vout.to_le_bytes().iter()).copied().collect();
            let inscription_sequences = OUTPOINT_TO_INSCRIPTIONS.select(&outpoint_bytes).get_list();
            if inscription_sequences.is_empty() { continue; }

            for seq_bytes in inscription_sequences {
                let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
                if entry_bytes.is_empty() { continue; }
                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    let inscription_id_str = entry.id.to_string();
                    let transfer_data = POW20_TRANSFERABLE.select(&inscription_id_str.as_bytes().to_vec()).get();
                    if transfer_data.is_empty() { continue; }

                    if let Ok(transfer_info) = serde_json::from_slice::<Pow20TransferInfo>(&transfer_data) {
                        if let Some(first_output) = tx.output.get(0) {
                            if let Some(new_owner) = get_address_from_txout(first_output, network) {
                                self.claim_pow20_transfer(&new_owner.to_string(), &transfer_info);
                                POW20_TRANSFERABLE.select(&inscription_id_str.as_bytes().to_vec()).set(Arc::new(vec![]));
                            }
                        }
                    }
                }
            }
        }
    }

    fn claim_pow20_transfer(&self, new_owner: &str, transfer_info: &Pow20TransferInfo) {
        // Credit new owner
        let key = format!("{}:{}", new_owner, transfer_info.ticker);
        let balance_data = POW20_BALANCES.select(&key.as_bytes().to_vec()).get();
        let mut balance: Pow20Balance = if balance_data.is_empty() {
            Pow20Balance::new(transfer_info.ticker.clone())
        } else {
            serde_json::from_slice(&balance_data).unwrap_or_else(|_| Pow20Balance::new(transfer_info.ticker.clone()))
        };
        balance.total_balance += transfer_info.amount;
        balance.available_balance += transfer_info.amount;
        let balance_bytes = serde_json::to_vec(&balance).unwrap_or_default();
        POW20_BALANCES.select(&key.as_bytes().to_vec()).set(Arc::new(balance_bytes));

        // Debit sender
        let sender_key = format!("{}:{}", transfer_info.sender, transfer_info.ticker);
        let sender_data = POW20_BALANCES.select(&sender_key.as_bytes().to_vec()).get();
        if !sender_data.is_empty() {
            if let Ok(mut sender_balance) = serde_json::from_slice::<Pow20Balance>(&sender_data) {
                sender_balance.total_balance = sender_balance.total_balance.saturating_sub(transfer_info.amount);
                let sender_bytes = serde_json::to_vec(&sender_balance).unwrap_or_default();
                POW20_BALANCES.select(&sender_key.as_bytes().to_vec()).set(Arc::new(sender_bytes));
            }
        }
    }
}

/// Check if hash has at least `difficulty` leading zero bits
fn check_leading_zero_bits(hash: &[u8], difficulty: u32) -> bool {
    let mut remaining = difficulty;
    for byte in hash {
        if remaining == 0 { return true; }
        if remaining >= 8 {
            if *byte != 0 { return false; }
            remaining -= 8;
        } else {
            let mask = 0xFF << (8 - remaining);
            return (*byte & mask) == 0;
        }
    }
    remaining == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn test_leading_zero_bits() {
        assert!(check_leading_zero_bits(&[0x00, 0x00, 0xFF], 16));
        assert!(!check_leading_zero_bits(&[0x00, 0x01, 0xFF], 16));
        assert!(check_leading_zero_bits(&[0x00, 0x0F, 0xFF], 12));
        assert!(!check_leading_zero_bits(&[0x00, 0x1F, 0xFF], 12));
        assert!(check_leading_zero_bits(&[0x00], 8));
        assert!(check_leading_zero_bits(&[0xFF], 0));
    }
}
