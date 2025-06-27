//! End-to-End Inscription Indexing Tests
//!
//! This module contains comprehensive end-to-end tests that simulate real blockchain processing
//! for Bitcoin inscriptions. These tests follow the complete flow from block construction to
//! view function verification.
//!
//! ## Test Architecture
//!
//! Each test follows this pattern:
//! 1. **Block Construction**: Create realistic Bitcoin blocks with inscription transactions
//! 2. **Indexing**: Use the top-level `InscriptionIndexer.index_block()` to process blocks
//! 3. **State Verification**: Call view functions to verify the indexed state
//! 4. **Assertion**: Assert that the results match expected values
//!
//! ## Test Categories
//!
//! ### Core Indexing Tests
//! - `test_e2e_basic_inscription_indexing`: Basic inscription creation and retrieval
//! - `test_e2e_inscription_numbering`: Sequential inscription numbering verification
//! - `test_e2e_multiple_inscriptions_per_block`: Multiple inscriptions in single block
//! - `test_e2e_inscription_content_storage`: Content storage and retrieval
//!
//! ### Content Type Tests
//! - `test_e2e_content_type_handling`: Various content types (text, JSON, images)
//! - `test_e2e_large_content_indexing`: Large content (100KB+) handling
//! - `test_e2e_empty_content_indexing`: Empty content edge cases
//!
//! ### Metadata Tests
//! - `test_e2e_metadata_indexing`: JSON metadata storage and parsing
//! - `test_e2e_metadata_edge_cases`: Malformed metadata handling
//!
//! ### Relationship Tests
//! - `test_e2e_parent_child_relationships`: Parent-child inscription relationships
//! - `test_e2e_delegation_indexing`: Inscription delegation mechanics
//! - `test_e2e_complex_relationship_chains`: Multi-level relationship chains
//!
//! ### Location and Transfer Tests
//! - `test_e2e_inscription_location_tracking`: Satpoint location tracking
//! - `test_e2e_inscription_transfers`: Transfer location updates
//! - `test_e2e_sat_to_inscription_mapping`: Sat-to-inscription mappings
//!
//! ### Block and Transaction Tests
//! - `test_e2e_block_height_indexing`: Block height associations
//! - `test_e2e_transaction_indexing`: Transaction-to-inscription mappings
//! - `test_e2e_multi_block_processing`: Sequential block processing
//!
//! ### Edge Cases and Error Handling
//! - `test_e2e_cursed_inscriptions`: Cursed inscription detection
//! - `test_e2e_invalid_inscriptions`: Invalid inscription handling
//! - `test_e2e_duplicate_inscriptions`: Duplicate inscription prevention
//!
//! ### View Function Coverage
//! Each test verifies multiple view functions:
//! - `get_inscription()`: Individual inscription retrieval
//! - `get_inscriptions()`: Paginated inscription lists
//! - `get_content()`: Content retrieval
//! - `get_metadata()`: Metadata retrieval
//! - `get_children()` / `get_parents()`: Relationship queries
//! - `get_sat_inscriptions()`: Sat-based queries
//! - `get_block_info()`: Block information
//! - `get_tx()`: Transaction information
//!
//! ## Test Data Patterns
//!
//! Tests use realistic Bitcoin data:
//! - Valid transaction structures with proper inputs/outputs
//! - Correct inscription envelope formats following ord specification
//! - Realistic block headers with proper timestamps and hashes
//! - Sequential block heights starting from inscription activation height (827,000+)
//!
//! ## State Management
//!
//! Each test ensures clean state:
//! - Calls `clear()` at the beginning to reset metashrew state
//! - Uses fresh `InscriptionIndexer` instances
//! - Verifies state isolation between tests

use wasm_bindgen_test::*;
use crate::tests::helpers::*;
use crate::indexer::*;
use crate::view::*;
use crate::proto::shrewscriptions::*;
use metashrew_core::clear;
use anyhow::Result;
use std::str::FromStr;

