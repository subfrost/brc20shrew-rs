//! Debug test for content truncation issue
//! 
//! This test isolates the specific issue where JSON content with "image" field
//! gets an extra 'e' byte prepended during storage/retrieval.

use crate::tests::helpers::*;
use crate::indexer::InscriptionIndexer;
use crate::view::*;
use crate::proto::*;
use bitcoin::Txid;
use bitcoin_hashes::Hash;
use wasm_bindgen_test::wasm_bindgen_test;
use anyhow::Result;

#[wasm_bindgen_test]
fn test_json_content_with_image_field() -> Result<()> {
    clear();
    
    // === SETUP PHASE ===
    let mut indexer = InscriptionIndexer::new();
    
    // Create the exact JSON content that's failing
    let json_content = br#"{"name": "Test NFT", "description": "A comprehensive test", "image": "data:image/svg+xml;base64,..."}"#;
    let content_type = "application/json";
    
    println!("DEBUG: Original JSON content length: {}", json_content.len());
    println!("DEBUG: Original JSON content bytes: {:?}", &json_content[..20]); // First 20 bytes
    
    let witness = create_inscription_envelope(content_type.as_bytes(), json_content);
    let mut commit_txid_bytes = [0u8; 32];
    commit_txid_bytes[31] = 1; // Make commit txid unique
    let commit_txid = Txid::from_slice(&commit_txid_bytes).unwrap();
    let tx = create_reveal_transaction_at_offset(&commit_txid, witness, 0);
    
    // Debug: Check if envelope parsing finds the inscription
    use crate::envelope::parse_inscriptions_from_transaction;
    let parsed_envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert!(!parsed_envelopes.is_empty(), "No envelopes found in transaction");
    assert_eq!(parsed_envelopes.len(), 1, "Expected 1 envelope, found {}", parsed_envelopes.len());
    
    // Debug: Check the parsed envelope content
    if let Some(body) = &parsed_envelopes[0].payload.body {
        println!("DEBUG: Parsed envelope body length: {}", body.len());
        println!("DEBUG: Parsed envelope body bytes: {:?}", &body[..20.min(body.len())]);
    }
    
    // Index the transaction
    indexer.index_block(&create_block_with_txs(vec![create_coinbase_transaction(840000), tx.clone()]), 840000).unwrap();
    
    // Test content retrieval
    let inscription_index = 0;
    let mut get_content_req = GetContentRequest::default();
    let mut proto_id = InscriptionId::default();
    proto_id.txid = tx.txid().to_byte_array().to_vec();
    proto_id.index = inscription_index;
    get_content_req.id = Some(proto_id.clone());
    
    let content_response = get_content(&get_content_req).map_err(|e| anyhow::anyhow!(e))?;
    
    println!("DEBUG: Retrieved content length: {}", content_response.content.len());
    println!("DEBUG: Retrieved content bytes: {:?}", &content_response.content[..20.min(content_response.content.len())]);
    
    // Debug: Check what content was stored in the table directly
    use crate::tables::InscriptionContentTable;
    let content_table = InscriptionContentTable::new();
    let inscription_id_str = format!("{}i{}", tx.txid(), inscription_index);
    let stored_content = content_table.get(&inscription_id_str);
    
    if let Some(stored) = &stored_content {
        println!("DEBUG: Stored content length: {}", stored.len());
        println!("DEBUG: Stored content bytes: {:?}", &stored[..20.min(stored.len())]);
    }
    
    // Check if content matches original
    if content_response.content != json_content {
        panic!("Content mismatch: expected {:?}, got {:?}, stored content: {:?}, inscription_id: {}",
               json_content, content_response.content, stored_content, inscription_id_str);
    }
    
    // Verify content matches original
    assert_eq!(content_response.content, json_content);
    
    Ok(())
}

#[wasm_bindgen_test]
fn test_simple_json_content() -> Result<()> {
    clear();
    
    // === SETUP PHASE ===
    let mut indexer = InscriptionIndexer::new();
    
    // Create simple JSON content without "image" field
    let json_content = br#"{"name": "Test NFT", "description": "A test inscription"}"#;
    let content_type = "application/json";
    
    println!("DEBUG: Simple JSON content length: {}", json_content.len());
    println!("DEBUG: Simple JSON content bytes: {:?}", &json_content[..20]); // First 20 bytes
    
    let witness = create_inscription_envelope(content_type.as_bytes(), json_content);
    let mut commit_txid_bytes = [0u8; 32];
    commit_txid_bytes[31] = 2; // Make commit txid unique
    let commit_txid = Txid::from_slice(&commit_txid_bytes).unwrap();
    let tx = create_reveal_transaction_at_offset(&commit_txid, witness, 0);
    
    // Index the transaction
    indexer.index_block(&create_block_with_txs(vec![create_coinbase_transaction(840000), tx.clone()]), 840000).unwrap();
    
    // Test content retrieval
    let inscription_index = 0;
    let mut get_content_req = GetContentRequest::default();
    let mut proto_id = InscriptionId::default();
    proto_id.txid = tx.txid().to_byte_array().to_vec();
    proto_id.index = inscription_index;
    get_content_req.id = Some(proto_id.clone());
    
    let content_response = get_content(&get_content_req).map_err(|e| anyhow::anyhow!(e))?;
    
    println!("DEBUG: Simple retrieved content length: {}", content_response.content.len());
    println!("DEBUG: Simple retrieved content bytes: {:?}", &content_response.content[..20.min(content_response.content.len())]);
    
    // Verify content matches original
    assert_eq!(content_response.content, json_content);
    
    Ok(())
}