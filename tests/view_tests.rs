use shrewscriptions_rs::{
    indexer::InscriptionIndexer,
    inscription::{InscriptionId, InscriptionEntry},
    proto::shrewscriptions::{
        InscriptionRequest, InscriptionsRequest, ChildrenRequest, ParentsRequest,
        ContentRequest, MetadataRequest, SatRequest, SatInscriptionsRequest,
        UtxoRequest, BlockHashAtHeightRequest, BlockHeightRequest, TxRequest,
    },
    view::*,
    tables::TABLES,
};
use bitcoin::{Txid, BlockHash};

mod test_utils;
use test_utils::{TestUtils, TestAssertions};

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_data() {
        // Clear any existing data
        // Note: In a real implementation, you'd want to use a test database
        
        let mut indexer = InscriptionIndexer::new();
        
        // Create and index a test block with inscription
        let block = TestUtils::create_test_block_with_inscription();
        indexer.index_block(&block, 800000).unwrap();
        
        // Create and index a block with multiple inscriptions
        let multi_block = TestUtils::create_test_block_with_multiple_inscriptions();
        indexer.index_block(&multi_block, 800001).unwrap();
        
        // Create and index a block with cursed inscription
        let cursed_block = TestUtils::create_test_block_with_cursed_inscription();
        indexer.index_block(&cursed_block, 800002).unwrap();
    }

    #[test]
    fn test_get_inscription_by_id() {
        setup_test_data();
        
        let block = TestUtils::create_test_block_with_inscription();
        let inscription_id = InscriptionId::new(block.txs[1].txid(), 0);
        
        let mut request = InscriptionRequest::new();
        request.set_id(inscription_id.to_string());
        
        let response = get_inscription(&request).unwrap();
        assert!(response.has_inscription());
        
        let info = response.get_inscription();
        assert_eq!(info.get_id(), inscription_id.to_string());
        assert_eq!(info.get_number(), 1); // First blessed inscription
        assert_eq!(info.get_height(), 800000);
        assert_eq!(info.get_content_type(), "text/plain");
    }

    #[test]
    fn test_get_inscription_by_number() {
        setup_test_data();
        
        let mut request = InscriptionRequest::new();
        request.set_number(1); // First blessed inscription
        
        let response = get_inscription(&request).unwrap();
        assert!(response.has_inscription());
        
        let info = response.get_inscription();
        assert_eq!(info.get_number(), 1);
        assert_eq!(info.get_height(), 800000);
        assert_eq!(info.get_content_type(), "text/plain");
    }

    #[test]
    fn test_get_inscription_not_found() {
        setup_test_data();
        
        let mut request = InscriptionRequest::new();
        request.set_id("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefi999".to_string());
        
        let response = get_inscription(&request);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_get_inscription_invalid_request() {
        let request = InscriptionRequest::new(); // No id or number set
        
        let response = get_inscription(&request);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("must be specified"));
    }

    #[test]
    fn test_get_inscriptions_all() {
        setup_test_data();
        
        let request = InscriptionsRequest::new();
        let response = get_inscriptions(&request).unwrap();
        
        assert!(response.get_total() > 0);
        assert!(!response.get_inscriptions().is_empty());
        assert_eq!(response.get_limit(), 100); // Default limit
        assert_eq!(response.get_offset(), 0); // Default offset
    }

    #[test]
    fn test_get_inscriptions_with_pagination() {
        setup_test_data();
        
        let mut request = InscriptionsRequest::new();
        request.set_limit(2);
        request.set_offset(1);
        
        let response = get_inscriptions(&request).unwrap();
        
        assert_eq!(response.get_limit(), 2);
        assert_eq!(response.get_offset(), 1);
        assert!(response.get_inscriptions().len() <= 2);
    }

    #[test]
    fn test_get_inscriptions_by_height() {
        setup_test_data();
        
        let mut request = InscriptionsRequest::new();
        request.set_height(800001); // Block with multiple inscriptions
        
        let response = get_inscriptions(&request).unwrap();
        
        // Should return inscriptions from that specific height
        for inscription in response.get_inscriptions() {
            assert_eq!(inscription.get_height(), 800001);
        }
    }

    #[test]
    fn test_get_inscriptions_by_content_type() {
        setup_test_data();
        
        let mut request = InscriptionsRequest::new();
        request.set_content_type("text/plain".to_string());
        
        let response = get_inscriptions(&request).unwrap();
        
        // Should return inscriptions with that content type
        for inscription in response.get_inscriptions() {
            assert_eq!(inscription.get_content_type(), "text/plain");
        }
    }

    #[test]
    fn test_get_children() {
        setup_test_data();
        
        // Create parent inscription
        let parent_block = TestUtils::create_test_block_with_inscription();
        let parent_id = InscriptionId::new(parent_block.txs[1].txid(), 0);
        
        // Create child inscription
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state().unwrap();
        let child_block = TestUtils::create_test_block_with_child_inscription(&parent_id);
        indexer.index_block(&child_block, 800003).unwrap();
        
        let mut request = ChildrenRequest::new();
        request.set_id(parent_id.to_string());
        
        let response = get_children(&request).unwrap();
        
        assert!(!response.get_children().is_empty());
        
        let child_id = InscriptionId::new(child_block.txs[1].txid(), 0);
        assert!(response.get_children().contains(&child_id.to_string()));
    }

    #[test]
    fn test_get_children_no_children() {
        setup_test_data();
        
        let block = TestUtils::create_test_block_with_inscription();
        let inscription_id = InscriptionId::new(block.txs[1].txid(), 0);
        
        let mut request = ChildrenRequest::new();
        request.set_id(inscription_id.to_string());
        
        let response = get_children(&request).unwrap();
        
        assert!(response.get_children().is_empty());
    }

    #[test]
    fn test_get_parents() {
        setup_test_data();
        
        // Create parent inscription
        let parent_block = TestUtils::create_test_block_with_inscription();
        let parent_id = InscriptionId::new(parent_block.txs[1].txid(), 0);
        
        // Create child inscription
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state().unwrap();
        let child_block = TestUtils::create_test_block_with_child_inscription(&parent_id);
        indexer.index_block(&child_block, 800003).unwrap();
        
        let child_id = InscriptionId::new(child_block.txs[1].txid(), 0);
        
        let mut request = ParentsRequest::new();
        request.set_id(child_id.to_string());
        
        let response = get_parents(&request).unwrap();
        
        assert!(!response.get_parents().is_empty());
        assert!(response.get_parents().contains(&parent_id.to_string()));
    }

    #[test]
    fn test_get_content() {
        setup_test_data();
        
        let block = TestUtils::create_test_block_with_inscription();
        let inscription_id = InscriptionId::new(block.txs[1].txid(), 0);
        
        let mut request = ContentRequest::new();
        request.set_id(inscription_id.to_string());
        
        let response = get_content(&request).unwrap();
        
        assert_eq!(response.get_content(), b"Hello, world!");
        assert_eq!(response.get_content_type(), "text/plain");
    }

    #[test]
    fn test_get_content_not_found() {
        let mut request = ContentRequest::new();
        request.set_id("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefi999".to_string());
        
        let response = get_content(&request);
        assert!(response.is_err());
    }

    #[test]
    fn test_get_metadata() {
        setup_test_data();
        
        // Create inscription with metadata
        let mut indexer = InscriptionIndexer::new();
        indexer.load_state().unwrap();
        let metadata_block = TestUtils::create_test_block_with_metadata();
        indexer.index_block(&metadata_block, 800003).unwrap();
        
        let inscription_id = InscriptionId::new(metadata_block.txs[1].txid(), 0);
        
        let mut request = MetadataRequest::new();
        request.set_id(inscription_id.to_string());
        
        let response = get_metadata(&request).unwrap();
        
        let expected_metadata = b"{\"name\": \"Test NFT\", \"description\": \"A test inscription\"}";
        assert_eq!(response.get_metadata(), expected_metadata);
    }

    #[test]
    fn test_get_sat() {
        let mut request = SatRequest::new();
        request.set_sat(1000000000); // 1 billion sats
        
        let response = get_sat(&request).unwrap();
        
        assert!(response.has_sat_info());
        let info = response.get_sat_info();
        assert_eq!(info.get_sat(), 1000000000);
        assert_eq!(info.get_rarity(), "common"); // Should be common rarity
    }

    #[test]
    fn test_get_sat_rare() {
        let mut request = SatRequest::new();
        request.set_sat(0); // Genesis sat (mythic)
        
        let response = get_sat(&request).unwrap();
        
        let info = response.get_sat_info();
        assert_eq!(info.get_sat(), 0);
        assert_eq!(info.get_rarity(), "mythic");
    }

    #[test]
    fn test_get_sat_inscriptions() {
        setup_test_data();
        
        // This test would require setting up sat tracking
        let mut request = SatInscriptionsRequest::new();
        request.set_sat(1000000000);
        
        let response = get_sat_inscriptions(&request).unwrap();
        
        // Should return empty list if no inscriptions on this sat
        assert!(response.get_inscription_ids().is_empty());
    }

    #[test]
    fn test_get_utxo() {
        setup_test_data();
        
        let block = TestUtils::create_test_block_with_inscription();
        let txid = block.txs[1].txid();
        let outpoint = format!("{}:0", txid);
        
        let mut request = UtxoRequest::new();
        request.set_outpoint(outpoint.clone());
        
        let response = get_utxo(&request).unwrap();
        
        assert!(response.has_utxo_info());
        let info = response.get_utxo_info();
        assert_eq!(info.get_outpoint(), outpoint);
        // Should have inscription IDs if inscriptions are on this UTXO
    }

    #[test]
    fn test_get_utxo_invalid_format() {
        let mut request = UtxoRequest::new();
        request.set_outpoint("invalid_format".to_string());
        
        let response = get_utxo(&request);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("Invalid outpoint format"));
    }

    #[test]
    fn test_get_block_hash_at_height() {
        setup_test_data();
        
        let mut request = BlockHashAtHeightRequest::new();
        request.set_height(800000);
        
        let response = get_block_hash_at_height(&request).unwrap();
        
        assert!(!response.get_block_hash().is_empty());
        
        // Verify it's a valid block hash format
        let block_hash = BlockHash::from_hex(response.get_block_hash());
        assert!(block_hash.is_ok());
    }

    #[test]
    fn test_get_block_height() {
        setup_test_data();
        
        let block = TestUtils::create_test_block_with_inscription();
        let block_hash = block.block_hash();
        
        let mut request = BlockHeightRequest::new();
        request.set_block_hash(block_hash.to_string());
        
        let response = get_block_height(&request).unwrap();
        
        assert_eq!(response.get_height(), 800000);
    }

    #[test]
    fn test_get_block_height_invalid_hash() {
        let mut request = BlockHeightRequest::new();
        request.set_block_hash("invalid_hash".to_string());
        
        let response = get_block_height(&request);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("Invalid block hash"));
    }

    #[test]
    fn test_get_tx() {
        setup_test_data();
        
        let block = TestUtils::create_test_block_with_inscription();
        let txid = block.txs[1].txid();
        
        let mut request = TxRequest::new();
        request.set_txid(txid.to_string());
        
        let response = get_tx(&request).unwrap();
        
        assert!(response.has_tx_info());
        let info = response.get_tx_info();
        assert_eq!(info.get_txid(), txid.to_string());
        assert!(!info.get_inscription_ids().is_empty());
    }

    #[test]
    fn test_get_tx_invalid_txid() {
        let mut request = TxRequest::new();
        request.set_txid("invalid_txid".to_string());
        
        let response = get_tx(&request);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("Invalid txid"));
    }

    #[test]
    fn test_parse_inscription_id_valid() {
        let id_str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefi42";
        let result = super::parse_inscription_id(id_str);
        
        assert!(result.is_ok());
        let id = result.unwrap();
        assert_eq!(id.index, 42);
    }

    #[test]
    fn test_parse_inscription_id_invalid_format() {
        let id_str = "invalid_format";
        let result = super::parse_inscription_id(id_str);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid inscription ID format"));
    }

    #[test]
    fn test_parse_inscription_id_invalid_txid() {
        let id_str = "invalid_txidi42";
        let result = super::parse_inscription_id(id_str);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid txid"));
    }

    #[test]
    fn test_parse_inscription_id_invalid_index() {
        let id_str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefiabc";
        let result = super::parse_inscription_id(id_str);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid index"));
    }

    #[test]
    fn test_get_inscriptions_empty_database() {
        // Test with empty database
        let request = InscriptionsRequest::new();
        let response = get_inscriptions(&request).unwrap();
        
        assert_eq!(response.get_total(), 0);
        assert!(response.get_inscriptions().is_empty());
    }

    #[test]
    fn test_get_children_inscription_not_found() {
        let mut request = ChildrenRequest::new();
        request.set_id("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefi999".to_string());
        
        let response = get_children(&request);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_get_parents_inscription_not_found() {
        let mut request = ParentsRequest::new();
        request.set_id("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefi999".to_string());
        
        let response = get_parents(&request);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_get_metadata_no_metadata() {
        setup_test_data();
        
        let block = TestUtils::create_test_block_with_inscription();
        let inscription_id = InscriptionId::new(block.txs[1].txid(), 0);
        
        let mut request = MetadataRequest::new();
        request.set_id(inscription_id.to_string());
        
        let response = get_metadata(&request).unwrap();
        
        // Should return empty metadata
        assert!(response.get_metadata().is_empty());
    }
}