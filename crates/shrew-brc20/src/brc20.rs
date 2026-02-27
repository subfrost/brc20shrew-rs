use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::tables::*;
use shrew_support::inscription::{InscriptionEntry, InscriptionId};
use shrew_support::utils::get_address_from_txout;
use shrew_ord::tables::{
    INSCRIPTION_ID_TO_SEQUENCE, SEQUENCE_TO_INSCRIPTION_ENTRY,
    OUTPOINT_TO_INSCRIPTIONS, INSCRIPTION_CONTENT,
};
use bitcoin_hashes::Hash;
use bitcoin::{Block, Network, Transaction};
use metashrew_support::index_pointer::KeyValuePointer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Brc20Operation {
    Deploy {
        ticker: String,
        max_supply: u64,
        limit_per_mint: u64,
        decimals: u8,
    },
    Mint {
        ticker: String,
        amount: u64,
    },
    Transfer {
        ticker: String,
        amount: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ticker {
    pub name: String,
    pub max_supply: u64,
    pub current_supply: u64,
    pub limit_per_mint: u64,
    pub decimals: u8,
    pub deploy_inscription_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Balance {
    pub ticker: String,
    pub total_balance: u64,
    pub available_balance: u64,
}

impl Balance {
    pub fn new(ticker: String) -> Self {
        Self { ticker, total_balance: 0, available_balance: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferInfo {
    pub ticker: String,
    pub amount: u64,
    pub sender: String,
}

pub struct Brc20Indexer;

impl Brc20Indexer {
    pub fn new() -> Self { Self }

    /// Process an entire block for BRC20 operations.
    pub fn process_block(&self, block: &Block, _height: u32) {
        let network = Network::Bitcoin;
        for tx in &block.txdata {
            // Check for BRC20 transfer claims (spending transferable inscriptions)
            self.process_brc20_transfers(tx, network);

            // Check for new BRC20 operations in inscriptions
            self.process_brc20_inscriptions(tx, network);
        }
    }

    fn process_brc20_inscriptions(&self, tx: &Transaction, network: Network) {
        // Look up inscriptions created in this transaction
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

            // Check content type
            let _content_type = match &entry.content_type {
                Some(ct) if ct.starts_with("text/plain") || ct.starts_with("application/json") => ct.clone(),
                _ => continue,
            };

            // Get content
            let inscription_id_str = inscription_id.to_string();
            let content_bytes = INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).get();
            if content_bytes.is_empty() { continue; }

            // Parse BRC20 operation
            if let Some(operation) = self.parse_operation(&content_bytes) {
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

    pub fn parse_operation(&self, content: &[u8]) -> Option<Brc20Operation> {
        let content_str = std::str::from_utf8(content).ok()?;
        let json: serde_json::Value = serde_json::from_str(content_str).ok()?;
        let op = json.get("op")?.as_str()?;
        let ticker = json.get("tick")?.as_str()?;

        match op {
            "deploy" => {
                let max_supply = json.get("max")?.as_str()?.parse::<u64>().ok()?;
                let limit_per_mint = json.get("lim")?.as_str()?.parse::<u64>().ok()?;
                let decimals = json.get("dec").and_then(|v| v.as_str()).and_then(|s| s.parse::<u8>().ok()).unwrap_or(18);
                Some(Brc20Operation::Deploy { ticker: ticker.to_string(), max_supply, limit_per_mint, decimals })
            }
            "mint" => {
                let amount = json.get("amt")?.as_str()?.parse::<u64>().ok()?;
                Some(Brc20Operation::Mint { ticker: ticker.to_string(), amount })
            }
            "transfer" => {
                let amount = json.get("amt")?.as_str()?.parse::<u64>().ok()?;
                Some(Brc20Operation::Transfer { ticker: ticker.to_string(), amount })
            }
            _ => None,
        }
    }

    pub fn process_operation(&self, operation: &Brc20Operation, inscription_id: &str, owner: &str) -> Result<()> {
        match operation {
            Brc20Operation::Deploy { ticker, max_supply, limit_per_mint, decimals } => {
                let tickers_table = Brc20Tickers::new();
                if tickers_table.get(ticker).is_some() { return Ok(()); }
                let new_ticker = Ticker {
                    name: ticker.clone(), max_supply: *max_supply, current_supply: 0,
                    limit_per_mint: *limit_per_mint, decimals: *decimals,
                    deploy_inscription_id: inscription_id.to_string(),
                };
                tickers_table.set(ticker, &serde_json::to_vec(&new_ticker)?);
            }
            Brc20Operation::Mint { ticker, amount } => {
                let tickers_table = Brc20Tickers::new();
                if let Some(ticker_data) = tickers_table.get(ticker) {
                    let mut ticker_entry: Ticker = serde_json::from_slice(&ticker_data)?;
                    if *amount > ticker_entry.limit_per_mint || ticker_entry.current_supply + amount > ticker_entry.max_supply {
                        return Ok(());
                    }
                    ticker_entry.current_supply += amount;
                    tickers_table.set(ticker, &serde_json::to_vec(&ticker_entry)?);
                    let balances_table = Brc20Balances::new();
                    let mut balance = balances_table.get(owner, ticker)
                        .and_then(|d| serde_json::from_slice(&d).ok())
                        .unwrap_or_else(|| Balance::new(ticker.clone()));
                    balance.total_balance += amount;
                    balance.available_balance += amount;
                    balances_table.set(owner, ticker, &serde_json::to_vec(&balance)?);
                }
            }
            Brc20Operation::Transfer { ticker, amount } => {
                let balances_table = Brc20Balances::new();
                let balance_data = match balances_table.get(owner, ticker) {
                    Some(data) => data,
                    None => return Ok(()),
                };
                let mut balance: Balance = match serde_json::from_slice(&balance_data) {
                    Ok(b) => b,
                    Err(_) => return Ok(()),
                };
                if balance.available_balance < *amount { return Ok(()); }
                balance.available_balance -= amount;
                balances_table.set(owner, ticker, &serde_json::to_vec(&balance)?);
                let transfer_info = TransferInfo { ticker: ticker.clone(), amount: *amount, sender: owner.to_string() };
                Brc20TransferableInscriptions::new().set(inscription_id, &serde_json::to_vec(&transfer_info)?);
            }
        }
        Ok(())
    }

    pub fn claim_transfer(&self, new_owner: &str, transfer_info: &TransferInfo) -> Result<()> {
        let balances_table = Brc20Balances::new();
        let mut new_owner_balance = balances_table.get(new_owner, &transfer_info.ticker)
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_else(|| Balance::new(transfer_info.ticker.clone()));
        new_owner_balance.total_balance += transfer_info.amount;
        new_owner_balance.available_balance += transfer_info.amount;
        balances_table.set(new_owner, &transfer_info.ticker, &serde_json::to_vec(&new_owner_balance)?);
        if let Some(sender_balance_data) = balances_table.get(&transfer_info.sender, &transfer_info.ticker) {
            let mut sender_balance: Balance = serde_json::from_slice(&sender_balance_data)?;
            sender_balance.total_balance -= transfer_info.amount;
            balances_table.set(&transfer_info.sender, &transfer_info.ticker, &serde_json::to_vec(&sender_balance)?);
        }
        Ok(())
    }
}
