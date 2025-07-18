//! Test suite for shrewscriptions-rs
//!
//! ## How to Write Tests (Following alkanes-rs Pattern)
//!
//! ### Key Principles:
//! 1. **Use wasm_bindgen_test**: All tests use `#[wasm_bindgen_test]` attribute, not `#[wasm_bindgen_test]`
//! 2. **Always call clear()**: Start every test with `clear()` to reset metashrew state
//! 3. **Use test helpers**: Create Bitcoin blocks/transactions using helper functions
//! 4. **Test indexing logic**: Verify that our indexing matches ord reference implementation
//!
//! ### Test Runner Configuration:
//! Tests run using wasm-bindgen-test-runner configured in `.cargo/config.toml`:
//! ```toml
//! [target.wasm32-unknown-unknown]
//! runner = "wasm-bindgen-test-runner"
//! ```
//!
//! Run tests with: `cargo test` (automatically uses wasm-bindgen-test-runner)
//!
//! ### Test Structure Pattern:
//! ```rust
//! #[wasm_bindgen_test]
//! fn test_something() -> Result<()> {
//!     clear(); // ALWAYS start with this
//!
//!     // Create test data using helpers
//!     let block = create_test_block();
//!
//!     // Index the data
//!     let mut indexer = InscriptionIndexer::new();
//!     indexer.index_block(&block, height)?;
//!
//!     // Verify results using assertions
//!     assert_eq!(expected, actual);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Helper Functions:
//! - `clear()`: Reset metashrew state (REQUIRED at start of every test)
//! - `create_test_transaction()`: Create mock Bitcoin transaction
//! - `create_inscription_envelope()`: Create inscription witness data
//! - `create_block_with_coinbase_tx()`: Create test block with coinbase
//!
//! ### Testing Guidelines:
//! - Test one specific behavior per test function
//! - Use descriptive test names that explain what is being tested
//! - Always verify both positive and negative cases
//! - Test edge cases and error conditions
//! - Follow the same patterns as alkanes-rs tests in ./reference/alkanes-rs/src/tests/

#[cfg(test)]
mod tests {
    use crate::tests::helpers::*;
    use crate::inscription::{InscriptionId, SatPoint, Charm, Rarity, Media};
    use bitcoin::{Txid, OutPoint};
    use bitcoin::hashes::Hash;
    use wasm_bindgen_test::wasm_bindgen_test;
    use anyhow::Result;

    #[wasm_bindgen_test]
    fn test_inscription_id_basic() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([1u8; 32]);
        let index = 42u32;
        
        // Create inscription ID
        let inscription_id = InscriptionId::new(txid, index);
        
        // Verify fields
        assert_eq!(inscription_id.txid, txid);
        assert_eq!(inscription_id.index, index);
        
        // Test serialization
        let bytes = inscription_id.to_bytes();
        assert_eq!(bytes.len(), 36); // 32 bytes for txid + 4 bytes for index
        
        // Test deserialization
        let restored = InscriptionId::from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize InscriptionId: {}", e))?;
        assert_eq!(restored.txid, txid);
        assert_eq!(restored.index, index);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_satpoint_basic() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([2u8; 32]);
        let outpoint = OutPoint { txid, vout: 1 };
        let offset = 12345u64;
        
        // Create SatPoint
        let satpoint = SatPoint::new(outpoint, offset);
        
        // Verify fields
        assert_eq!(satpoint.outpoint.txid, txid);
        assert_eq!(satpoint.outpoint.vout, 1);
        assert_eq!(satpoint.offset, offset);
        
        // Test serialization
        let bytes = satpoint.to_bytes();
        assert_eq!(bytes.len(), 44); // 32 bytes txid + 4 bytes vout + 8 bytes offset
        
