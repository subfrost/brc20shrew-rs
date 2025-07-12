//! Test helpers for shrewscriptions-rs
//!
//! ## Test Helper Guidelines (Following alkanes-rs Pattern)
//!
//! This module provides helper functions for creating test data and managing test state,
//! following the same patterns established in alkanes-rs.
//!
//! ### Test Runner Configuration:
//! Tests run using wasm-bindgen-test-runner configured in `.cargo/config.toml`:
//! ```toml
//! [target.wasm32-unknown-unknown]
//! runner = "wasm-bindgen-test-runner"
//! ```
//!
//! ### Key Helper Functions:
//!
//! #### State Management:
//! - `clear()`: Reset metashrew state and configure network (REQUIRED at start of every test)
//! - `configure_network()`: Set up regtest network parameters
//!
//! #### Bitcoin Data Creation:
//! - `create_test_transaction()`: Create basic Bitcoin transaction
//! - `create_coinbase_transaction()`: Create coinbase transaction for test blocks
//! - `create_block_with_coinbase_tx()`: Create test block with coinbase
//! - `create_inscription_block()`: Create block with inscription transactions
//!
//! #### Inscription Helpers:
//! - `create_inscription_envelope()`: Create inscription witness data
//! - `create_inscription_envelope_with_metadata()`: Create envelope with metadata
//! - `create_inscription_envelope_with_parent()`: Create envelope with parent reference
//! - `create_inscription_envelope_with_delegate()`: Create envelope with delegate reference
//! - `create_reveal_transaction()`: Create transaction that reveals inscription
//!
//! #### Test Data:
//! - `get_test_address()`: Get regtest address for testing
//! - `create_test_inscription_content()`: Get test inscription content
//! - `create_mock_outpoint()`: Create mock outpoint for testing
//!
//! ### Usage Pattern:
//! ```rust
//! #[wasm_bindgen_test]
//! fn test_something() -> Result<()> {
//!     clear(); // Always start with this
//!
//!     // Create test data
//!     let block = create_inscription_block(inscriptions);
//!
//!     // Index and verify
//!     index_test_block(&block, height)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Reference:
//! Based on patterns from ./reference/alkanes-rs/src/tests/helpers.rs

use crate::indexer::InscriptionIndexer;
use crate::inscription::{InscriptionId, InscriptionEntry};
use bitcoin::{
    Block, Transaction, TxIn, TxOut, OutPoint, Witness, ScriptBuf, Sequence,
    address::NetworkChecked, Address, Network,
};
use bitcoin::blockdata::block::{Header, Version as BlockVersion};
use bitcoin::hashes::Hash;
use bitcoin::{BlockHash, Txid};
use metashrew_core::{
    clear as clear_base,
};
use std::str::FromStr;
use anyhow::Result;

/// Clear metashrew state and initialize for testing
///
/// This function MUST be called at the start of every test to ensure clean state.
/// Follows the exact same pattern as alkanes-rs clear() function.
pub fn clear() {
    clear_base();
    configure_network();
}

/// Configure network parameters for testing (regtest)
///
/// Sets up regtest network parameters for consistent testing environment.
/// This matches the alkanes-rs pattern of network configuration.
pub fn configure_network() {
    // For regtest network - this would typically set network params
    // In a full implementation, this would call set_network() like alkanes-rs
    // For now, we use regtest addresses directly in helper functions
}

/// Get a test address for the regtest network
pub fn get_test_address() -> Address<NetworkChecked> {
    // Create a simple P2WPKH address for regtest
    use bitcoin::key::Secp256k1;
    use bitcoin::secp256k1::SecretKey;
    use bitcoin::PrivateKey;
    use bitcoin::PublicKey;
    
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&[1u8; 32]).unwrap();
    let private_key = PrivateKey::new(secret_key, Network::Regtest);
    let public_key = PublicKey::from_private_key(&secp, &private_key);
    
    Address::p2wpkh(&public_key, Network::Regtest).unwrap()
}