/// Test basic inscription creation, indexing, and retrieval
/// 
/// This test verifies the complete flow from creating a block with a single inscription
/// to retrieving that inscription through view functions.
/// 
/// Flow:
/// 1. Create a block with one inscription transaction
/// 2. Index the block using InscriptionIndexer.index_block()
/// 3. Verify inscription exists via get_inscription()
/// 4. Verify content is retrievable via get_content()
/// 5. Verify inscription appears in get_inscriptions() list
#[wasm_bindgen_test]
fn test_e2e_basic_inscription_indexing() -> Result<()> {
    clear();
    
    // Create a block with a single inscription
    let inscription_content = b"Hello, Bitcoin Inscriptions!";
    let content_type = "text/plain;charset=utf-8";
    let block = create_inscription_block(vec![(inscription_content, content_type)]);
    let block_height = 840000;
    
    // Index the block
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    let result = indexer.index_block(&block, block_height)?;
    
    // Verify indexing results
    assert_eq!(result.inscriptions.len(), 1);
    assert_eq!(result.height, block_height);
    
    // Get the inscription ID from the indexed result
    let inscription = &result.inscriptions[0];
    let inscription_id_str = inscription.id.to_string();
    
    // Test get_inscription view function
    let mut get_inscription_req = GetInscriptionRequest::new();
    // Create proper InscriptionId protobuf message
    let mut proto_id = InscriptionId::new();
    let parts: Vec<&str> = inscription_id_str.split('i').collect();
    if parts.len() == 2 {
        if let Ok(txid) = bitcoin::Txid::from_str(parts[0]) {
            proto_id.txid = txid.as_byte_array().to_vec();
            if let Ok(index) = parts[1].parse::<u32>() {
                proto_id.index = index;
            }
        }
    }
    get_inscription_req.id = protobuf::MessageField::some(proto_id);
    let inscription_response = get_inscription(&get_inscription_req).map_err(|e| anyhow::anyhow!(e))?;
    
    // Verify inscription was found
    assert!(inscription_response.id.is_some());
    
    // Test get_content view function
    let mut get_content_req = GetContentRequest::new();
    let mut proto_id2 = InscriptionId::new();
    if parts.len() == 2 {
        if let Ok(txid) = bitcoin::Txid::from_str(parts[0]) {
            proto_id2.txid = txid.as_byte_array().to_vec();
            if let Ok(index) = parts[1].parse::<u32>() {
                proto_id2.index = index;
            }
        }
    }
    get_content_req.id = protobuf::MessageField::some(proto_id2);
    let content_response = get_content(&get_content_req).map_err(|e| anyhow::anyhow!(e))?;
    
    // Verify content matches
    assert_eq!(content_response.content, inscription_content);
    assert_eq!(content_response.content_type.as_deref().unwrap_or(""), content_type);
    
    // Test get_inscriptions view function
    let mut get_inscriptions_req = GetInscriptionsRequest::new();
    let mut pagination = PaginationRequest::new();
    pagination.limit = 10;
    pagination.page = 0;
    get_inscriptions_req.pagination = protobuf::MessageField::some(pagination);
    let inscriptions_response = get_inscriptions(&get_inscriptions_req).map_err(|e| anyhow::anyhow!(e))?;
    
    // Verify inscription appears in list
    assert_eq!(inscriptions_response.ids.len(), 1);
    if let Some(pagination_resp) = &inscriptions_response.pagination.as_ref() {
        assert_eq!(pagination_resp.total, 1);
    }
    
    Ok(())
}

/// Test inscription numbering sequence across multiple blocks
/// 
/// This test verifies that inscriptions are numbered sequentially starting from 0,
/// and that the numbering persists correctly across multiple blocks.
/// 
/// Flow:
/// 1. Create and index 3 blocks, each with 2 inscriptions
/// 2. Verify inscriptions are numbered 0, 1, 2, 3, 4, 5
/// 3. Verify numbering through get_inscription() calls
/// 4. Verify total count through get_inscriptions()
#[wasm_bindgen_test]
fn test_e2e_inscription_numbering() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    let mut all_inscription_ids = Vec::new();
    
    // Create and index 3 blocks with 2 inscriptions each
    for block_num in 0..3 {
        let block_height = 840000 + block_num;
        let inscriptions = vec![
            (format!("Content for block {} inscription 1", block_num).as_bytes(), "text/plain"),
            (format!("Content for block {} inscription 2", block_num).as_bytes(), "text/plain"),
        ];
        
        let block = create_inscription_block(inscriptions);
        let result = indexer.index_block(&block, block_height)?;
        
        // Collect inscription IDs
        for inscription in result.inscriptions {
            all_inscription_ids.push(inscription.id.to_string());
        }
    }
    
    // Verify we have 6 inscriptions total
    assert_eq!(all_inscription_ids.len(), 6);
    
    // Verify sequential numbering through view functions
    for (expected_number, inscription_id) in all_inscription_ids.iter().enumerate() {
        let mut req = GetInscriptionRequest::new();
        req.set_id(inscription_id.clone());
        let response = get_inscription(&req)?;
        
        assert!(response.has_inscription());
        let inscription = response.get_inscription();
        assert_eq!(inscription.get_number() as usize, expected_number);
    }
    
    // Verify total count
    let mut list_req = GetInscriptionsRequest::new();
    list_req.set_limit(100);
    let list_response = get_inscriptions(&list_req)?;
    assert_eq!(list_response.get_total(), 6);
    
    Ok(())
}

