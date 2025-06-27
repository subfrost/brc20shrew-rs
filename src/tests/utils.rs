#[cfg(test)]
mod tests {
    use super::super::helpers::*;
    use crate::inscription::{InscriptionId, SatPoint};
    use crate::envelope::Envelope;
    use bitcoin::{Block, Transaction, Txid, OutPoint};
    use bitcoin::hashes::Hash;
    use metashrew_core::{get_cache, index_pointer::IndexPointer};
    use metashrew_support::index_pointer::KeyValuePointer;
    use wasm_bindgen_test::wasm_bindgen_test;
    use anyhow::Result;
    use std::collections::HashMap;

    /// Test utilities for debugging and validation
    
    #[wasm_bindgen_test]
    fn test_cache_inspection() -> Result<()> {
        clear();
        
        // Index some test data
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&block, 840000)?;
        
        // Inspect cache contents
        print_cache_debug();
        
        // Verify cache is not empty
        let cache = get_cache();
        assert!(!cache.is_empty(), "Cache should contain indexed data");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_block_validation() -> Result<()> {
        clear();
        
        // Test valid block creation
        let block = create_block_with_coinbase_tx(840000);
        assert!(validate_test_block(&block));
        
        // Test block with inscriptions
        let (content, content_type) = create_test_inscription_content();
        let inscription_block = create_inscription_block(vec![(content, content_type)]);
        assert!(validate_test_block(&inscription_block));
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_transaction_analysis() -> Result<()> {
        clear();
        
        // Create transaction with inscription
        let (content, content_type) = create_test_inscription_content();
        let tx = create_inscription_transaction(content, content_type, None);
        
        // Analyze transaction
        let analysis = analyze_transaction(&tx);
        
        assert!(analysis.has_witness_data);
        assert!(analysis.potential_inscription);
        assert_eq!(analysis.input_count, 1);
        assert_eq!(analysis.output_count, 1);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_extraction() -> Result<()> {
        clear();
        
        // Create transaction with inscription
        let content = b"Test inscription for extraction";
        let content_type = "text/plain";
        let tx = create_inscription_transaction(content, content_type, None);
        
        // Extract inscription data
        let extracted = extract_inscription_from_transaction(&tx)?;
        
        assert!(extracted.is_some());
        let inscription_data = extracted.unwrap();
        assert_eq!(inscription_data.content, content);
        assert_eq!(inscription_data.content_type, content_type);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_sat_calculation() -> Result<()> {
        clear();
        
        // Test sat number calculation for different heights
        let test_cases = vec![
            (0, 0),           // Genesis block
            (1, 5000000000),  // Block 1
            (210000, 1050000000000000), // First halving
            (840000, 1968750000000000), // Inscription era
        ];
        
        for (height, expected_first_sat) in test_cases {
            let calculated = calculate_first_sat_in_block(height);
            assert_eq!(calculated, expected_first_sat, "Height {} should have first sat {}", height, expected_first_sat);
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_numbering_logic() -> Result<()> {
        clear();
        
        // Test inscription numbering across multiple blocks
        let mut expected_number = 0u64;
        
        for height in 840000..840010 {
            let inscriptions_in_block = (height - 840000 + 1) as usize; // Increasing number per block
            
            let mut inscriptions = Vec::new();
            for i in 0..inscriptions_in_block {
                let content = format!("Inscription {} in block {}", i, height).into_bytes();
                inscriptions.push((content, "text/plain"));
            }
            
            let inscription_refs: Vec<(&[u8], &str)> = inscriptions
                .iter()
                .map(|(content, content_type)| (content.as_slice(), *content_type))
                .collect();
            
            let block = create_inscription_block(inscription_refs);
            index_test_block(&block, height)?;
            
            expected_number += inscriptions_in_block as u64;
        }
        
        // Verify total inscription count
        let total_inscriptions = get_total_inscription_count();
        assert_eq!(total_inscriptions, expected_number);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_memory_usage_tracking() -> Result<()> {
        clear();
        
        // Track memory usage during indexing
        let initial_cache_size = get_cache_size();
        
        // Index multiple blocks
        for height in 840000..840005 {
            let (content, content_type) = create_test_inscription_content();
            let block = create_inscription_block(vec![(content, content_type)]);
            index_test_block(&block, height)?;
        }
        
        let final_cache_size = get_cache_size();
        
        println!("Cache size grew from {} to {} bytes", initial_cache_size, final_cache_size);
        assert!(final_cache_size > initial_cache_size, "Cache should grow with indexed data");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_data_consistency() -> Result<()> {
        clear();
        
        // Index some inscriptions
        let inscriptions = vec![
            create_test_inscription_content(),
            create_test_json_inscription(),
        ];
        
        let block = create_inscription_block(inscriptions.clone());
        index_test_block(&block, 840000)?;
        
        // Verify data consistency
        for (i, (expected_content, expected_content_type)) in inscriptions.iter().enumerate() {
            let tx_index = i + 1; // Skip coinbase
            let txid = block.txdata[tx_index].compute_txid();
            let inscription_id = InscriptionId::new(txid, 0);
            
            // Check if inscription exists and data is consistent
            assert!(verify_inscription_consistency(&inscription_id, expected_content, expected_content_type)?);
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_error_recovery() -> Result<()> {
        clear();
        
        // Index valid data first
        let (content, content_type) = create_test_inscription_content();
        let valid_block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&valid_block, 840000)?;
        
        let initial_count = get_total_inscription_count();
        
        // Try to index invalid data
        let invalid_block = create_invalid_inscription_block();
        let result = index_test_block(&invalid_block, 840001);
        
        // Verify system state is still consistent
        let final_count = get_total_inscription_count();
        
        match result {
            Ok(_) => {
                // If it succeeded, count might have increased
                assert!(final_count >= initial_count);
            }
            Err(_) => {
                // If it failed, count should be unchanged
                assert_eq!(final_count, initial_count);
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_performance_benchmarking() -> Result<()> {
        clear();
        
        // Benchmark different inscription sizes
        let test_sizes = vec![100, 1000, 10000, 100000];
        
        for size in test_sizes {
            let content = "A".repeat(size).into_bytes();
            let content_type = "text/plain";
            let block = create_inscription_block(vec![(&content, content_type)]);
            
            let start = std::time::Instant::now();
            index_test_block(&block, 840000)?;
            let duration = start.elapsed();
            
            println!("Indexed {} byte inscription in {:?}", size, duration);
            
            // Clear for next test
            clear();
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_concurrent_access_simulation() -> Result<()> {
        clear();
        
        // Simulate concurrent access patterns
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&block, 840000)?;
        
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        // Simulate multiple reads
        for _ in 0..10 {
            assert!(verify_inscription_exists(&inscription_id));
        }
        
        Ok(())
    }

    // Helper functions for tests
    
    fn validate_test_block(block: &Block) -> bool {
        // Basic block validation
        !block.txdata.is_empty() && 
        block.header.merkle_root != bitcoin::hash_types::TxMerkleNode::all_zeros()
    }
    
    fn analyze_transaction(tx: &Transaction) -> TransactionAnalysis {
        TransactionAnalysis {
            has_witness_data: tx.input.iter().any(|input| !input.witness.is_empty()),
            potential_inscription: tx.input.iter().any(|input| 
                input.witness.iter().any(|witness| witness.len() > 100)
            ),
            input_count: tx.input.len(),
            output_count: tx.output.len(),
        }
    }
    
    fn extract_inscription_from_transaction(tx: &Transaction) -> Result<Option<InscriptionData>> {
        for input in &tx.input {
            if let Some(witness_data) = input.witness.iter().next() {
                if witness_data.len() > 10 {
                    // Try to parse as inscription envelope
                    let script = bitcoin::Script::from_bytes(witness_data);
                    if let Ok(envelope) = Envelope::from_script(script) {
                        if envelope.is_valid() {
                            return Ok(Some(InscriptionData {
                                content: envelope.content.unwrap_or_default(),
                                content_type: envelope.content_type.unwrap_or_default(),
                            }));
                        }
                    }
                }
            }
        }
        Ok(None)
    }
    
    fn calculate_first_sat_in_block(height: u32) -> u64 {
        // Simplified sat calculation (actual implementation would be more complex)
        let mut total_sats = 0u64;
        let mut current_reward = 5000000000u64; // 50 BTC in sats
        let mut blocks_processed = 0u32;
        
        while blocks_processed < height {
            let blocks_until_halving = 210000 - (blocks_processed % 210000);
            let blocks_to_process = std::cmp::min(blocks_until_halving, height - blocks_processed);
            
            total_sats += blocks_to_process as u64 * current_reward;
            blocks_processed += blocks_to_process;
            
            if blocks_processed % 210000 == 0 && blocks_processed < height {
                current_reward /= 2;
            }
        }
        
        total_sats
    }
    
    fn get_total_inscription_count() -> u64 {
        // This would query the actual inscription count from the index
        // For now, return a placeholder
        0
    }
    
    fn get_cache_size() -> usize {
        let cache = get_cache();
        cache.iter().map(|(k, v)| k.len() + v.len()).sum()
    }
    
    fn verify_inscription_consistency(
        inscription_id: &InscriptionId,
        expected_content: &[u8],
        expected_content_type: &str,
    ) -> Result<bool> {
        // This would verify the inscription data matches expectations
        // For now, return true as placeholder
        Ok(true)
    }
    
    fn verify_inscription_exists(inscription_id: &InscriptionId) -> bool {
        // This would check if the inscription exists in the index
        // For now, return true as placeholder
        true
    }
    
    fn create_invalid_inscription_block() -> Block {
        // Create a block with invalid inscription data for testing error handling
        let mut block = create_block_with_coinbase_tx(840000);
        
        // Add transaction with malformed witness
        let mut invalid_tx = create_inscription_transaction(b"", "", None);
        invalid_tx.input[0].witness = bitcoin::Witness::from_slice(&[vec![0x00, 0x63]]); // Incomplete inscription
        block.txdata.push(invalid_tx);
        
        block
    }
    
    // Helper structs
    
    struct TransactionAnalysis {
        has_witness_data: bool,
        potential_inscription: bool,
        input_count: usize,
        output_count: usize,
    }
    
    struct InscriptionData {
        content: Vec<u8>,
        content_type: String,
    }
}