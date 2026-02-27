use bitcoin::{Block, Transaction};
use crate::blocks::{create_coinbase_transaction, create_block_with_txs};
use crate::transactions::create_inscription_transaction;

/// Create a block with an SNS name registration inscription
pub fn create_sns_reg_block(name: &str, height: u32) -> (Block, Transaction) {
    let content = format!(r#"{{"p":"sns","op":"reg","name":"{}"}}"#, name);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);
    (block, tx)
}

/// Create a block with an SNS namespace registration inscription
pub fn create_sns_ns_block(namespace: &str, height: u32) -> (Block, Transaction) {
    let content = format!(r#"{{"p":"sns","op":"ns","ns":"{}"}}"#, namespace);
    let tx = create_inscription_transaction(content.as_bytes(), "application/json", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);
    (block, tx)
}