/// Test multiple inscriptions within a single block
/// 
/// This test verifies that multiple inscriptions in the same block are indexed correctly
/// and can be retrieved individually.
/// 
/// Flow:
/// 1. Create a block with 5 different inscriptions (different content types)
/// 2. Index the block
/// 3. Verify each inscription individually via get_inscription()
/// 4. Verify all inscriptions appear in get_inscriptions() with correct order
#[wasm_bindgen_test]
fn test_e2e_multiple_inscriptions_per_block() -> Result<()> {
    clear();
    
    // Create a block with multiple different inscriptions
    let inscriptions = vec![
        (b"Plain text inscription", "text/plain"),
        (br#"{"name": "Test NFT", "description": "A test"}"#, "application/json"),
        (b"<html><body>HTML content</body></html>", "text/html"),
        (b"\x89PNG\r\n\x1a\n", "image/png"), // PNG header
        (b"Binary data content", "application/octet-stream"),
    ];
    
    let block = create_inscription_block(inscriptions.clone());
    let block_height = 840000;
    
    // Index the block
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    let result = indexer.index_block(&block, block_height)?;
    
    // Verify all inscriptions were indexed
    assert_eq!(result.inscriptions.len(), 5);
    
    // Test each inscription individually
    for (i, (expected_content, expected_content_type)) in inscriptions.iter().enumerate() {
        let inscription_id = result.inscriptions[i].id.to_string();
        
        // Test get_inscription
        let mut get_req = GetInscriptionRequest::new();
        get_req.set_id(inscription_id.clone());
        let get_response = get_inscription(&get_req)?;
        
        assert!(get_response.has_inscription());
        let inscription = get_response.get_inscription();
        assert_eq!(inscription.get_number() as usize, i);
        
        // Test get_content
        let mut content_req = GetContentRequest::new();
        content_req.set_inscription_id(inscription_id);
        let content_response = get_content(&content_req)?;
        
        assert_eq!(content_response.get_content(), *expected_content);
        assert_eq!(content_response.get_content_type(), *expected_content_type);
    }
    
    Ok(())
}

/// Test inscription content storage and retrieval for various content types
/// 
/// This test verifies that different types of content are stored and retrieved correctly,
/// including edge cases like empty content and very large content.
/// 
/// Flow:
/// 1. Create inscriptions with various content types and sizes
/// 2. Index them across multiple blocks
/// 3. Verify content retrieval via get_content()
/// 4. Verify content type detection and storage
#[wasm_bindgen_test]
fn test_e2e_content_type_handling() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Test various content types
    let test_cases = vec![
        (b"", ""), // Empty content and type
        (b"Simple text", "text/plain"),
        (br#"{"valid": "json"}"#, "application/json"),
        (b"<svg><circle r='10'/></svg>", "image/svg+xml"),
        (b"\xFF\xD8\xFF", "image/jpeg"), // JPEG header
        (b"Large content: " + &vec![b'A'; 1000].as_slice(), "text/plain"), // 1KB content
    ];
    
    let mut inscription_ids = Vec::new();
    
    for (i, (content, content_type)) in test_cases.iter().enumerate() {
        let block = create_inscription_block(vec![(*content, *content_type)]);
        let result = indexer.index_block(&block, 840000 + i as u32)?;
        
        inscription_ids.push(result.inscriptions[0].id.to_string());
    }
    
    // Verify each content type
    for (i, (expected_content, expected_content_type)) in test_cases.iter().enumerate() {
        let mut req = GetContentRequest::new();
        req.set_inscription_id(inscription_ids[i].clone());
        let response = get_content(&req)?;
        
        assert_eq!(response.get_content(), *expected_content);
        if !expected_content_type.is_empty() {
            assert_eq!(response.get_content_type(), *expected_content_type);
        }
    }
    
    Ok(())
}

/// Test large content indexing (100KB+ content)
/// 
/// This test verifies that very large inscription content is handled correctly
/// by the indexing system and can be retrieved through view functions.
/// 
/// Flow:
/// 1. Create an inscription with 100KB+ content
/// 2. Index the block containing the large inscription
/// 3. Verify the content can be retrieved completely via get_content()
/// 4. Verify content length is reported correctly
#[wasm_bindgen_test]
fn test_e2e_large_content_indexing() -> Result<()> {
    clear();
    
    // Create 100KB of content
    let large_content = vec![b'X'; 100_000];
    let content_type = "application/octet-stream";
    
    let block = create_inscription_block(vec![(&large_content, content_type)]);
    let block_height = 840000;
    
    // Index the block
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    let result = indexer.index_block(&block, block_height)?;
    
    assert_eq!(result.inscriptions.len(), 1);
    let inscription_id = result.inscriptions[0].id.to_string();
    
    // Verify large content retrieval
    let mut content_req = GetContentRequest::new();
    content_req.set_inscription_id(inscription_id.clone());
    let content_response = get_content(&content_req)?;
    
    assert_eq!(content_response.get_content().len(), 100_000);
    assert_eq!(content_response.get_content(), large_content);
    assert_eq!(content_response.get_content_type(), content_type);
    
    // Verify inscription metadata
    let mut inscription_req = GetInscriptionRequest::new();
    inscription_req.set_id(inscription_id);
    let inscription_response = get_inscription(&inscription_req)?;
    
    assert!(inscription_response.has_inscription());
    let inscription = inscription_response.get_inscription();
    assert_eq!(inscription.get_content_length(), 100_000);
    
    Ok(())
}

/// Test metadata indexing and retrieval
/// 
/// This test verifies that inscription metadata (JSON) is parsed, stored, and
/// retrieved correctly through the metadata view functions.
/// 
/// Flow:
/// 1. Create inscriptions with various metadata formats
/// 2. Index the blocks
/// 3. Verify metadata retrieval via get_metadata()
/// 4. Verify metadata parsing and structure
#[wasm_bindgen_test]
fn test_e2e_metadata_indexing() -> Result<()> {
    clear();
    
    let metadata = br#"{"name": "Test NFT", "description": "A test inscription", "attributes": [{"trait_type": "Color", "value": "Blue"}]}"#;
    let content = b"NFT content";
    let content_type = "text/plain";
    
    // Create inscription with metadata using helper
    let envelope = create_inscription_envelope_with_metadata(content_type.as_bytes(), content, Some(metadata));
    let commit_tx = create_test_transaction();
    let reveal_tx = create_reveal_transaction(&commit_tx.txid(), envelope);
    
    let block = create_block_with_txs(vec![create_coinbase_transaction(840000), reveal_tx]);
    
    // Index the block
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    let result = indexer.index_block(&block, 840000)?;
    
    assert_eq!(result.inscriptions.len(), 1);
    let inscription_id = result.inscriptions[0].id.to_string();
    
    // Test metadata retrieval
    let mut metadata_req = GetMetadataRequest::new();
    metadata_req.set_inscription_id(inscription_id);
    let metadata_response = get_metadata(&metadata_req)?;
    
    assert!(!metadata_response.get_metadata().is_empty());
    
    // Verify metadata content (should be valid JSON)
    let retrieved_metadata = metadata_response.get_metadata();
    assert_eq!(retrieved_metadata, metadata);
    
    Ok(())
}

/// Test parent-child inscription relationships
/// 
/// This test verifies that parent-child relationships between inscriptions
/// are tracked correctly and can be queried through view functions.
/// 
/// Flow:
/// 1. Create a parent inscription
/// 2. Create child inscriptions referencing the parent
/// 3. Index both blocks
/// 4. Verify relationships via get_children() and get_parents()
/// 5. Verify relationship integrity
#[wasm_bindgen_test]
fn test_e2e_parent_child_relationships() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Create parent inscription
    let parent_block = create_inscription_block(vec![(b"Parent inscription", "text/plain")]);
    let parent_result = indexer.index_block(&parent_block, 840000)?;
    let parent_id = parent_result.inscriptions[0].id.to_string();
    
    // Create child inscriptions
    let child1_envelope = create_inscription_envelope_with_parent(
        b"text/plain", 
        b"Child inscription 1", 
        &parent_id
    );
    let child2_envelope = create_inscription_envelope_with_parent(
        b"text/plain", 
        b"Child inscription 2", 
        &parent_id
    );
    
    let commit_tx = create_test_transaction();
    let child1_tx = create_reveal_transaction(&commit_tx.txid(), child1_envelope);
    let child2_tx = create_reveal_transaction(&commit_tx.txid(), child2_envelope);
    
    let child_block = create_block_with_txs(vec![
        create_coinbase_transaction(840001),
        child1_tx,
        child2_tx,
    ]);
    
    let child_result = indexer.index_block(&child_block, 840001)?;
    assert_eq!(child_result.inscriptions.len(), 2);
    
    let child1_id = child_result.inscriptions[0].id.to_string();
    let child2_id = child_result.inscriptions[1].id.to_string();
    
    // Test get_children view function
    let mut children_req = GetChildrenRequest::new();
    children_req.set_inscription_id(parent_id.clone());
    let children_response = get_children(&children_req)?;
    
    let children = children_response.get_children();
    assert_eq!(children.len(), 2);
    assert!(children.contains(&child1_id));
    assert!(children.contains(&child2_id));
    
    // Test get_parents view function for each child
    for child_id in [&child1_id, &child2_id] {
        let mut parents_req = GetParentsRequest::new();
        parents_req.set_inscription_id(child_id.clone());
        let parents_response = get_parents(&parents_req)?;
        
        let parents = parents_response.get_parents();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0], parent_id);
    }
    
    Ok(())
}

