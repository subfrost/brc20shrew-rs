#[cfg(test)]
mod tests {
    use super::super::helpers::*;
    use crate::envelope::{Envelope, ParseError};
    use bitcoin::{Script, Witness};
    use wasm_bindgen_test::wasm_bindgen_test;
    use anyhow::Result;

    #[wasm_bindgen_test]
    fn test_valid_inscription_envelope_parsing() -> Result<()> {
        clear();
        
        // Create a valid inscription witness
        let content = b"Hello, Bitcoin!";
        let content_type = "text/plain";
        let witness = create_inscription_witness(content, content_type);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Verify parsed data
        assert_eq!(envelope.content_type.as_deref(), Some(content_type));
        assert_eq!(envelope.content.as_deref(), Some(content));
        assert!(envelope.is_valid());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_json_inscription_envelope() -> Result<()> {
        clear();
        
        // Create a JSON inscription
        let content = br#"{"name": "Test NFT", "description": "A test inscription"}"#;
        let content_type = "application/json";
        let witness = create_inscription_witness(content, content_type);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Verify parsed data
        assert_eq!(envelope.content_type.as_deref(), Some(content_type));
        assert_eq!(envelope.content.as_deref(), Some(content));
        assert!(envelope.is_valid());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_image_inscription_envelope() -> Result<()> {
        clear();
        
        // Create a mock image inscription
        let (content, content_type) = create_test_image_inscription();
        let witness = create_inscription_witness(&content, content_type);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Verify parsed data
        assert_eq!(envelope.content_type.as_deref(), Some(content_type));
        assert_eq!(envelope.content.as_deref(), Some(content.as_slice()));
        assert!(envelope.is_valid());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_large_content_envelope() -> Result<()> {
        clear();
        
        // Create a large inscription (> 520 bytes)
        let large_content = "A".repeat(1000).into_bytes();
        let content_type = "text/plain";
        let witness = create_inscription_witness(&large_content, content_type);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Verify parsed data
        assert_eq!(envelope.content_type.as_deref(), Some(content_type));
        assert_eq!(envelope.content.as_deref(), Some(large_content.as_slice()));
        assert!(envelope.is_valid());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_empty_content_envelope() -> Result<()> {
        clear();
        
        // Create an inscription with empty content
        let content = b"";
        let content_type = "text/plain";
        let witness = create_inscription_witness(content, content_type);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Verify parsed data
        assert_eq!(envelope.content_type.as_deref(), Some(content_type));
        assert_eq!(envelope.content.as_deref(), Some(content));
        assert!(envelope.is_valid());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_malformed_envelope_missing_ord_tag() -> Result<()> {
        clear();
        
        // Create a witness without the "ord" tag
        let mut witness = Witness::new();
        let mut script = Vec::new();
        
        // OP_FALSE OP_IF (missing "ord" tag)
        script.push(0x00);
        script.push(0x63);
        script.push(0x51); // OP_1
        script.push(0x09); // "text/plain"
        script.extend_from_slice(b"text/plain");
        script.push(0x00); // OP_0
        script.push(0x05); // "Hello"
        script.extend_from_slice(b"Hello");
        script.push(0x68); // OP_ENDIF
        
        witness.push(&script);
        
        // Parse should fail or return invalid envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        match Envelope::from_script(script) {
            Ok(envelope) => {
                assert!(!envelope.is_valid());
            }
            Err(_) => {
                // Expected to fail
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_malformed_envelope_missing_endif() -> Result<()> {
        clear();
        
        // Create a witness without OP_ENDIF
        let mut witness = Witness::new();
        let mut script = Vec::new();
        
        // OP_FALSE OP_IF "ord" OP_1 <content-type> OP_0 <content> (missing OP_ENDIF)
        script.push(0x00);
        script.push(0x63);
        script.push(0x03);
        script.extend_from_slice(b"ord");
        script.push(0x51);
        script.push(0x09);
        script.extend_from_slice(b"text/plain");
        script.push(0x00);
        script.push(0x05);
        script.extend_from_slice(b"Hello");
        // Missing OP_ENDIF
        
        witness.push(&script);
        
        // Parse should fail or return invalid envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        match Envelope::from_script(script) {
            Ok(envelope) => {
                assert!(!envelope.is_valid());
            }
            Err(_) => {
                // Expected to fail
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_envelope_with_no_content_type() -> Result<()> {
        clear();
        
        // Create a witness with content but no content type
        let mut witness = Witness::new();
        let mut script = Vec::new();
        
        // OP_FALSE OP_IF "ord" OP_0 <content> OP_ENDIF (missing content type)
        script.push(0x00);
        script.push(0x63);
        script.push(0x03);
        script.extend_from_slice(b"ord");
        script.push(0x00); // OP_0 (content tag)
        script.push(0x05); // "Hello"
        script.extend_from_slice(b"Hello");
        script.push(0x68); // OP_ENDIF
        
        witness.push(&script);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Should have content but no content type
        assert_eq!(envelope.content.as_deref(), Some(b"Hello"));
        assert!(envelope.content_type.is_none());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_envelope_with_multiple_content_chunks() -> Result<()> {
        clear();
        
        // Create a witness with multiple content chunks
        let mut witness = Witness::new();
        let mut script = Vec::new();
        
        // OP_FALSE OP_IF "ord" OP_1 <content-type> OP_0 <chunk1> <chunk2> OP_ENDIF
        script.push(0x00);
        script.push(0x63);
        script.push(0x03);
        script.extend_from_slice(b"ord");
        script.push(0x51); // OP_1
        script.push(0x09);
        script.extend_from_slice(b"text/plain");
        script.push(0x00); // OP_0
        script.push(0x05);
        script.extend_from_slice(b"Hello");
        script.push(0x07);
        script.extend_from_slice(b", World");
        script.push(0x01);
        script.extend_from_slice(b"!");
        script.push(0x68); // OP_ENDIF
        
        witness.push(&script);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Should combine all content chunks
        assert_eq!(envelope.content_type.as_deref(), Some("text/plain"));
        assert_eq!(envelope.content.as_deref(), Some(b"Hello, World!"));
        assert!(envelope.is_valid());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_envelope_with_metadata_fields() -> Result<()> {
        clear();
        
        // Create a witness with additional metadata fields
        let mut witness = Witness::new();
        let mut script = Vec::new();
        
        // OP_FALSE OP_IF "ord" OP_1 <content-type> OP_2 <metadata> OP_0 <content> OP_ENDIF
        script.push(0x00);
        script.push(0x63);
        script.push(0x03);
        script.extend_from_slice(b"ord");
        script.push(0x51); // OP_1 (content type)
        script.push(0x09);
        script.extend_from_slice(b"text/plain");
        script.push(0x52); // OP_2 (metadata field)
        script.push(0x0A);
        script.extend_from_slice(b"some_metadata");
        script.push(0x00); // OP_0 (content)
        script.push(0x05);
        script.extend_from_slice(b"Hello");
        script.push(0x68); // OP_ENDIF
        
        witness.push(&script);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Should parse content and content type, metadata handling depends on implementation
        assert_eq!(envelope.content_type.as_deref(), Some("text/plain"));
        assert_eq!(envelope.content.as_deref(), Some(b"Hello"));
        assert!(envelope.is_valid());
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_non_inscription_script() -> Result<()> {
        clear();
        
        // Create a regular script (not an inscription)
        let mut witness = Witness::new();
        let script = vec![0x51, 0x52, 0x53]; // OP_1 OP_2 OP_3
        witness.push(&script);
        
        // Parse should fail or return invalid envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        match Envelope::from_script(script) {
            Ok(envelope) => {
                assert!(!envelope.is_valid());
            }
            Err(_) => {
                // Expected to fail
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_envelope_serialization() -> Result<()> {
        clear();
        
        // Create a valid inscription envelope
        let content = b"Test content for serialization";
        let content_type = "text/plain";
        let witness = create_inscription_witness(content, content_type);
        
        // Parse the envelope
        let script = Script::from_bytes(&witness.to_vec()[0]);
        let envelope = Envelope::from_script(script)?;
        
        // Test serialization (if implemented)
        // This would test to_bytes() and from_bytes() methods
        // For now, just verify the envelope is valid
        assert!(envelope.is_valid());
        assert_eq!(envelope.content_type.as_deref(), Some(content_type));
        assert_eq!(envelope.content.as_deref(), Some(content));
        
        Ok(())
    }
}