        // Test deserialization
        let restored = SatPoint::from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize SatPoint: {}", e))?;
        assert_eq!(restored.outpoint.txid, txid);
        assert_eq!(restored.outpoint.vout, 1);
        assert_eq!(restored.offset, offset);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_media_type_detection() -> Result<()> {
        clear();
        
        // Test media type detection from content type
        assert_eq!(Media::from_content_type("text/plain"), Media::Text);
        assert_eq!(Media::from_content_type("text/html"), Media::Iframe);
        assert_eq!(Media::from_content_type("application/json"), Media::Code);
        assert_eq!(Media::from_content_type("image/png"), Media::Image);
        assert_eq!(Media::from_content_type("image/jpeg"), Media::Image);
        assert_eq!(Media::from_content_type("audio/mpeg"), Media::Audio);
        assert_eq!(Media::from_content_type("video/mp4"), Media::Video);
        assert_eq!(Media::from_content_type("application/pdf"), Media::Pdf);
        assert_eq!(Media::from_content_type("model/gltf+json"), Media::Model);
        assert_eq!(Media::from_content_type("unknown/type"), Media::Unknown);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_rarity_enum() -> Result<()> {
        clear();
        
        // Test rarity ordering
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
        assert!(Rarity::Epic < Rarity::Legendary);
        
        // Test rarity from sat
        assert_eq!(Rarity::from_sat(0), Rarity::Mythic);
        assert_eq!(Rarity::from_sat(1), Rarity::Common);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_charm_enum() -> Result<()> {
        clear();
        
        // Test charm values
        let charms = Charm::all();
        assert!(charms.len() > 0);
        
        // Test charm names
        assert_eq!(Charm::Cursed.name(), "cursed");
        assert_eq!(Charm::Rare.name(), "rare");
        assert_eq!(Charm::Epic.name(), "epic");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_witness_creation() -> Result<()> {
        clear();
        
        // Test creating inscription witness
        let content = b"Hello, Bitcoin!";
        let content_type = "text/plain";
        let witness = create_inscription_envelope(content_type.as_bytes(), content);
        
        // Verify witness is not empty
        assert!(!witness.is_empty());
        assert!(witness.to_vec().len() > 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_block_creation() -> Result<()> {
        clear();
        
        // Test creating a basic block
        let block = create_block_with_coinbase_tx(840000);
        
        // Verify block structure
        assert_eq!(block.txdata.len(), 1); // Only coinbase
        assert!(block.header.merkle_root != bitcoin::hash_types::TxMerkleNode::all_zeros());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_block_creation() -> Result<()> {
        clear();
        
        // Test creating a block with inscriptions
        let inscriptions = vec![
            (b"Hello, Bitcoin!" as &[u8], "text/plain"),
            (br#"{"test": true}"#, "application/json"),
        ];
        
        let block = create_inscription_block(inscriptions);
        
        // Verify block structure
        assert_eq!(block.txdata.len(), 3); // Coinbase + 2 inscription txs
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_test_addresses() -> Result<()> {
        clear();
        
        // Test that test addresses are valid
        let addr1 = get_test_address(0);
        let addr2 = get_test_address(1);
        
        // Verify addresses are different
        assert_ne!(addr1.script_pubkey(), addr2.script_pubkey());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_mock_outpoint_creation() -> Result<()> {
        clear();
        
        // Test creating mock outpoints
        let outpoint1 = create_mock_outpoint(1);
        let outpoint2 = create_mock_outpoint(2);
        
        // Verify outpoints are different
        assert_ne!(outpoint1.txid, outpoint2.txid);
        assert_eq!(outpoint1.vout, 0);
        assert_eq!(outpoint2.vout, 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_test_content_helpers() -> Result<()> {
        clear();
        
        // Test content helper functions
        let (content, content_type) = create_test_inscription_content();
        assert!(!content.is_empty());
        assert_eq!(content_type, "text/plain");
        
        let (json_content, json_type) = create_test_json_inscription();
        assert!(!json_content.is_empty());
        assert_eq!(json_type, "application/json");
        
        let (image_content, image_type) = create_test_image_inscription();
        assert!(!image_content.is_empty());
        assert_eq!(image_type, "image/png");
        
        Ok(())
    }
}