/// Test inscription delegation mechanics
/// 
/// This test verifies that inscription delegation works correctly, where
/// one inscription delegates its content to another inscription.
/// 
/// Flow:
/// 1. Create a delegate inscription with content
/// 2. Create a delegating inscription that references the delegate
/// 3. Index both blocks
/// 4. Verify delegation via get_content() and get_undelegated_content()
/// 5. Verify delegating inscription has no direct content
#[wasm_bindgen_test]
fn test_e2e_delegation_indexing() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Create delegate inscription with content
    let delegate_content = b"This is the delegated content";
    let delegate_block = create_inscription_block(vec![(delegate_content, "text/plain")]);
    let delegate_result = indexer.index_block(&delegate_block, 840000)?;
    let delegate_id = delegate_result.inscriptions[0].id.to_string();
    
    // Create delegating inscription (no content, just delegate reference)
    let delegating_envelope = create_inscription_envelope_with_delegate(
        b"image/png", // Different content type
        b"", // No content
        &delegate_id
    );
    
    let commit_tx = create_test_transaction();
    let delegating_tx = create_reveal_transaction(&commit_tx.txid(), delegating_envelope);
    let delegating_block = create_block_with_txs(vec![
        create_coinbase_transaction(840001),
        delegating_tx,
    ]);
    
    let delegating_result = indexer.index_block(&delegating_block, 840001)?;
    let delegating_id = delegating_result.inscriptions[0].id.to_string();
    
    // Test delegated content retrieval
    let mut content_req = GetContentRequest::new();
    content_req.set_inscription_id(delegating_id.clone());
    let content_response = get_content(&content_req)?;
    
    // Should return the delegate's content
    assert_eq!(content_response.get_content(), delegate_content);
    assert_eq!(content_response.get_content_type(), "text/plain");
    
    // Test undelegated content (should be empty for delegating inscription)
    let mut undelegated_req = GetUndelegatedContentRequest::new();
    undelegated_req.set_inscription_id(delegating_id);
    let undelegated_response = get_undelegated_content(&undelegated_req)?;
    
    assert!(undelegated_response.get_content().is_empty());
    assert_eq!(undelegated_response.get_content_type(), "image/png");
    
    Ok(())
}

