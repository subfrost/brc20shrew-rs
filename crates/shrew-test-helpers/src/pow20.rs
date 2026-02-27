use bitcoin::{Block, Transaction};
use sha2::{Sha256, Digest};
use crate::blocks::{create_coinbase_transaction, create_block_with_txs};
use crate::transactions::create_inscription_transaction;

/// Create a block with a POW20 deploy inscription
pub fn create_pow20_deploy_block(
    ticker: &str,
    max: &str,
    lim: &str,
    diff: u32,
    start: u32,
) -> (Block, Transaction) {
    let content = format!(
        r#"{{"p":"pow-20","op":"deploy","tick":"{}","max":"{}","lim":"{}","diff":"{}","start":"{}"}}"#,
        ticker, max, lim, diff, start,
    );
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(0), tx.clone()]);
    (block, tx)
}

/// Create a block with a POW20 mint inscription
pub fn create_pow20_mint_block(
    ticker: &str,
    amt: &str,
    nonce: &str,
    height: u32,
) -> (Block, Transaction) {
    let content = format!(
        r#"{{"p":"pow-20","op":"mint","tick":"{}","amt":"{}","nonce":"{}"}}"#,
        ticker, amt, nonce,
    );
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);
    (block, tx)
}

/// Brute-force find a valid nonce for a given inscription_id and difficulty
pub fn find_valid_nonce(inscription_id: &str, difficulty: u32) -> String {
    for i in 0u64.. {
        let nonce = format!("{}", i);
        let pow_input = format!("{}{}", inscription_id, nonce);
        let hash = Sha256::digest(pow_input.as_bytes());
        if check_leading_zero_bits(&hash, difficulty) {
            return nonce;
        }
    }
    unreachable!()
}

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
