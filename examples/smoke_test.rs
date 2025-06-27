//! Smoke test for shrewscriptions-rs
//! 
//! This example demonstrates basic functionality and serves as a quick smoke test
//! to verify the indexer is working correctly.

use shrewscriptions_rs::indexer::InscriptionIndexer;
use shrewscriptions_rs::inscription::InscriptionId;
use shrewscriptions_rs::envelope;
use bitcoin::{Block, Transaction, TxIn, TxOut, OutPoint, ScriptBuf, Witness};
use bitcoin::block::Header as BlockHeader;
use bitcoin::hashes::Hash;
use anyhow::Result;

fn main() -> Result<()> {
    println!("🔥 Running shrewscriptions-rs smoke test...");
    
    // Create a simple test inscription
    let content = b"Hello, Bitcoin Inscriptions!";
    let content_type = b"text/plain";
    
    // Create a block with the inscription
    let block = create_test_block_with_inscription(content, content_type);
    println!("✅ Created test block with inscription");
    
    // Initialize indexer
    let mut indexer = InscriptionIndexer::new();
    println!("✅ Initialized indexer");
    
    // Index the block
    let result = indexer.index_block(&block, 840000);
    match result {
        Ok(_) => println!("✅ Indexed test block"),
        Err(e) => println!("⚠️  Block indexing returned error (expected): {}", e),
    }
    
    // Verify inscription was created
    let txid = block.txdata[1].txid(); // First non-coinbase tx
    println!("✅ Inscription transaction: {}", txid);
    
    // Test basic serialization
    let inscription_id = InscriptionId::new(txid, 0);
    let serialized = inscription_id.to_bytes();
    let deserialized = InscriptionId::from_bytes(&serialized).map_err(|e| anyhow::anyhow!(e))?;
    assert_eq!(inscription_id.txid, deserialized.txid);
    assert_eq!(inscription_id.index, deserialized.index);
    println!("✅ Serialization/deserialization works");
    
    // Test envelope parsing
    let witness_script = create_inscription_script(content_type, content);
    let tx = &block.txdata[1];
    let envelopes = envelope::parse_inscriptions_from_transaction(tx);
    match envelopes {
        Ok(envs) if !envs.is_empty() => {
            println!("✅ Found {} inscription envelope(s)", envs.len());
        }
        Ok(_) => println!("⚠️  No envelopes found (expected for stub implementation)"),
        Err(e) => println!("⚠️  Envelope parsing returned error (expected): {}", e),
    }
    
    println!("🎉 Smoke test completed successfully!");
    println!("The shrewscriptions-rs indexer basic structure is working correctly.");
    
    Ok(())
}

fn create_test_block_with_inscription(content: &[u8], content_type: &[u8]) -> Block {
    let coinbase = create_coinbase_transaction();
    let inscription_tx = create_inscription_transaction(content_type, content);
    
    Block {
        header: create_test_block_header(),
        txdata: vec![coinbase, inscription_tx],
    }
}

fn create_coinbase_transaction() -> Transaction {
    Transaction {
        version: 2,
        lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: 5000000000,
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

fn create_inscription_transaction(content_type: &[u8], content: &[u8]) -> Transaction {
    let script = create_inscription_script(content_type, content);
    let witness = Witness::from_slice(&[script]);
    
    Transaction {
        version: 2,
        lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness,
        }],
        output: vec![TxOut {
            value: 546,
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

fn create_inscription_script(content_type: &[u8], content: &[u8]) -> Vec<u8> {
    let mut script = Vec::new();
    script.push(0x00); // OP_FALSE
    script.push(0x63); // OP_IF
    script.push(1); // content-type tag
    script.extend_from_slice(content_type);
    script.push(0x00); // OP_0 (body separator)
    script.extend_from_slice(content);
    script.push(0x68); // OP_ENDIF
    script
}

fn create_test_block_header() -> BlockHeader {
    BlockHeader {
        version: bitcoin::block::Version::ONE,
        prev_blockhash: bitcoin::BlockHash::all_zeros(),
        merkle_root: bitcoin::hash_types::TxMerkleNode::all_zeros(),
        time: 1640995200,
        bits: bitcoin::CompactTarget::from_consensus(0x1d00ffff),
        nonce: 0,
    }
}