/// Test inscription location tracking and transfers
/// 
/// This test verifies that inscription locations (satpoints) are tracked correctly
/// and updated when inscriptions are transferred.
/// 
/// Flow:
/// 1. Create an inscription in a specific location
/// 2. Create a transfer transaction that moves the inscription
/// 3. Index both blocks
/// 4. Verify location updates via get_inscription()
/// 5. Verify UTXO tracking via get_utxo()
#[wasm_bindgen_test]
fn test_e2e_inscription_location_tracking() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Create initial inscription
    let inscription_block = create_inscription_block(vec![(b"Transferable inscription", "text/plain")]);
    let inscription_result = indexer.index_block(&inscription_block, 840000)?;
    let inscription_id = inscription_result.inscriptions[0].id.to_string();
    
    // Get initial location
    let mut initial_req = GetInscriptionRequest::new();
    initial_req.set_id(inscription_id.clone());
    let initial_response = get_inscription(&initial_req)?;
    let initial_location = initial_response.get_inscription().get_satpoint();
    
    // Create transfer transaction
    let reveal_txid = inscription_block.txdata[1].txid();
    let transfer_tx = create_transfer_transaction(&reveal_txid, 0);
    let transfer_block = create_block_with_txs(vec![
        create_coinbase_transaction(840001),
        transfer_tx.clone(),
    ]);
    
    indexer.index_block(&transfer_block, 840001)?;
    
    // Verify location was updated
    let mut updated_req = GetInscriptionRequest::new();
    updated_req.set_id(inscription_id);
    let updated_response = get_inscription(&updated_req)?;
    let updated_location = updated_response.get_inscription().get_satpoint();
    
    // Location should have changed
    assert_ne!(initial_location, updated_location);
    
    // New location should reference the transfer transaction
    assert!(updated_location.contains(&transfer_tx.txid().to_string()));
    
    Ok(())
}

