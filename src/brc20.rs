// Chadson v69.0.0: This file defines the core data structures and logic for BRC20 indexing.
// It includes the Brc20Operation enum, Ticker, Balance, and the new TransferInfo struct.
// The Brc20Indexer struct contains the core processing logic for deploy, mint, and transfer operations.

use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};

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
        Self {
            ticker,
            total_balance: 0,
            available_balance: 0,
        }
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
    pub fn new() -> Self {
        Self
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
                let decimals = json
                    .get("dec")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u8>().ok())
                    .unwrap_or(18);
                Some(Brc20Operation::Deploy {
                    ticker: ticker.to_string(),
                    max_supply,
                    limit_per_mint,
                    decimals,
                })
            }
            "mint" => {
                let amount = json.get("amt")?.as_str()?.parse::<u64>().ok()?;
                Some(Brc20Operation::Mint {
                    ticker: ticker.to_string(),
                    amount,
                })
            }
            "transfer" => {
                let amount = json.get("amt")?.as_str()?.parse::<u64>().ok()?;
                Some(Brc20Operation::Transfer {
                    ticker: ticker.to_string(),
                    amount,
                })
            }
            _ => None,
        }
    }

    pub fn process_operation(&self, operation: &Brc20Operation, inscription_id: &str, owner: &str) -> Result<()> {
        match operation {
            Brc20Operation::Deploy {
                ticker,
                max_supply,
                limit_per_mint,
                decimals,
            } => {
                let tickers_table = Brc20Tickers::new();
                if tickers_table.get(ticker).is_some() {
                    return Ok(()); // Ticker already exists
                }

                let new_ticker = Ticker {
                    name: ticker.clone(),
                    max_supply: *max_supply,
                    current_supply: 0,
                    limit_per_mint: *limit_per_mint,
                    decimals: *decimals,
                    deploy_inscription_id: inscription_id.to_string(),
                };

                let ticker_bytes = serde_json::to_vec(&new_ticker)?;
                tickers_table.set(ticker, &ticker_bytes);
            }
            Brc20Operation::Mint { ticker, amount } => {
                let tickers_table = Brc20Tickers::new();
                if let Some(ticker_data) = tickers_table.get(ticker) {
                    let mut ticker_entry: Ticker = serde_json::from_slice(&ticker_data)?;
                    
                    if *amount > ticker_entry.limit_per_mint || ticker_entry.current_supply + amount > ticker_entry.max_supply {
                        return Ok(()); // Exceeds limit or max supply
                    }

                    ticker_entry.current_supply += amount;
                    let ticker_bytes = serde_json::to_vec(&ticker_entry)?;
                    tickers_table.set(ticker, &ticker_bytes);

                    // Update owner's balance
                    let balances_table = Brc20Balances::new();
                    let mut balance = balances_table.get(owner, ticker)
                        .and_then(|d| serde_json::from_slice(&d).ok())
                        .unwrap_or_else(|| Balance::new(ticker.clone()));
                    
                    balance.total_balance += amount;
                    balance.available_balance += amount;

                    let balance_bytes = serde_json::to_vec(&balance)?;
                    balances_table.set(owner, ticker, &balance_bytes);
                }
            }
            Brc20Operation::Transfer { ticker, amount } => {
                // For now, we only handle the inscription of a transfer.
                // The actual transfer of the UTXO will be handled separately.
                let balances_table = Brc20Balances::new();
                let balance_data = match balances_table.get(owner, ticker) {
                    Some(data) => data,
                    None => return Ok(()), // No balance, do nothing.
                };

                let mut balance: Balance = match serde_json::from_slice(&balance_data) {
                    Ok(b) => b,
                    Err(_) => return Ok(()), // Failed to parse, do nothing.
                };

                if balance.available_balance < *amount {
                    return Ok(()); // Not enough available balance, do nothing.
                }

                // Decrement available balance and save
                balance.available_balance -= *amount;
                let balance_bytes = serde_json::to_vec(&balance)?;
                balances_table.set(owner, ticker, &balance_bytes);

                // Create the transferable inscription record
                let transfer_info = TransferInfo {
                    ticker: ticker.clone(),
                    amount: *amount,
                    sender: owner.to_string(),
                };
                let transfer_info_bytes = serde_json::to_vec(&transfer_info)?;
                let transferable_table = Brc20TransferableInscriptions::new();
                transferable_table.set(inscription_id, &transfer_info_bytes);
            }
        }
        Ok(())
    }
    pub fn claim_transfer(&self, new_owner: &str, transfer_info: &TransferInfo) -> Result<()> {
        let balances_table = Brc20Balances::new();

        // Credit the new owner
        let mut new_owner_balance = balances_table
            .get(new_owner, &transfer_info.ticker)
            .and_then(|d| serde_json::from_slice(&d).ok())
            .unwrap_or_else(|| Balance::new(transfer_info.ticker.clone()));
        new_owner_balance.total_balance += transfer_info.amount;
        new_owner_balance.available_balance += transfer_info.amount;
        let new_owner_balance_bytes = serde_json::to_vec(&new_owner_balance)?;
        balances_table.set(new_owner, &transfer_info.ticker, &new_owner_balance_bytes);

        // Debit the original sender
        if let Some(sender_balance_data) = balances_table.get(&transfer_info.sender, &transfer_info.ticker) {
            let mut sender_balance: Balance = serde_json::from_slice(&sender_balance_data)?;
            sender_balance.total_balance -= transfer_info.amount;
            // Note: available_balance was already debited at inscription time.
            let sender_balance_bytes = serde_json::to_vec(&sender_balance)?;
            balances_table.set(&transfer_info.sender, &transfer_info.ticker, &sender_balance_bytes);
        }

        Ok(())
    }
}