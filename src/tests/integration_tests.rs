#[cfg(test)]
mod tests {
    use super::super::helpers::*;
    use crate::indexer::InscriptionIndexer;
    use crate::inscription::InscriptionId;
    use anyhow::Result;
    use bitcoin::hashes::Hash;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn test_basic_inscription_indexing() -> Result<()> {
        clear();
        
        // Create a block with a simple text inscription
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        
        // Index the block
        index_test_block(&block, 840000)?;
        
        // Verify the inscription was indexed
        let txid = block.txdata[1].compute_txid(); // First non-coinbase tx
        assert_inscription_indexed(txid, 0, content_type, content.len())?;
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_multiple_inscriptions_in_block() -> Result<()> {
        clear();
        
        // Create a block with multiple inscriptions
        let inscriptions = vec![
            create_test_inscription_content(),
            create_test_json_inscription(),
        ];
        
        let block = create_inscription_block(inscriptions.clone());
        
        // Index the block
        index_test_block(&block, 840000)?;
        
        // Verify all inscriptions were indexed
        for (i, (content, content_type)) in inscriptions.iter().enumerate() {
            let tx_index = i + 1; // Skip coinbase
            let txid = block.txdata[tx_index].compute_txid();
            assert_inscription_indexed(txid, 0, content_type, content.len())?;
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_chain_processing() -> Result<()> {
        clear();
        
        // Create a chain of blocks with inscriptions
        let mut blocks = vec![create_block_with_coinbase_tx(840000)];
        
        // Add blocks with inscriptions
        for i in 1..=5 {
            let height = 840000 + i;
            let (content, content_type) = create_test_inscription_content();
            let block = create_inscription_block(vec![(content, content_type)]);
            blocks.push(block);
        }
        
        // Process the entire chain
        index_test_chain(&blocks, 840000)?;
        
        // Verify inscriptions from each block
        for (block_index, block) in blocks.iter().enumerate().skip(1) {
            if block.txdata.len() > 1 {
                let txid = block.txdata[1].compute_txid();
                let (content, content_type) = create_test_inscription_content();
                assert_inscription_indexed(txid, 0, content_type, content.len())?;
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_large_inscription_content() -> Result<()> {
        clear();
        
        // Create a large inscription (> 520 bytes to test chunking)
        let large_content = "A".repeat(1000).into_bytes();
        let content_type = "text/plain";
        
        let block = create_inscription_block(vec![(&large_content, content_type)]);
        
        // Index the block
        index_test_block(&block, 840000)?;
        
        // Verify the large inscription was indexed
        let txid = block.txdata[1].compute_txid();
        assert_inscription_indexed(txid, 0, content_type, large_content.len())?;
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_json_inscription_indexing() -> Result<()> {
        clear();
        
        // Create a JSON inscription
        let (content, content_type) = create_test_json_inscription();
        let block = create_inscription_block(vec![(content, content_type)]);
        
        // Index the block
        index_test_block(&block, 840000)?;
        
        // Verify the JSON inscription was indexed
        let txid = block.txdata[1].compute_txid();
        assert_inscription_indexed(txid, 0, content_type, content.len())?;
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_image_inscription_indexing() -> Result<()> {
        clear();
        
        // Create a mock image inscription
        let (content, content_type) = create_test_image_inscription();
        let block = create_inscription_block(vec![(&content, content_type)]);
        
        // Index the block
        index_test_block(&block, 840000)?;
        
        // Verify the image inscription was indexed
        let txid = block.txdata[1].compute_txid();
        assert_inscription_indexed(txid, 0, content_type, content.len())?;
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_empty_block_processing() -> Result<()> {
        clear();
        
        // Create a block with only coinbase transaction
        let block = create_block_with_coinbase_tx(840000);
        
        // Index the block (should not error)
        index_test_block(&block, 840000)?;
        
        // Verify state is still clean
        assert_clean_state();
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_indexer_state_persistence() -> Result<()> {
        clear();
        
        // Create and index first block
        let (content1, content_type1) = create_test_inscription_content();
        let block1 = create_inscription_block(vec![(content1, content_type1)]);
        index_test_block(&block1, 840000)?;
        
        // Create and index second block
        let (content2, content_type2) = create_test_json_inscription();
        let block2 = create_inscription_block(vec![(content2, content_type2)]);
        index_test_block(&block2, 840001)?;
        
        // Verify both inscriptions are still accessible
        let txid1 = block1.txdata[1].compute_txid();
        let txid2 = block2.txdata[1].compute_txid();
        
        assert_inscription_indexed(txid1, 0, content_type1, content1.len())?;
        assert_inscription_indexed(txid2, 0, content_type2, content2.len())?;
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_numbering() -> Result<()> {
        clear();
        
        // Create multiple blocks with inscriptions to test numbering
        let mut expected_number = 0u64;
        
        for block_height in 840000..840005 {
            let (content, content_type) = create_test_inscription_content();
            let block = create_inscription_block(vec![(content, content_type)]);
            
            index_test_block(&block, block_height)?;
            
            // Verify inscription number increments
            let txid = block.txdata[1].compute_txid();
            let inscription_id = InscriptionId::new(txid, 0);
            
            // This would check the inscription number once view functions are implemented
            expected_number += 1;
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_malformed_inscription_handling() -> Result<()> {
        clear();
        
        // Create a transaction with malformed inscription data
        let mut block = create_block_with_coinbase_tx(840000);
        
        // Add a transaction with invalid witness data
        let invalid_tx = create_inscription_transaction(b"", "", None);
        block.txdata.push(invalid_tx);
        
        // Index should handle malformed data gracefully
        let result = index_test_block(&block, 840000);
        
        // Should either succeed (ignoring malformed data) or fail gracefully
        match result {
            Ok(_) => {
                // If it succeeds, verify no inscription was created
                assert_clean_state();
            }
            Err(_) => {
                // If it fails, that's also acceptable for malformed data
                println!("Indexer correctly rejected malformed inscription");
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_duplicate_inscription_handling() -> Result<()> {
        clear();
        
        // Create identical inscriptions in the same block
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![
            (content, content_type),
            (content, content_type), // Duplicate
        ]);
        
        // Index the block
        index_test_block(&block, 840000)?;
        
        // Verify both inscriptions were indexed with different IDs
        let txid1 = block.txdata[1].compute_txid();
        let txid2 = block.txdata[2].compute_txid();
        
        assert_inscription_indexed(txid1, 0, content_type, content.len())?;
        assert_inscription_indexed(txid2, 0, content_type, content.len())?;
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_performance_large_block() -> Result<()> {
        clear();
        
        // Create a block with many inscriptions to test performance
        let mut inscriptions = Vec::new();
        for i in 0..50 {
            let content = format!("Inscription number {}", i).into_bytes();
            inscriptions.push((content, "text/plain"));
        }
        
        // Convert to references for the function call
        let inscription_refs: Vec<(&[u8], &str)> = inscriptions
            .iter()
            .map(|(content, content_type)| (content.as_slice(), *content_type))
            .collect();
        
        let block = create_inscription_block(inscription_refs);
        
        // Measure indexing time
        let start = std::time::Instant::now();
        index_test_block(&block, 840000)?;
        let duration = start.elapsed();
        
        println!("Indexed {} inscriptions in {:?}", inscriptions.len(), duration);
        
        // Verify a few inscriptions were indexed correctly
        for i in [0, 25, 49] {
            let tx_index = i + 1; // Skip coinbase
            let txid = block.txdata[tx_index].compute_txid();
            let expected_content = format!("Inscription number {}", i);
            assert_inscription_indexed(txid, 0, "text/plain", expected_content.len())?;
        }
        
        Ok(())
    }
}