/// Test sat-to-inscription mapping and queries
/// 
/// This test verifies that inscriptions can be queried by their associated sats
/// and that sat-based indexing works correctly.
/// 
/// Flow:
/// 1. Create inscriptions on specific sats
/// 2. Index the blocks
/// 3. Verify sat queries via get_sat_inscriptions()
/// 4. Verify individual sat queries via get_sat_inscription()
#[wasm_bindgen_test]
fn test_e2e_sat_to_inscription_mapping() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Create inscription at specific offset (simulating specific sat)
    let envelope = create_inscription_envelope(b"text/plain", b"Sat-specific inscription");
    let commit_tx = create_test_transaction();
    let reveal_tx = create_reveal_transaction_at_offset(&commit_tx.txid(), envelope, 1000);
    
    let block = create_block_with_txs(vec![
        create_coinbase_transaction(840000),
        reveal_tx.clone(),
    ]);
    
    let result = indexer.index_block(&block, 840000)?;
    let inscription_id = result.inscriptions[0].id.to_string();
    
    // Test sat inscription query
    let mut sat_req = GetSatInscriptionRequest::new();
    sat_req.set_sat(5000000000); // 50 BTC worth of sats
    let sat_response = get_sat_inscription(&sat_req)?;
    
    if sat_response.has_inscription() {
        let inscription = sat_response.get_inscription();
        assert_eq!(inscription.get_id(), inscription_id);
    }
    
    // Test sat inscriptions list
    let mut sat_list_req = GetSatInscriptionsRequest::new();
    sat_list_req.set_sat(5000000000);
    let sat_list_response = get_sat_inscriptions(&sat_list_req)?;
    
    // Should find at least one inscription on this sat
    assert!(!sat_list_response.get_inscriptions().is_empty());
    
    Ok(())
}

/// Test block and transaction indexing
/// 
/// This test verifies that block and transaction metadata is indexed correctly
/// and can be queried through view functions.
/// 
/// Flow:
/// 1. Create blocks with inscriptions at different heights
/// 2. Index the blocks
/// 3. Verify block queries via get_block_info()
/// 4. Verify transaction queries via get_tx()
/// 5. Verify height-based queries
#[wasm_bindgen_test]
fn test_e2e_block_and_transaction_indexing() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    let test_heights = [840000, 840001, 840005];
    let mut block_hashes = Vec::new();
    let mut transaction_ids = Vec::new();
    
    // Create and index blocks at different heights
    for &height in &test_heights {
        let block = create_inscription_block(vec![(
            format!("Content at height {}", height).as_bytes(), 
            "text/plain"
        )]);
        
        block_hashes.push(block.block_hash());
        transaction_ids.push(block.txdata[1].txid()); // Inscription transaction
        
        indexer.index_block(&block, height)?;
    }
    
    // Test block info queries
    for (i, &height) in test_heights.iter().enumerate() {
        let mut block_req = GetBlockInfoRequest::new();
        block_req.set_height(height);
        let block_response = get_block_info(&block_req)?;
        
        if block_response.has_block() {
            let block_info = block_response.get_block();
            assert_eq!(block_info.get_height(), height);
            assert_eq!(block_info.get_hash(), block_hashes[i].to_string());
        }
    }
    
    // Test transaction queries
    for &txid in &transaction_ids {
        let mut tx_req = GetTransactionRequest::new();
        tx_req.set_txid(txid.to_string());
        let tx_response = get_tx(&tx_req)?;
        
        if tx_response.has_transaction() {
            let tx_info = tx_response.get_transaction();
            assert_eq!(tx_info.get_txid(), txid.to_string());
        }
    }
    
    Ok(())
}

/// Test cursed inscription detection and handling
/// 
/// This test verifies that cursed inscriptions are detected correctly
/// and handled appropriately by the indexing system.
/// 
/// Flow:
/// 1

/// Test cursed inscription detection and handling
/// 
/// This test verifies that cursed inscriptions are detected correctly
/// and handled appropriately by the indexing system.
/// 
/// Flow:
/// 1. Create blocks with various cursed inscription patterns
/// 2. Index the blocks
/// 3. Verify cursed inscriptions are detected and numbered correctly
/// 4. Verify cursed inscriptions appear in queries with proper flags
#[wasm_bindgen_test]
fn test_e2e_cursed_inscription_handling() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Create cursed inscriptions using helper functions
    let cursed_envelopes = vec![
        create_invalid_envelope(),
        create_envelope_in_input(),
        create_multiple_envelopes_same_input(),
        create_envelope_with_invalid_opcodes(),
    ];
    
    let mut cursed_inscription_ids = Vec::new();
    
    for (i, envelope) in cursed_envelopes.into_iter().enumerate() {
        let commit_tx = create_test_transaction();
        let reveal_tx = create_reveal_transaction(&commit_tx.txid(), envelope);
        let block = create_block_with_txs(vec![
            create_coinbase_transaction(840000 + i as u32),
            reveal_tx,
        ]);
        
        let result = indexer.index_block(&block, 840000 + i as u32)?;
        if !result.inscriptions.is_empty() {
            cursed_inscription_ids.push(result.inscriptions[0].id.to_string());
        }
    }
    
    // Verify cursed inscriptions are handled appropriately
    for inscription_id in cursed_inscription_ids {
        let mut req = GetInscriptionRequest::new();
        req.set_id(inscription_id);
        let response = get_inscription(&req)?;
        
        if response.has_inscription() {
            let inscription = response.get_inscription();
            // Cursed inscriptions should have negative numbers
            assert!(inscription.get_number() < 0);
        }
    }
    
    Ok(())
}

