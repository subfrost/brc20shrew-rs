use bitcoin::{Block, Transaction};
use crate::blocks::{create_coinbase_transaction, create_block_with_txs};
use crate::transactions::create_inscription_transaction;

/// Create a block with a bitmap inscription
pub fn create_bitmap_inscription_block(bitmap_number: u64, height: u32) -> (Block, Transaction) {
    let content = format!("{}.bitmap", bitmap_number);
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);
    (block, tx)
}

/// Create a block with an invalid bitmap inscription
pub fn create_invalid_bitmap_block(content: &str, height: u32) -> (Block, Transaction) {
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);
    (block, tx)
}
