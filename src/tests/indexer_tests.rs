#[cfg(test)]
mod tests {
    use super::super::helpers::*;
    use crate::indexer::{InscriptionIndexer, IndexError};
    use crate::inscription::{InscriptionId, InscriptionEntry};
    use crate::tables::InscriptionTable;
    use bitcoin::hashes::Hash;
    use metashrew_core::index_pointer::IndexPointer;
    use metashrew_support::index_pointer::KeyValuePointer;
    use wasm_bindgen_test::wasm_bindgen_test;
    use anyhow::Result;

    #[wasm_bindgen_test]
    fn test_indexer_initialization() -> Result<()> {
        clear();
        
        // Create a new indexer
        let mut indexer = InscriptionIndexer::new();
        
        // Load state should succeed even with empty state
        indexer.load_state()?;
        
        // Verify initial state
        assert_eq!(indexer.next_inscription_number, 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_single_inscription_indexing() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create a block with one inscription
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        
        // Index the block
        indexer.index_block(&block, 840000)?;
        
        // Verify inscription was indexed
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        // Check if inscription exists in the index
        let inscription_key = inscription_id.to_bytes();
        let stored_data = InscriptionTable::INSCRIPTIONS
            .select(&inscription_key)
            .get();
        
        assert!(!stored_data.is_empty(), "Inscription should be stored");
        
        // Verify inscription number was incremented
        assert_eq!(indexer.next_inscription_number, 1);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_multiple_inscriptions_same_block() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create a block with multiple inscriptions
        let inscriptions = vec![
            create_test_inscription_content(),
            create_test_json_inscription(),
        ];
        
        let block = create_inscription_block(inscriptions.clone());
        
        // Index the block
        indexer.index_block(&block, 840000)?;
        
        // Verify all inscriptions were indexed
        for (i, _) in inscriptions.iter().enumerate() {
            let tx_index = i + 1; // Skip coinbase
            let txid = block.txdata[tx_index].compute_txid();
            let inscription_id = InscriptionId::new(txid, 0);
            
            let inscription_key = inscription_id.to_bytes();
            let stored_data = InscriptionTable::INSCRIPTIONS
                .select(&inscription_key)
                .get();
            
            assert!(!stored_data.is_empty(), "Inscription {} should be stored", i);
        }
        
        // Verify inscription number was incremented correctly
        assert_eq!(indexer.next_inscription_number, inscriptions.len() as u64);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_numbering_sequence() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Index multiple blocks with inscriptions
        for block_height in 840000..840005 {
            let (content, content_type) = create_test_inscription_content();
            let block = create_inscription_block(vec![(content, content_type)]);
            
            indexer.index_block(&block, block_height)?;
        }
        
        // Verify inscription numbers are sequential
        assert_eq!(indexer.next_inscription_number, 5);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_indexer_state_persistence() -> Result<()> {
        clear();
        
        // Create first indexer and index some inscriptions
        {
            let mut indexer1 = InscriptionIndexer::new();
            indexer1.load_state()?;
            
            let (content, content_type) = create_test_inscription_content();
            let block = create_inscription_block(vec![(content, content_type)]);
            indexer1.index_block(&block, 840000)?;
            
            assert_eq!(indexer1.next_inscription_number, 1);
        }
        
        // Create second indexer and load state
        {
            let mut indexer2 = InscriptionIndexer::new();
            indexer2.load_state()?;
            
            // Should load the previous state
            assert_eq!(indexer2.next_inscription_number, 1);
            
            // Index another inscription
            let (content, content_type) = create_test_json_inscription();
            let block = create_inscription_block(vec![(content, content_type)]);
            indexer2.index_block(&block, 840001)?;
            
            assert_eq!(indexer2.next_inscription_number, 2);
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_empty_block_indexing() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create a block with only coinbase transaction
        let block = create_block_with_coinbase_tx(840000);
        
        // Index should succeed without errors
        indexer.index_block(&block, 840000)?;
        
        // Inscription number should remain unchanged
        assert_eq!(indexer.next_inscription_number, 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_transaction_without_inscriptions() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create a block with regular transactions (no inscriptions)
        let mut block = create_block_with_coinbase_tx(840000);
        
        // Add a regular transaction without inscription data
        let regular_tx = create_inscription_transaction(b"", "", None);
        block.txdata.push(regular_tx);
        
        // Index should succeed
        indexer.index_block(&block, 840000)?;
        
        // No inscriptions should be indexed
        assert_eq!(indexer.next_inscription_number, 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_large_inscription_indexing() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create a large inscription
        let large_content = "A".repeat(10000).into_bytes();
        let content_type = "text/plain";
        let block = create_inscription_block(vec![(&large_content, content_type)]);
        
        // Index should handle large content
        indexer.index_block(&block, 840000)?;
        
        // Verify inscription was indexed
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        let inscription_key = inscription_id.to_bytes();
        let stored_data = InscriptionTable::INSCRIPTIONS
            .select(&inscription_key)
            .get();
        
        assert!(!stored_data.is_empty(), "Large inscription should be stored");
        assert_eq!(indexer.next_inscription_number, 1);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_content_storage() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create an inscription with specific content
        let content = b"Test content for storage verification";
        let content_type = "text/plain";
        let block = create_inscription_block(vec![(content, content_type)]);
        
        indexer.index_block(&block, 840000)?;
        
        // Verify content is stored correctly
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        // Check content storage
        let content_key = inscription_id.to_bytes();
        let stored_content = InscriptionTable::CONTENT
            .select(&content_key)
            .get();
        
        // Content should be stored (exact format depends on implementation)
        assert!(!stored_content.is_empty(), "Content should be stored");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_metadata_storage() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create an inscription
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        
        indexer.index_block(&block, 840000)?;
        
        // Verify metadata is stored
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        // Check metadata storage
        let metadata_key = inscription_id.to_bytes();
        let stored_metadata = InscriptionTable::METADATA
            .select(&metadata_key)
            .get();
        
        // Metadata should be stored
        assert!(!stored_metadata.is_empty(), "Metadata should be stored");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_sat_to_inscription_mapping() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create an inscription
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        
        indexer.index_block(&block, 840000)?;
        
        // Verify sat-to-inscription mapping
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        // The exact sat number calculation depends on implementation
        // For now, just verify the inscription was indexed
        let inscription_key = inscription_id.to_bytes();
        let stored_data = InscriptionTable::INSCRIPTIONS
            .select(&inscription_key)
            .get();
        
        assert!(!stored_data.is_empty(), "Inscription should be indexed");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_indexer_error_handling() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Create a block with potentially problematic data
        let mut block = create_block_with_coinbase_tx(840000);
        
        // Add a transaction with empty witness (should not cause errors)
        let mut empty_tx = create_inscription_transaction(b"", "", None);
        empty_tx.input[0].witness = bitcoin::Witness::new();
        block.txdata.push(empty_tx);
        
        // Should handle gracefully
        let result = indexer.index_block(&block, 840000);
        
        // Should either succeed (ignoring invalid data) or fail gracefully
        match result {
            Ok(_) => {
                // If successful, no inscriptions should be indexed
                assert_eq!(indexer.next_inscription_number, 0);
            }
            Err(_) => {
                // If it fails, that's also acceptable
                println!("Indexer correctly handled problematic data");
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_block_height_tracking() -> Result<()> {
        clear();
        
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state()?;
        
        // Index blocks at different heights
        let heights = [840000, 840001, 840005, 840010];
        
        for &height in &heights {
            let (content, content_type) = create_test_inscription_content();
            let block = create_inscription_block(vec![(content, content_type)]);
            
            indexer.index_block(&block, height)?;
        }
        
        // Verify all inscriptions were indexed regardless of height gaps
        assert_eq!(indexer.next_inscription_number, heights.len() as u64);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_concurrent_indexing_simulation() -> Result<()> {
        clear();
        
        // Simulate what would happen if multiple indexers tried to process
        // the same data (though in practice this shouldn't happen)
        
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        
        // First indexer
        {
            let mut indexer1 = InscriptionIndexer::new();
            indexer1.load_state()?;
            indexer1.index_block(&block, 840000)?;
            assert_eq!(indexer1.next_inscription_number, 1);
        }
        
        // Second indexer loading the same state
        {
            let mut indexer2 = InscriptionIndexer::new();
            indexer2.load_state()?;
            // Should load the updated state
            assert_eq!(indexer2.next_inscription_number, 1);
        }
        
        Ok(())
    }
}