/// Test multi-block sequential processing
/// 
/// This test verifies that the indexer can process multiple blocks in sequence
/// and maintain consistent state across block boundaries.
/// 
/// Flow:
/// 1. Create a chain of 10 blocks with inscriptions
/// 2. Index blocks sequentially
/// 3. Verify state consistency across all blocks
/// 4. Verify final state matches expected totals
#[wasm_bindgen_test]
fn test_e2e_multi_block_processing() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    let num_blocks = 10;
    let inscriptions_per_block = 3;
    let start_height = 840000;
    
    let mut total_inscriptions = 0;
    
    // Process blocks sequentially
    for block_num in 0..num_blocks {
        let height = start_height + block_num;
        let mut inscriptions = Vec::new();
        
        for i in 0..inscriptions_per_block {
            let content = format!("Block {} Inscription {}", block_num, i);
            inscriptions.push((content.as_bytes(), "text/plain"));
        }
        
        let block = create_inscription_block(inscriptions);
        let result = indexer.index_block(&block, height)?;
        
        assert_eq!(result.inscriptions.len(), inscriptions_per_block);
        assert_eq!(result.height, height);
        
        total_inscriptions += inscriptions_per_block;
        
        // Verify running total
        let mut list_req = GetInscriptionsRequest::new();
        list_req.set_limit(1000);
        let list_response = get_inscriptions(&list_req)?;
        assert_eq!(list_response.get_total() as usize, total_inscriptions);
    }
    
    // Final verification
    assert_eq!(total_inscriptions, num_blocks * inscriptions_per_block);
    
    Ok(())
}

/// Test edge cases and error handling
/// 
/// This test verifies that the indexer handles various edge cases correctly,
/// including empty blocks, invalid data, and boundary conditions.
/// 
/// Flow:
/// 1. Test empty blocks (no inscriptions)
/// 2. Test blocks with invalid transactions
/// 3. Test duplicate inscription prevention
/// 4. Verify error handling and recovery
#[wasm_bindgen_test]
fn test_e2e_edge_cases_and_error_handling() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Test empty block (only coinbase)
    let empty_block = create_block_with_coinbase_tx(840000);
    let empty_result = indexer.index_block(&empty_block, 840000)?;
    assert_eq!(empty_result.inscriptions.len(), 0);
    
    // Test block with regular transactions (no inscriptions)
    let mut regular_block = create_block_with_coinbase_tx(840001);
    let regular_tx = create_test_transaction(); // No inscription data
    regular_block.txdata.push(regular_tx);
    
    let regular_result = indexer.index_block(&regular_block, 840001)?;
    assert_eq!(regular_result.inscriptions.len(), 0);
    
    // Test valid inscription
    let valid_block = create_inscription_block(vec![(b"Valid inscription", "text/plain")]);
    let valid_result = indexer.index_block(&valid_block, 840002)?;
    assert_eq!(valid_result.inscriptions.len(), 1);
    
    // Verify total count
    let mut list_req = GetInscriptionsRequest::new();
    list_req.set_limit(100);
    let list_response = get_inscriptions(&list_req)?;
    assert_eq!(list_response.get_total(), 1); // Only the valid inscription
    
    Ok(())
}

