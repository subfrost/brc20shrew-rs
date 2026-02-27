use bitcoin::{Block, Transaction, TxIn, TxOut, OutPoint, Witness, ScriptBuf, Sequence};
use bitcoin::blockdata::block::{Header, Version as BlockVersion};
use bitcoin::BlockHash;
use std::str::FromStr;

use crate::state::get_test_address;
use crate::transactions::create_inscription_transaction;

/// Create a coinbase transaction for the given height
pub fn create_coinbase_transaction(height: u32) -> Transaction {
    let script_pubkey = get_test_address(0).script_pubkey();
    let coinbase_input = TxIn {
        previous_output: OutPoint::default(),
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };
    let coinbase_output = TxOut {
        value: 50_000_000,
        script_pubkey,
    };
    let locktime = bitcoin::absolute::LockTime::from_height(height).unwrap_or(bitcoin::absolute::LockTime::ZERO);
    Transaction {
        version: 2,
        lock_time: locktime,
        input: vec![coinbase_input],
        output: vec![coinbase_output],
    }
}

/// Create a block with just a coinbase transaction
pub fn create_block_with_coinbase_tx(height: u32) -> Block {
    let coinbase_tx = create_coinbase_transaction(height);
    create_block_with_txs(vec![coinbase_tx])
}

/// Create a block with the given transactions
pub fn create_block_with_txs(txdata: Vec<Transaction>) -> Block {
    let previous_blockhash = BlockHash::from_str(
        "00000000000000000005c3b409b4f17f9b3a97ed46d1a63d3f660d24168b2b3e"
    ).unwrap();
    let merkle_root = bitcoin::hash_types::TxMerkleNode::from_str(
        "4e07408562b4b5a9c0555f0671e0d2b6c5764c1d2a5e97c1d7f36f7c91e4c77a"
    ).unwrap();
    let header = Header {
        version: BlockVersion::from_consensus(1),
        prev_blockhash: previous_blockhash,
        merkle_root,
        time: 1231006505,
        bits: bitcoin::CompactTarget::from_consensus(0x1234),
        nonce: 2083236893,
    };
    Block { header, txdata }
}

/// Create a block with inscription transactions
pub fn create_inscription_block(inscriptions: Vec<(&[u8], &str)>) -> Block {
    let mut block = create_block_with_coinbase_tx(840000);
    for (content, content_type) in inscriptions {
        let inscription_tx = create_inscription_transaction(content, content_type, None);
        block.txdata.push(inscription_tx);
    }
    block
}

/// Create a chain of empty blocks (coinbase only)
pub fn create_test_chain(num_blocks: u32, start_height: u32) -> Vec<Block> {
    (0..num_blocks)
        .map(|i| create_block_with_coinbase_tx(start_height + i))
        .collect()
}
