#[cfg(test)]
mod tests {
    use super::super::helpers::*;
    use crate::inscription::{InscriptionId, InscriptionEntry, SatPoint, Charm, Rarity, Media};
    use bitcoin::{Txid, OutPoint};
    use bitcoin::hashes::Hash;
    use wasm_bindgen_test::wasm_bindgen_test;
    use anyhow::Result;

    #[wasm_bindgen_test]
    fn test_inscription_id_creation_and_serialization() -> Result<()> {
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
        let restored = InscriptionId::from_bytes(&bytes)?;
        assert_eq!(restored.txid, txid);
        assert_eq!(restored.index, index);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_id_edge_cases() -> Result<()> {
        clear();
        
        // Test with zero values
        let zero_txid = Txid::from_byte_array([0u8; 32]);
        let zero_id = InscriptionId::new(zero_txid, 0);
        
        let bytes = zero_id.to_bytes();
        let restored = InscriptionId::from_bytes(&bytes)?;
        assert_eq!(restored.txid, zero_txid);
        assert_eq!(restored.index, 0);
        
        // Test with max values
        let max_txid = Txid::from_byte_array([255u8; 32]);
        let max_id = InscriptionId::new(max_txid, u32::MAX);
        
        let bytes = max_id.to_bytes();
        let restored = InscriptionId::from_bytes(&bytes)?;
        assert_eq!(restored.txid, max_txid);
        assert_eq!(restored.index, u32::MAX);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_satpoint_creation_and_serialization() -> Result<()> {
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
        let restored = SatPoint::from_bytes(&bytes)?;
        assert_eq!(restored.outpoint.txid, txid);
        assert_eq!(restored.outpoint.vout, 1);
        assert_eq!(restored.offset, offset);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_satpoint_edge_cases() -> Result<()> {
        clear();
        
        // Test with zero offset
        let txid = Txid::from_byte_array([3u8; 32]);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        
        let bytes = satpoint.to_bytes();
        let restored = SatPoint::from_bytes(&bytes)?;
        assert_eq!(restored.offset, 0);
        
        // Test with large offset
        let large_offset = u64::MAX;
        let satpoint = SatPoint::new(outpoint, large_offset);
        
        let bytes = satpoint.to_bytes();
        let restored = SatPoint::from_bytes(&bytes)?;
        assert_eq!(restored.offset, large_offset);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_entry_creation() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([4u8; 32]);
        let inscription_id = InscriptionId::new(txid, 0);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 546);
        
        // Create inscription entry
        let entry = InscriptionEntry {
            id: inscription_id,
            number: 12345,
            satpoint,
            content_type: Some("text/plain".to_string()),
            content_length: Some(1024),
            timestamp: 1640995200, // 2022-01-01
            height: 720000,
            fee: 1000,
            charms: 0,
            rarity: Rarity::Common,
            media_type: MediaType::Text,
            content: Some(b"Hello, Bitcoin!".to_vec()),
        };
        
        // Verify fields
        assert_eq!(entry.id.txid, txid);
        assert_eq!(entry.number, 12345);
        assert_eq!(entry.content_type.as_deref(), Some("text/plain"));
        assert_eq!(entry.content_length, Some(1024));
        assert_eq!(entry.height, 720000);
        assert_eq!(entry.fee, 1000);
        assert_eq!(entry.rarity, Rarity::Common);
        assert_eq!(entry.media_type, MediaType::Text);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_entry_serialization() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([5u8; 32]);
        let inscription_id = InscriptionId::new(txid, 1);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 1000);
        
        let entry = InscriptionEntry {
            id: inscription_id,
            number: 999,
            satpoint,
            content_type: Some("application/json".to_string()),
            content_length: Some(256),
            timestamp: 1640995200,
            height: 720001,
            fee: 2000,
            charms: 0,
            rarity: Rarity::Uncommon,
            media_type: MediaType::Json,
            content: Some(br#"{"test": true}"#.to_vec()),
        };
        
        // Test serialization
        let bytes = entry.to_bytes()?;
        assert!(!bytes.is_empty());
        
        // Test deserialization
        let restored = InscriptionEntry::from_bytes(&bytes)?;
        assert_eq!(restored.id.txid, entry.id.txid);
        assert_eq!(restored.id.index, entry.id.index);
        assert_eq!(restored.number, entry.number);
        assert_eq!(restored.content_type, entry.content_type);
        assert_eq!(restored.content_length, entry.content_length);
        assert_eq!(restored.height, entry.height);
        assert_eq!(restored.fee, entry.fee);
        assert_eq!(restored.rarity, entry.rarity);
        assert_eq!(restored.media_type, entry.media_type);
        assert_eq!(restored.content, entry.content);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_charm_flags() -> Result<()> {
        clear();
        
        // Test individual charm flags
        assert_eq!(Charm::Cursed as u16, 1);
        assert_eq!(Charm::Reinscription as u16, 2);
        assert_eq!(Charm::Unbound as u16, 4);
        assert_eq!(Charm::Lost as u16, 8);
        assert_eq!(Charm::Nineball as u16, 16);
        assert_eq!(Charm::Vindicated as u16, 32);
        
        // Test combining charms
        let combined_charms = Charm::Cursed as u16 | Charm::Reinscription as u16;
        assert_eq!(combined_charms, 3);
        
        // Test checking for specific charms
        assert!(combined_charms & Charm::Cursed as u16 != 0);
        assert!(combined_charms & Charm::Reinscription as u16 != 0);
        assert!(combined_charms & Charm::Unbound as u16 == 0);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_rarity_enum() -> Result<()> {
        clear();
        
        // Test rarity values
        assert_eq!(Rarity::Common as u8, 0);
        assert_eq!(Rarity::Uncommon as u8, 1);
        assert_eq!(Rarity::Rare as u8, 2);
        assert_eq!(Rarity::Epic as u8, 3);
        assert_eq!(Rarity::Legendary as u8, 4);
        assert_eq!(Rarity::Mythic as u8, 5);
        
        // Test rarity ordering
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
        assert!(Rarity::Epic < Rarity::Legendary);
        assert!(Rarity::Legendary < Rarity::Mythic);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_media_type_detection() -> Result<()> {
        clear();
        
        // Test media type detection from content type
        assert_eq!(MediaType::from_content_type("text/plain"), MediaType::Text);
        assert_eq!(MediaType::from_content_type("text/html"), MediaType::Text);
        assert_eq!(MediaType::from_content_type("application/json"), MediaType::Json);
        assert_eq!(MediaType::from_content_type("image/png"), MediaType::Image);
        assert_eq!(MediaType::from_content_type("image/jpeg"), MediaType::Image);
        assert_eq!(MediaType::from_content_type("image/gif"), MediaType::Image);
        assert_eq!(MediaType::from_content_type("image/svg+xml"), MediaType::Image);
        assert_eq!(MediaType::from_content_type("image/webp"), MediaType::Image);
        assert_eq!(MediaType::from_content_type("audio/mpeg"), MediaType::Audio);
        assert_eq!(MediaType::from_content_type("audio/wav"), MediaType::Audio);
        assert_eq!(MediaType::from_content_type("video/mp4"), MediaType::Video);
        assert_eq!(MediaType::from_content_type("video/webm"), MediaType::Video);
        assert_eq!(MediaType::from_content_type("application/pdf"), MediaType::Pdf);
        assert_eq!(MediaType::from_content_type("model/gltf+json"), MediaType::Model);
        assert_eq!(MediaType::from_content_type("unknown/type"), MediaType::Unknown);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_entry_with_different_media_types() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([6u8; 32]);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        
        // Test different media types
        let media_types = vec![
            ("text/plain", MediaType::Text),
            ("application/json", MediaType::Json),
            ("image/png", MediaType::Image),
            ("audio/mpeg", MediaType::Audio),
            ("video/mp4", MediaType::Video),
            ("application/pdf", MediaType::Pdf),
            ("model/gltf+json", MediaType::Model),
            ("unknown/type", MediaType::Unknown),
        ];
        
        for (i, (content_type, expected_media_type)) in media_types.iter().enumerate() {
            let inscription_id = InscriptionId::new(txid, i as u32);
            
            let entry = InscriptionEntry {
                id: inscription_id,
                number: i as u64,
                satpoint,
                content_type: Some(content_type.to_string()),
                content_length: Some(100),
                timestamp: 1640995200,
                height: 720000,
                fee: 1000,
                charms: 0,
                rarity: Rarity::Common,
                media_type: *expected_media_type,
                content: Some(b"test content".to_vec()),
            };
            
            assert_eq!(entry.media_type, *expected_media_type);
            
            // Test serialization/deserialization
            let bytes = entry.to_bytes()?;
            let restored = InscriptionEntry::from_bytes(&bytes)?;
            assert_eq!(restored.media_type, *expected_media_type);
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_entry_with_charms() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([7u8; 32]);
        let inscription_id = InscriptionId::new(txid, 0);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        
        // Test with multiple charms
        let charms = Charm::Cursed as u16 | Charm::Reinscription as u16 | Charm::Unbound as u16;
        
        let entry = InscriptionEntry {
            id: inscription_id,
            number: 666,
            satpoint,
            content_type: Some("text/plain".to_string()),
            content_length: Some(13),
            timestamp: 1640995200,
            height: 720000,
            fee: 1000,
            charms,
            rarity: Rarity::Rare,
            media_type: MediaType::Text,
            content: Some(b"cursed content".to_vec()),
        };
        
        // Verify charms
        assert!(entry.charms & Charm::Cursed as u16 != 0);
        assert!(entry.charms & Charm::Reinscription as u16 != 0);
        assert!(entry.charms & Charm::Unbound as u16 != 0);
        assert!(entry.charms & Charm::Lost as u16 == 0);
        
        // Test serialization/deserialization
        let bytes = entry.to_bytes()?;
        let restored = InscriptionEntry::from_bytes(&bytes)?;
        assert_eq!(restored.charms, entry.charms);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_entry_minimal() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([8u8; 32]);
        let inscription_id = InscriptionId::new(txid, 0);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        
        // Test with minimal data (no content type, no content)
        let entry = InscriptionEntry {
            id: inscription_id,
            number: 0,
            satpoint,
            content_type: None,
            content_length: None,
            timestamp: 1640995200,
            height: 720000,
            fee: 1000,
            charms: 0,
            rarity: Rarity::Common,
            media_type: MediaType::Unknown,
            content: None,
        };
        
        // Test serialization/deserialization
        let bytes = entry.to_bytes()?;
        let restored = InscriptionEntry::from_bytes(&bytes)?;
        
        assert_eq!(restored.content_type, None);
        assert_eq!(restored.content_length, None);
        assert_eq!(restored.content, None);
        assert_eq!(restored.media_type, MediaType::Unknown);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_inscription_entry_large_content() -> Result<()> {
        clear();
        
        let txid = Txid::from_byte_array([9u8; 32]);
        let inscription_id = InscriptionId::new(txid, 0);
        let outpoint = OutPoint { txid, vout: 0 };
        let satpoint = SatPoint::new(outpoint, 0);
        
        // Test with large content
        let large_content = "A".repeat(100000).into_bytes();
        
        let entry = InscriptionEntry {
            id: inscription_id,
            number: 1,
            satpoint,
            content_type: Some("text/plain".to_string()),
            content_length: Some(large_content.len()),
            timestamp: 1640995200,
            height: 720000,
            fee: 10000, // Higher fee for large content
            charms: 0,
            rarity: Rarity::Common,
            media_type: MediaType::Text,
            content: Some(large_content.clone()),
        };
        
        // Test serialization/deserialization
        let bytes = entry.to_bytes()?;
        let restored = InscriptionEntry::from_bytes(&bytes)?;
        
        assert_eq!(restored.content_length, Some(large_content.len()));
        assert_eq!(restored.content, Some(large_content));
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_invalid_serialization_data() -> Result<()> {
        clear();
        
        // Test with invalid data
        let invalid_data = vec![1, 2, 3, 4, 5];
        
        // Should handle invalid data gracefully
        match InscriptionId::from_bytes(&invalid_data) {
            Ok(_) => panic!("Should not succeed with invalid data"),
            Err(_) => {
                // Expected to fail
            }
        }
        
        match SatPoint::from_bytes(&invalid_data) {
            Ok(_) => panic!("Should not succeed with invalid data"),
            Err(_) => {
                // Expected to fail
            }
        }
        
        match InscriptionEntry::from_bytes(&invalid_data) {
            Ok(_) => panic!("Should not succeed with invalid data"),
            Err(_) => {
                // Expected to fail
            }
        }
        
        Ok(())
    }
}