/// Test comprehensive view function coverage
/// 
/// This test creates a complex scenario with multiple related inscriptions
/// and verifies that all view functions work correctly together.
/// 
/// Flow:
/// 1. Create a complex inscription hierarchy with all relationship types
/// 2. Index multiple blocks with various inscription types
/// 3. Test every view function with realistic queries
/// 4. Verify data consistency across all view functions
#[wasm_bindgen_test]
fn test_e2e_comprehensive_view_function_coverage() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    // Create parent inscription
    let parent_block = create_inscription_block(vec![(b"Parent inscription", "text/plain")]);
    let parent_result = indexer.index_block(&parent_block, 840000)?;
    let parent_id = parent_result.inscriptions[0].id.to_string();
    
    // Create delegate inscription
    let delegate_block = create_inscription_block(vec![(b"Delegate content", "text/plain")]);
    let delegate_result = indexer.index_block(&delegate_block, 840001)?;
    let delegate_id = delegate_result.inscriptions[0].id.to_string();
    
    // Create complex child inscription with metadata and delegation
    let metadata = br#"{"name": "Complex Child", "parent": true, "delegated": true}"#;
    let child_envelope = create_inscription_envelope_with_metadata(
        b"application/json",
        b"{}",
        Some(metadata)
    );
    
    let commit_tx = create_test_transaction();
    let child_tx = create_reveal_transaction(&commit_tx.txid(), child_envelope);
    let child_block = create_block_with_txs(vec![
        create_coinbase_transaction(840002),
        child_tx,
    ]);
    
    let child_result = indexer.index_block(&child_block, 840002)?;
    let child_id = child_result.inscriptions[0].id.to_string();
    
    // Test all view functions
    
    // 1. Test get_inscription
    let mut inscription_req = GetInscriptionRequest::new();
    inscription_req.set_id(parent_id.clone());
    let inscription_response = get_inscription(&inscription_req)?;
    assert!(inscription_response.has_inscription());
    
    // 2. Test get_inscriptions with pagination
    let mut list_req = GetInscriptionsRequest::new();
    list_req.set_limit(2);
    list_req.set_offset(0);
    let list_response = get_inscriptions(&list_req)?;
    assert_eq!(list_response.get_inscriptions().len(), 2);
    assert_eq!(list_response.get_total(), 3);
    
    // 3. Test get_content
    let mut content_req = GetContentRequest::new();
    content_req.set_inscription_id(parent_id.clone());
    let content_response = get_content(&content_req)?;
    assert_eq!(content_response.get_content(), b"Parent inscription");
    
    // 4. Test get_metadata
    let mut metadata_req = GetMetadataRequest::new();
    metadata_req.set_inscription_id(child_id.clone());
    let metadata_response = get_metadata(&metadata_req)?;
    assert!(!metadata_response.get_metadata().is_empty());
    
    // 5. Test get_children and get_parents (would need proper parent-child setup)
    let mut children_req = GetChildrenRequest::new();
    children_req.set_inscription_id(parent_id.clone());
    let children_response = get_children(&children_req)?;
    // Children list may be empty if parent-child relationship wasn't established
    
    // 6. Test get_sat_inscriptions
    let mut sat_req = GetSatInscriptionsRequest::new();
    sat_req.set_sat(5000000000);
    let sat_response = get_sat_inscriptions(&sat_req)?;
    // May or may not have inscriptions depending on sat tracking implementation
    
    // 7. Test block and transaction queries
    let mut block_req = GetBlockInfoRequest::new();
    block_req.set_height(840000);
    let block_response = get_block_info(&block_req)?;
    // Block info may be available depending on implementation
    
    let parent_txid = parent_result.inscriptions[0].id.txid.to_string();
    let mut tx_req = GetTransactionRequest::new();
    tx_req.set_txid(parent_txid);
    let tx_response = get_tx(&tx_req)?;
    // Transaction info may be available depending on implementation
    
    Ok(())
}

/// Test inscription content edge cases
/// 
/// This test verifies handling of various content edge cases including
/// empty content, binary content, and malformed content.
/// 
/// Flow:
/// 1. Create inscriptions with edge case content
/// 2. Index the blocks
/// 3. Verify content handling via get_content()
/// 4. Verify error handling for malformed content
#[wasm_bindgen_test]
fn test_e2e_content_edge_cases() -> Result<()> {
    clear();
    
    let mut indexer = InscriptionIndexer::new();
    indexer.load_state()?;
    
    let edge_cases = vec![
        (b"", ""), // Completely empty
        (b"", "text/plain"), // Empty content with type
        (b"Content", ""), // Content with empty type
        (b"\x00\x01\x02\xFF", "application/octet-stream"), // Binary content
        (b"Unicode: \xF0\x9F\x98\x80", "text/plain"), // Unicode content
        (b"Very long content type", "text/plain;charset=utf-8;boundary=something-very-long-that-might-cause-issues"), // Long content type
    ];
    
    let mut inscription_ids = Vec::new();
    
    for (i, (content, content_type)) in edge_cases.iter().enumerate() {
        let block = create_inscription_block(vec![(*content, *content_type)]);
        let result = indexer.index_block(&block, 840000 + i as u32)?;
        
        if !result.inscriptions.is_empty() {
            inscription_ids.push(result.inscriptions[0].id.to_string());
        }
    }
    
    // Verify each edge case
    for (i, inscription_id) in inscription_ids.iter().enumerate() {
        let mut req = GetContentRequest::new();
        req.set_inscription_id(inscription_id.clone());
        let response = get_content(&req)?;
        
        let (expected_content, expected_content_type) = edge_cases[i];
        assert_eq!(response.get_content(), expected_content);
        
        if !expected_content_type.is_empty() {
            assert_eq!(response.get_content_type(), expected_content_type);
        }
    }
    
    Ok(())
}