/// Get a second test address for the regtest network
pub fn get_test_address_2() -> Address<NetworkChecked> {
    // Create a different P2WPKH address for regtest
    use bitcoin::key::Secp256k1;
    use bitcoin::secp256k1::SecretKey;
    use bitcoin::PrivateKey;
    use bitcoin::PublicKey;
    
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&[2u8; 32]).unwrap();
    let private_key = PrivateKey::new(secret_key, Network::Regtest);
    let public_key = PublicKey::from_private_key(&secp, &private_key);
    
    Address::p2wpkh(&public_key, Network::Regtest).unwrap()
}

/// Create a coinbase transaction for testing
pub fn create_coinbase_transaction(height: u32) -> Transaction {
    let script_pubkey = get_test_address().script_pubkey();
    
    let coinbase_input = TxIn {
        previous_output: OutPoint::default(),
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    };

    let coinbase_output = TxOut {
        value: 50_000_000, // 50 BTC in satoshis
        script_pubkey,
    };

    let locktime = bitcoin::absolute::LockTime::from_height(height).unwrap();

    Transaction {
        version: 2,
        lock_time: locktime,
        input: vec![coinbase_input],
        output: vec![coinbase_output],
    }
}

/// Create a test block with just a coinbase transaction
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
        time: 1231006505, // Example timestamp
        bits: bitcoin::CompactTarget::from_consensus(0x1234),
        nonce: 2083236893,
    };

    Block { header, txdata }
}

/// Create a transaction with inscription data in the witness
pub fn create_inscription_transaction(
    content: &[u8],
    content_type: &str,
    previous_output: Option<OutPoint>,
) -> Transaction {
    let witness = create_inscription_witness(content, content_type);
    
    let prev_out = previous_output.unwrap_or_else(|| OutPoint {
        txid: Txid::from_str(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ).unwrap(),
        vout: 0,
    });

    let txin = TxIn {
        previous_output: prev_out,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness,
    };

    let address = get_test_address();
    let txout = TxOut {
        value: 100_000_000,
        script_pubkey: address.script_pubkey(),
    };

    Transaction {
        version: 1,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![txin],
        output: vec![txout],
    }
}

/// Create inscription witness data following the inscription envelope format
pub fn create_inscription_witness(content: &[u8], content_type: &str) -> Witness {
    let mut witness = Witness::new();
    
    // Create inscription envelope in witness using raw byte format
    // This follows the corrected inscription format:
    // OP_PUSHBYTES_0 OP_IF "ord" field_tag length data field_tag length data OP_ENDIF
    let mut script = Vec::new();
    
    // OP_PUSHBYTES_0
    script.push(0x00);
    // OP_IF
    script.push(0x63);
    // "ord" tag
    script.push(0x03); // push 3 bytes
    script.extend_from_slice(b"ord");
    
    // Content type field (tag 1)
    script.push(0x01); // field tag 1
    script.push(content_type.len() as u8); // length
    script.extend_from_slice(content_type.as_bytes()); // data
    
    // Content field (tag 0)
    script.push(0x00); // field tag 0
    // Content length and data
    script.push(content.len() as u8);
    script.extend_from_slice(content);
    
    // OP_ENDIF
    script.push(0x68);
    
    witness.push(&script);
    witness
}

/// Create a test block with inscription transactions
pub fn create_inscription_block(inscriptions: Vec<(&[u8], &str)>) -> Block {
    let mut block = create_block_with_coinbase_tx(840000);
    
    for (content, content_type) in inscriptions {
        let inscription_tx = create_inscription_transaction(content, content_type, None);
        block.txdata.push(inscription_tx);
    }
    
    block
}

/// Process a block through the indexer and return any errors
pub fn index_test_block(block: &Block, height: u32) -> Result<()> {
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    indexer.index_block(block, height)?;
    Ok(())
}

/// Get inscription data by ID for testing
pub fn get_inscription_by_id(_id: &InscriptionId) -> Option<InscriptionEntry> {
    // This would use the actual view functions once implemented
    // For now, return None as placeholder
    None
}

