//! Debug delegation test to isolate the truncation issue

use crate::tests::helpers::*;
use crate::indexer::InscriptionIndexer;
use crate::view::*;
use crate::proto::shrewscriptions::*;
use bitcoin::Txid;
use bitcoin_hashes::Hash;
use wasm_bindgen_test::wasm_bindgen_test;
use anyhow::Result;

#[wasm_bindgen_test]
fn test_delegate_content_storage() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    
    // Create delegate inscription with 49-byte content
    let delegate_content = b"This is the actual content that will be delegated";
    println!("Original delegate content length: {}", delegate_content.len());
    println!("Original delegate content: {:?}", delegate_content);
    
    let delegate_witness = create_inscription_envelope(b"text/plain", delegate_content);
    let mut delegate_commit_txid_bytes = [0u8; 32];
    delegate_commit_txid_bytes[30] = 1;
    let delegate_commit_txid = Txid::from_slice(&delegate_commit_txid_bytes).unwrap();
    let delegate_tx = create_reveal_transaction(&delegate_commit_txid, delegate_witness);
    
    // Index delegate inscription
    indexer.index_block(&create_block_with_txs(vec![create_coinbase_transaction(840000), delegate_tx.clone()]), 840000).unwrap();
    
    // Test direct content retrieval from delegate
    let mut get_content_req = GetContentRequest::default();
    let mut delegate_proto_id = InscriptionId::default();
    delegate_proto_id.txid = delegate_tx.txid().to_byte_array().to_vec();
    delegate_proto_id.index = 0;
    get_content_req.id = Some(delegate_proto_id.clone());
    
    let delegate_content_response = get_content(&get_content_req).map_err(|e| anyhow::anyhow!(e))?;
    
    println!("Retrieved delegate content length: {}", delegate_content_response.content.len());
    println!("Retrieved delegate content: {:?}", delegate_content_response.content);
    
    // Check if delegate content is properly stored
    if delegate_content_response.content != delegate_content {
        panic!("Delegate content mismatch: expected {:?}, got {:?}", 
               delegate_content, delegate_content_response.content);
    }
    
    println!("Delegate content stored correctly!");
    
    // Now test delegation
    let delegate_id = format!("{}i0", delegate_tx.txid());
    println!("Delegate ID: {}", delegate_id);
    
    // Create delegating inscription (empty content, just delegate reference)
    let delegating_witness = create_inscription_envelope_with_delegate(b"text/plain", b"", &delegate_id);
    let mut delegating_commit_txid_bytes = [0u8; 32];
    delegating_commit_txid_bytes[30] = 2;
    let delegating_commit_txid = Txid::from_slice(&delegating_commit_txid_bytes).unwrap();
    let delegating_tx = create_reveal_transaction(&delegating_commit_txid, delegating_witness);
    
    // Index delegating inscription
    indexer.index_block(&create_block_with_txs(vec![create_coinbase_transaction(840001), delegating_tx.clone()]), 840001).unwrap();
    
    // Test delegating inscription content retrieval
    let mut delegating_get_content_req = GetContentRequest::default();
    let mut delegating_proto_id = InscriptionId::default();
    delegating_proto_id.txid = delegating_tx.txid().to_byte_array().to_vec();
    delegating_proto_id.index = 0;
    delegating_get_content_req.id = Some(delegating_proto_id.clone());
    
    let delegating_content_response = get_content(&delegating_get_content_req).map_err(|e| anyhow::anyhow!(e))?;
    
    println!("Retrieved delegating content length: {}", delegating_content_response.content.len());
    println!("Retrieved delegating content: {:?}", delegating_content_response.content);
    
    // This should return the delegate's content
    if delegating_content_response.content != delegate_content {
        panic!("Delegation failed: expected delegate content {:?}, got {:?}", 
               delegate_content, delegating_content_response.content);
    }
    
    println!("Delegation working correctly!");
    
    Ok(())
}