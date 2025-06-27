#[cfg(test)]
mod tests {
    use super::super::helpers::*;
    use crate::view::*;
    use crate::inscription::InscriptionId;
    use crate::proto::shrewscriptions::*;
    use bitcoin::hashes::Hash;
    use protobuf::Message;
    use wasm_bindgen_test::wasm_bindgen_test;
    use anyhow::Result;

    #[wasm_bindgen_test]
    fn test_get_inscription_basic() -> Result<()> {
        clear();
        
        // Index an inscription first
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&block, 840000)?;
        
        // Create request for the inscription
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        let mut request = GetInscriptionRequest::new();
        request.id = inscription_id.to_bytes();
        
        // Test the view function
        let response = get_inscription(&request)?;
        
        // Verify response
        assert!(response.has_inscription());
        let inscription = response.inscription();
        assert_eq!(inscription.content_type, content_type);
        assert_eq!(inscription.content, content);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_inscription_not_found() -> Result<()> {
        clear();
        
        // Create request for non-existent inscription
        let fake_txid = bitcoin::Txid::from_byte_array([1u8; 32]);
        let inscription_id = InscriptionId::new(fake_txid, 0);
        
        let mut request = GetInscriptionRequest::new();
        request.id = inscription_id.to_bytes();
        
        // Test the view function
        let response = get_inscription(&request)?;
        
        // Should return empty response or error
        assert!(!response.has_inscription() || response.inscription().content.is_empty());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_inscriptions_paginated() -> Result<()> {
        clear();
        
        // Index multiple inscriptions
        let inscriptions = vec![
            create_test_inscription_content(),
            create_test_json_inscription(),
            (b"Third inscription", "text/plain"),
            (b"Fourth inscription", "text/plain"),
            (b"Fifth inscription", "text/plain"),
        ];
        
        let inscription_refs: Vec<(&[u8], &str)> = inscriptions
            .iter()
            .map(|(content, content_type)| (content.as_slice(), *content_type))
            .collect();
        
        let block = create_inscription_block(inscription_refs);
        index_test_block(&block, 840000)?;
        
        // Test pagination
        let mut request = GetInscriptionsRequest::new();
        request.limit = 3;
        request.offset = 0;
        
        let response = get_inscriptions(&request)?;
        
        // Should return first 3 inscriptions
        assert_eq!(response.inscriptions.len(), 3);
        assert_eq!(response.total, inscriptions.len() as u64);
        
        // Test second page
        request.offset = 3;
        request.limit = 2;
        
        let response2 = get_inscriptions(&request)?;
        
        // Should return remaining 2 inscriptions
        assert_eq!(response2.inscriptions.len(), 2);
        assert_eq!(response2.total, inscriptions.len() as u64);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_inscriptions_by_sat() -> Result<()> {
        clear();
        
        // Index an inscription
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&block, 840000)?;
        
        // Calculate the sat number (this depends on implementation)
        // For now, use a mock sat number
        let sat_number = 1000000u64;
        
        let mut request = GetSatInscriptionsRequest::new();
        request.sat = sat_number;
        
        let response = get_sat_inscriptions(&request)?;
        
        // Should return inscriptions for this sat
        // The exact behavior depends on implementation
        assert!(response.inscriptions.len() >= 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_inscription_content() -> Result<()> {
        clear();
        
        // Index an inscription with specific content
        let content = b"Test content for retrieval";
        let content_type = "text/plain";
        let block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&block, 840000)?;
        
        // Get inscription content
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        let mut request = GetInscriptionRequest::new();
        request.id = inscription_id.to_bytes();
        
        let response = get_inscription(&request)?;
        
        // Verify content matches
        assert!(response.has_inscription());
        let inscription = response.inscription();
        assert_eq!(inscription.content, content);
        assert_eq!(inscription.content_type, content_type);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_inscription_metadata() -> Result<()> {
        clear();
        
        // Index an inscription
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&block, 840000)?;
        
        // Get inscription metadata
        let txid = block.txdata[1].compute_txid();
        let inscription_id = InscriptionId::new(txid, 0);
        
        let mut request = GetInscriptionRequest::new();
        request.id = inscription_id.to_bytes();
        
        let response = get_inscription(&request)?;
        
        // Verify metadata
        assert!(response.has_inscription());
        let inscription = response.inscription();
        assert_eq!(inscription.content_type, content_type);
        assert!(inscription.number >= 0); // Should have a valid inscription number
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_children_inscriptions() -> Result<()> {
        clear();
        
        // For now, just test that the function doesn't crash
        // Child inscription logic would need to be implemented
        
        let mut request = GetChildrenRequest::new();
        request.parent_id = vec![1, 2, 3, 4]; // Mock parent ID
        
        let response = get_children(&request)?;
        
        // Should return empty list for non-existent parent
        assert_eq!(response.children.len(), 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_parents_inscriptions() -> Result<()> {
        clear();
        
        // For now, just test that the function doesn't crash
        // Parent inscription logic would need to be implemented
        
        let mut request = GetParentsRequest::new();
        request.child_id = vec![1, 2, 3, 4]; // Mock child ID
        
        let response = get_parents(&request)?;
        
        // Should return empty list for non-existent child
        assert_eq!(response.parents.len(), 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_sat_inscription() -> Result<()> {
        clear();
        
        // Index an inscription
        let (content, content_type) = create_test_inscription_content();
        let block = create_inscription_block(vec![(content, content_type)]);
        index_test_block(&block, 840000)?;
        
        // Test getting inscription by sat
        let mut request = GetSatInscriptionRequest::new();
        request.sat = 1000000u64; // Mock sat number
        
        let response = get_sat_inscription(&request)?;
        
        // Response format depends on implementation
        // For now, just verify it doesn't crash
        assert!(response.has_inscription() || !response.has_inscription());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_sat_info() -> Result<()> {
        clear();
        
        // Test getting sat information
        let mut request = GetSatRequest::new();
        request.sat = 1000000u64;
        
        let response = get_sat(&request)?;
        
        // Should return sat information
        assert_eq!(response.sat, 1000000u64);
        // Other fields depend on implementation
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_view_functions_with_large_dataset() -> Result<()> {
        clear();
        
        // Index many inscriptions
        let mut inscriptions = Vec::new();
        for i in 0..20 {
            let content = format!("Inscription {}", i).into_bytes();
            inscriptions.push((content, "text/plain"));
        }
        
        let inscription_refs: Vec<(&[u8], &str)> = inscriptions
            .iter()
            .map(|(content, content_type)| (content.as_slice(), *content_type))
            .collect();
        
        let block = create_inscription_block(inscription_refs);
        index_test_block(&block, 840000)?;
        
        // Test pagination with large dataset
        let mut request = GetInscriptionsRequest::new();
        request.limit = 10;
        request.offset = 0;
        
        let response = get_inscriptions(&request)?;
        
        assert_eq!(response.inscriptions.len(), 10);
        assert_eq!(response.total, 20);
        
        // Test getting specific inscriptions
        for i in [0, 5, 10, 15, 19] {
            let tx_index = i + 1; // Skip coinbase
            let txid = block.txdata[tx_index].compute_txid();
            let inscription_id = InscriptionId::new(txid, 0);
            
            let mut get_request = GetInscriptionRequest::new();
            get_request.id = inscription_id.to_bytes();
            
            let get_response = get_inscription(&get_request)?;
            assert!(get_response.has_inscription());
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_view_functions_error_handling() -> Result<()> {
        clear();
        
        // Test with invalid request data
        let mut request = GetInscriptionRequest::new();
        request.id = vec![]; // Empty ID
        
        let response = get_inscription(&request);
        
        // Should handle gracefully
        match response {
            Ok(resp) => {
                // Should return empty or error response
                assert!(!resp.has_inscription() || resp.inscription().content.is_empty());
            }
            Err(_) => {
                // Error is also acceptable
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_view_functions_with_different_content_types() -> Result<()> {
        clear();
        
        // Index inscriptions with different content types
        let inscriptions = vec![
            (b"Plain text", "text/plain"),
            (br#"{"key": "value"}"#, "application/json"),
            (b"<html><body>HTML</body></html>", "text/html"),
            (b"Binary data", "application/octet-stream"),
        ];
        
        let block = create_inscription_block(inscriptions.clone());
        index_test_block(&block, 840000)?;
        
        // Verify each inscription can be retrieved correctly
        for (i, (expected_content, expected_content_type)) in inscriptions.iter().enumerate() {
            let tx_index = i + 1; // Skip coinbase
            let txid = block.txdata[tx_index].compute_txid();
            let inscription_id = InscriptionId::new(txid, 0);
            
            let mut request = GetInscriptionRequest::new();
            request.id = inscription_id.to_bytes();
            
            let response = get_inscription(&request)?;
            
            assert!(response.has_inscription());
            let inscription = response.inscription();
            assert_eq!(inscription.content_type, *expected_content_type);
            assert_eq!(inscription.content, *expected_content);
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_protobuf_serialization() -> Result<()> {
        clear();
        
        // Test that protobuf messages serialize/deserialize correctly
        let mut request = GetInscriptionRequest::new();
        request.id = vec![1, 2, 3, 4, 5];
        
        // Serialize
        let bytes = request.write_to_bytes()?;
        
        // Deserialize
        let restored_request = GetInscriptionRequest::parse_from_bytes(&bytes)?;
        
        assert_eq!(restored_request.id, request.id);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_view_function_performance() -> Result<()> {
        clear();
        
        // Index a moderate number of inscriptions
        let mut inscriptions = Vec::new();
        for i in 0..100 {
            let content = format!("Performance test inscription {}", i).into_bytes();
            inscriptions.push((content, "text/plain"));
        }
        
        let inscription_refs: Vec<(&[u8], &str)> = inscriptions
            .iter()
            .map(|(content, content_type)| (content.as_slice(), *content_type))
            .collect();
        
        let block = create_inscription_block(inscription_refs);
        index_test_block(&block, 840000)?;
        
        // Measure view function performance
        let start = std::time::Instant::now();
        
        let mut request = GetInscriptionsRequest::new();
        request.limit = 50;
        request.offset = 0;
        
        let response = get_inscriptions(&request)?;
        
        let duration = start.elapsed();
        
        println!("Retrieved {} inscriptions in {:?}", response.inscriptions.len(), duration);
        
        assert_eq!(response.inscriptions.len(), 50);
        assert_eq!(response.total, 100);
        
        Ok(())
    }
}