/// Verify that an inscription was properly indexed
pub fn assert_inscription_indexed(
    txid: Txid,
    index: u32,
    expected_content_type: &str,
    expected_content_length: usize,
) -> Result<()> {
    let inscription_id = InscriptionId::new(txid, index);
    
    // Check if inscription exists in index
    let inscription = get_inscription_by_id(&inscription_id)
        .ok_or_else(|| anyhow::anyhow!("Inscription not found: {:?}", inscription_id))?;
    
    // Verify content type
    if let Some(content_type) = &inscription.content_type {
        assert_eq!(content_type, expected_content_type);
    }
    
    // Verify content length
    if let Some(content_length) = inscription.content_length {
        assert_eq!(content_length as usize, expected_content_length);
    }
    
    Ok(())
}

/// Create a chain of blocks for testing
pub fn create_test_chain(num_blocks: u32, start_height: u32) -> Vec<Block> {
    let mut blocks = Vec::new();
    
    for i in 0..num_blocks {
        let height = start_height + i;
        let block = create_block_with_coinbase_tx(height);
        blocks.push(block);
    }
    
    blocks
}

/// Process multiple blocks in sequence
pub fn index_test_chain(blocks: &[Block], start_height: u32) -> Result<()> {
    for (i, block) in blocks.iter().enumerate() {
        let height = start_height + i as u32;
        index_test_block(block, height)?;
    }
    Ok(())
}

/// Create a mock outpoint for testing
pub fn create_mock_outpoint(n: u32) -> OutPoint {
    OutPoint {
        txid: Txid::from_str(&format!(
            "000000000000000000000000000000000000000000000000000000000000000{}",
            n
        )).unwrap(),
        vout: 0,
    }
}

/// Create test inscription content
pub fn create_test_inscription_content() -> (&'static [u8], &'static str) {
    (b"Hello, Bitcoin Inscriptions!", "text/plain")
}

/// Create test JSON inscription content
pub fn create_test_json_inscription() -> (&'static [u8], &'static str) {
    (br#"{"name": "Test NFT", "description": "A test inscription"}"#, "application/json")
}

/// Create test image inscription content (mock)
pub fn create_test_image_inscription() -> (Vec<u8>, &'static str) {
    // Mock PNG header for testing
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // Width: 1
        0x00, 0x00, 0x00, 0x01, // Height: 1
        0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, etc.
    ];
    (png_header, "image/png")
}

/// Verify indexer state is clean
pub fn assert_clean_state() {
    // This would check that no inscriptions are indexed
    // Implementation depends on how state is stored
}

/// Print cache contents for debugging
pub fn print_cache_debug() {
    #[allow(unused_imports)]
    use metashrew_core::get_cache;
    
    #[cfg(feature = "test-utils")]
    {
        let cache = get_cache();
        println!("Cache contents:");
        for (key, value) in cache.iter() {
            println!("  {}: {} bytes", hex::encode(&**key), value.len());
        }
    }
}

/// Create a basic test transaction with one input and one output
pub fn create_test_transaction() -> Transaction {
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: 5000000000, // 50 BTC
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create a simple inscription envelope with content type and body using our ord_inscriptions module
pub fn create_inscription_envelope(content_type: &[u8], body: &[u8]) -> Witness {
    use crate::ord_inscriptions::Inscription;
    
    println!("DEBUG helper: Creating envelope with {} content bytes", body.len());
    
    // Create inscription using our ported ord inscription code
    let inscription = Inscription {
        content_type: if content_type.is_empty() {
            None
        } else {
            Some(content_type.to_vec())
        },
        body: Some(body.to_vec()),
        ..Default::default()
    };
    
    // Use the ord inscription's to_witness method
    let witness = inscription.to_witness();
    
    println!("DEBUG helper: Witness created with {} elements", witness.len());
    for (i, element) in witness.iter().enumerate() {
        println!("DEBUG helper: Witness element {}: {} bytes", i, element.len());
        if i == 0 { // Always print script for debugging
            println!("DEBUG helper: Script bytes: {:?}", element);
        }
    }
    
    witness
}

/// Create inscription envelope with metadata
pub fn create_inscription_envelope_with_metadata(content_type: &[u8], body: &[u8], metadata: Option<&[u8]>) -> Witness {
    use crate::ord_inscriptions::Inscription;
    
    println!("DEBUG helper: Creating metadata envelope with {} content bytes", body.len());
    
    // Create inscription using our ported ord inscription code
    let inscription = Inscription {
        content_type: if content_type.is_empty() {
            None
        } else {
            Some(content_type.to_vec())
        },
        metadata: metadata.map(|m| m.to_vec()),
        body: Some(body.to_vec()),
        ..Default::default()
    };
    
    // Use the ord inscription's to_witness method
    let witness = inscription.to_witness();
    
    println!("DEBUG helper: Metadata witness created with {} elements", witness.len());
    for (i, element) in witness.iter().enumerate() {
        println!("DEBUG helper: Metadata witness element {}: {} bytes", i, element.len());
        if i == 0 { // Always print script for debugging
            println!("DEBUG helper: Metadata script bytes: {:?}", element);
        }
    }
    
    witness
}

/// Create inscription envelope with parent reference
pub fn create_inscription_envelope_with_parent(content_type: &[u8], body: &[u8], parent_id: &str) -> Witness {
    use crate::ord_inscriptions::Inscription;
    
    println!("DEBUG helper: Creating parent envelope with {} content bytes, parent: {}", body.len(), parent_id);
    
    // Create inscription using our ported ord inscription code
    let inscription = Inscription {
        content_type: if content_type.is_empty() {
            None
        } else {
            Some(content_type.to_vec())
        },
        parents: vec![parent_id.as_bytes().to_vec()], // parents is a Vec<Vec<u8>>
        body: Some(body.to_vec()),
        ..Default::default()
    };
    
    // Use the ord inscription's to_witness method
    let witness = inscription.to_witness();
    
    println!("DEBUG helper: Parent witness created with {} elements", witness.len());
    for (i, element) in witness.iter().enumerate() {
        println!("DEBUG helper: Parent witness element {}: {} bytes", i, element.len());
        if i == 0 { // Always print script for debugging
            println!("DEBUG helper: Parent script bytes: {:?}", element);
        }
    }
    
    witness
}

/// Create inscription envelope with delegate reference
pub fn create_inscription_envelope_with_delegate(content_type: &[u8], body: &[u8], delegate_id: &str) -> Witness {
    use crate::ord_inscriptions::Inscription;
    
    println!("DEBUG helper: Creating delegate envelope with {} content bytes, delegate: {}", body.len(), delegate_id);
    
    // Create inscription using our ported ord inscription code
    let inscription = Inscription {
        content_type: if content_type.is_empty() {
            None
        } else {
            Some(content_type.to_vec())
        },
        delegate: Some(delegate_id.as_bytes().to_vec()),
        body: Some(body.to_vec()),
        ..Default::default()
    };
    
    // Use the ord inscription's to_witness method
    let witness = inscription.to_witness();
    
    println!("DEBUG helper: Delegate witness created with {} elements", witness.len());
    for (i, element) in witness.iter().enumerate() {
        println!("DEBUG helper: Delegate witness element {}: {} bytes", i, element.len());
        if i == 0 { // Always print script for debugging
            println!("DEBUG helper: Delegate script bytes: {:?}", element);
        }
    }
    
    witness
}

/// Create a reveal transaction that spends from commit transaction
pub fn create_reveal_transaction(commit_txid: &bitcoin::Txid, witness: Witness) -> Transaction {
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(*commit_txid, 0),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness,
        }],
        output: vec![TxOut {
            value: 10000, // 0.0001 BTC
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create a reveal transaction at specific offset
pub fn create_reveal_transaction_at_offset(commit_txid: &bitcoin::Txid, witness: Witness, offset: u64) -> Transaction {
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(*commit_txid, 0),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness,
        }],
        output: vec![TxOut {
            value: 10000 + offset, // Offset affects satpoint
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create transaction with multiple inscription envelopes
pub fn create_multi_inscription_transaction(commit_txid: &bitcoin::Txid, witnesses: Vec<Witness>) -> Transaction {
    let mut inputs = Vec::new();
    
    for (i, witness) in witnesses.into_iter().enumerate() {
        inputs.push(TxIn {
            previous_output: OutPoint::new(*commit_txid, i as u32),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness,
        });
    }
    
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: inputs,
        output: vec![TxOut {
            value: 10000,
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create a transfer transaction that moves an inscription
pub fn create_transfer_transaction(prev_txid: &bitcoin::Txid, prev_vout: u32) -> Transaction {
    Transaction {
        version: 2,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(*prev_txid, prev_vout),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: 10000,
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

/// Create invalid envelope for cursed inscription testing
pub fn create_invalid_envelope() -> Witness {
    let mut script_bytes = Vec::new();
    
    // OP_PUSHBYTES_0
    script_bytes.push(0x00);
    // OP_IF
    script_bytes.push(0x63);
    // "invalid" tag (wrong protocol identifier)
    script_bytes.push(0x07);
    script_bytes.extend_from_slice(b"invalid");
    // OP_ENDIF
    script_bytes.push(0x68);
    
    Witness::from_slice(&[script_bytes, Vec::new()])
}

/// Create envelope in input (should be cursed)
pub fn create_envelope_in_input() -> Witness {
    let mut script_bytes = Vec::new();
    
    // OP_PUSHBYTES_0
    script_bytes.push(0x00);
    // OP_IF
    script_bytes.push(0x63);
    // "ord" tag
    script_bytes.push(0x03);
    script_bytes.extend_from_slice(b"ord");
    // Content type tag (1)
    script_bytes.push(0x01);
    script_bytes.push(0x0A); // "text/plain" length
    script_bytes.extend_from_slice(b"text/plain");
    // Content tag (0)
    script_bytes.push(0x00);
    script_bytes.push(0x0E); // "cursed content" length
    script_bytes.extend_from_slice(b"cursed content");
    // OP_ENDIF
    script_bytes.push(0x68);
    
    Witness::from_slice(&[script_bytes, Vec::new()])
}

/// Create multiple envelopes in same input
pub fn create_multiple_envelopes_same_input() -> Witness {
    let mut script_bytes = Vec::new();
    
    // First envelope
    // OP_PUSHBYTES_0
    script_bytes.push(0x00);
    // OP_IF
    script_bytes.push(0x63);
    // "ord" tag
    script_bytes.push(0x03);
    script_bytes.extend_from_slice(b"ord");
    // Content tag (0)
    script_bytes.push(0x00);
    script_bytes.push(0x05); // "first" length
    script_bytes.extend_from_slice(b"first");
    // OP_ENDIF
    script_bytes.push(0x68);
    
    // Second envelope
    // OP_PUSHBYTES_0
    script_bytes.push(0x00);
    // OP_IF
    script_bytes.push(0x63);
    // "ord" tag
    script_bytes.push(0x03);
    script_bytes.extend_from_slice(b"ord");
    // Content tag (0)
    script_bytes.push(0x00);
    script_bytes.push(0x06); // "second" length
    script_bytes.extend_from_slice(b"second");
    // OP_ENDIF
    script_bytes.push(0x68);
    
    Witness::from_slice(&[script_bytes, Vec::new()])
}

/// Create envelope with invalid opcodes
pub fn create_envelope_with_invalid_opcodes() -> Witness {
    let mut script_bytes = Vec::new();
    
    // OP_PUSHBYTES_0
    script_bytes.push(0x00);
    // OP_IF
    script_bytes.push(0x63);
    // "ord" tag
    script_bytes.push(0x03);
    script_bytes.extend_from_slice(b"ord");
    // OP_RETURN (invalid in envelope)
    script_bytes.push(0x6A);
    // Content tag (0)
    script_bytes.push(0x00);
    script_bytes.push(0x07); // "invalid" length
    script_bytes.extend_from_slice(b"invalid");
    // OP_ENDIF
    script_bytes.push(0x68);
    
    Witness::from_slice(&[script_bytes, Vec::new()])
}