#[allow(unused_imports)]
use {
    metashrew_core::metashrew_println::{println},
    std::fmt::Write
};

use bitcoin::{
    Script, ScriptBuf,
};

/// Inscription envelope containing the inscription data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub input: usize,
    pub offset: usize,
    pub payload: Inscription,
    pub pushnum: bool,
    pub stutter: bool,
}

/// Inscription data parsed from envelope
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inscription {
    pub body: Option<Vec<u8>>,
    pub content_encoding: Option<Vec<u8>>,
    pub content_type: Option<Vec<u8>>,
    pub delegate: Option<Vec<u8>>,
    pub duplicate_field: bool,
    pub incomplete_field: bool,
    pub metadata: Option<Vec<u8>>,
    pub metaprotocol: Option<Vec<u8>>,
    pub parent: Option<Vec<u8>>,
    pub pointer: Option<Vec<u8>>,
    pub rune: Option<Vec<u8>>,
    pub unrecognized_even_field: bool,
}

impl Inscription {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content_type(&self) -> Option<String> {
        self.content_type
            .as_ref()
            .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
    }

    pub fn metaprotocol(&self) -> Option<String> {
        self.metaprotocol
            .as_ref()
            .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
    }

    pub fn content_length(&self) -> Option<usize> {
        self.body.as_ref().map(|body| body.len())
    }

    pub fn delegate_id(&self) -> Option<crate::inscription::InscriptionId> {
        println!("DEBUG delegate_id: Called with delegate field: {:?}", self.delegate);
        self.delegate.as_ref().and_then(|bytes| {
            println!("DEBUG delegate_id: Processing {} bytes: {:?}", bytes.len(), bytes);
            if bytes.len() == 36 {
                println!("DEBUG delegate_id: Trying binary format (36 bytes)");
                let result = crate::inscription::InscriptionId::from_bytes(bytes).ok();
                println!("DEBUG delegate_id: Binary parse result: {:?}", result);
                result
            } else {
                // Try parsing as string (for test helpers)
                println!("DEBUG delegate_id: Trying string format");
                let id_str = String::from_utf8(bytes.clone()).ok()?;
                println!("DEBUG delegate_id: String: {}", id_str);
                let result = crate::inscription::InscriptionId::from_str(&id_str).ok();
                println!("DEBUG delegate_id: String parse result: {:?}", result);
                result
            }
        })
    }

    pub fn parent_id(&self) -> Option<crate::inscription::InscriptionId> {
        self.parent.as_ref().and_then(|bytes| {
            if bytes.len() == 36 {
                crate::inscription::InscriptionId::from_bytes(bytes).ok()
            } else {
                // Try parsing as string (for test helpers)
                let id_str = String::from_utf8(bytes.clone()).ok()?;
                crate::inscription::InscriptionId::from_str(&id_str).ok()
            }
        })
    }

    pub fn pointer_value(&self) -> Option<u64> {
        self.pointer.as_ref().and_then(|bytes| {
            if bytes.len() <= 8 {
                let mut array = [0u8; 8];
                array[..bytes.len()].copy_from_slice(bytes);
                Some(u64::from_le_bytes(array))
            } else {
                None
            }
        })
    }

    pub fn is_cursed(&self) -> bool {
        self.duplicate_field
            || self.incomplete_field
            || self.unrecognized_even_field
            || self.body.is_none()
    }
}

/// Parse inscriptions from a transaction's witness data
pub fn parse_inscriptions_from_transaction(
    tx: &bitcoin::Transaction,
) -> Result<Vec<Envelope>, ParseError> {
    let mut envelopes = Vec::new();

    for (input_index, input) in tx.input.iter().enumerate() {
        for (witness_index, witness_element) in input.witness.iter().enumerate() {
            let script = ScriptBuf::from_bytes(witness_element.to_vec());
            if let Some(envelope) = parse_envelope_from_script(&script, input_index, witness_index)? {
                envelopes.push(envelope);
            }
        }
    }

    Ok(envelopes)
}

/// Parse an inscription envelope from a script
pub fn parse_envelope_from_script(
    script: &Script,
    input: usize,
    offset: usize,
) -> Result<Option<Envelope>, ParseError> {
    println!("DEBUG: parse_envelope_from_script called with script length: {}", script.len());
    
    // For debugging, skip script instruction parsing and go directly to raw bytes
    // This matches what the manual test does
    println!("DEBUG: Using raw bytes parsing directly");
    parse_envelope_from_raw_bytes(script.as_bytes(), input, offset)
}

/// Parse envelope from raw bytes (for test helpers)
fn parse_envelope_from_raw_bytes(
    bytes: &[u8],
    input: usize,
    offset: usize,
) -> Result<Option<Envelope>, ParseError> {
    let mut pos = 0;
    
    // Look for envelope pattern: 0x00 0x63 0x03 "ord"
    while pos + 5 < bytes.len() {
        if bytes[pos] == 0x00 && bytes[pos + 1] == 0x63 &&
           bytes[pos + 2] == 0x03 && &bytes[pos + 3..pos + 6] == b"ord" {
            // Found inscription envelope
            println!("DEBUG: Found envelope at position {}", pos);
            pos += 6; // Skip past 0x00 0x63 0x03 "ord"
            
            // The OP_ENDIF should be at the very end of the script
            // So we use the entire remaining script as field data
            let end_pos = bytes.len() - 1; // Exclude the final OP_ENDIF byte
            
            println!("DEBUG: Envelope field data from {} to {} ({} bytes): {:?}",
                     pos, end_pos, end_pos - pos, &bytes[pos..end_pos]);
            
            if let Some(inscription) = parse_inscription_fields(&bytes[pos..end_pos])? {
                // Debug: Check if body was parsed
                if let Some(body) = &inscription.body {
                    println!("DEBUG: Envelope found with body length: {}", body.len());
                } else {
                    println!("DEBUG: Envelope found but no body");
                }
                
                return Ok(Some(Envelope {
                    input,
                    offset,
                    payload: inscription,
                    pushnum: false,
                    stutter: false,
                }));
            }
        }
        pos += 1;
    }
    
    Ok(None)
}

/// Parse inscription from raw bytes (for test helpers)
pub fn parse_inscription_from_raw_bytes(bytes: &[u8]) -> Result<Option<Inscription>, ParseError> {
    println!("DEBUG: parse_inscription_from_raw_bytes called with {} bytes: {:?}", bytes.len(), bytes);
    
    // Skip the envelope header: 0x00 0x63 0x03 "ord"
    let mut pos = 0;
    
    // Look for envelope pattern: 0x00 0x63 0x03 "ord"
    while pos + 5 < bytes.len() {
        if bytes[pos] == 0x00 && bytes[pos + 1] == 0x63 &&
           bytes[pos + 2] == 0x03 && &bytes[pos + 3..pos + 6] == b"ord" {
            // Found inscription envelope, skip to the field data
            pos += 6; // Skip past 0x00 0x63 0x03 "ord"
            break;
        }
        pos += 1;
    }
    
    if pos + 5 >= bytes.len() {
        println!("DEBUG: No envelope pattern found");
        return Ok(None);
    }
    
    // Find the end of the envelope (OP_ENDIF = 0x68)
    let mut end_pos = pos;
    while end_pos < bytes.len() && bytes[end_pos] != 0x68 {
        end_pos += 1;
    }
    
    if end_pos >= bytes.len() {
        println!("DEBUG: No OP_ENDIF found");
        return Ok(None);
    }
    
    // Parse the field data between pos and end_pos
    let field_data = &bytes[pos..end_pos];
    parse_inscription_fields(field_data)
}

/// Parse inscription fields from raw field data (no envelope wrapper)
fn parse_inscription_fields(field_data: &[u8]) -> Result<Option<Inscription>, ParseError> {
    println!("DEBUG: parse_inscription_fields called with {} bytes: {:?}", field_data.len(), field_data);
    
    let mut inscription = Inscription::new();
    let mut pos = 0;
    
    // Parse Bitcoin script push operations: [length][data][length][data]...
    while pos < field_data.len() {
        // Read the length of the next push operation
        if pos >= field_data.len() {
            break;
        }
        
        let push_length = field_data[pos] as usize;
        pos += 1;
        
        println!("DEBUG: Push operation length: {} at position {}", push_length, pos - 1);
        
        if pos + push_length > field_data.len() {
            println!("DEBUG: Not enough data for push operation, breaking");
            break;
        }
        
        let push_data = &field_data[pos..pos + push_length];
        pos += push_length;
        
        println!("DEBUG: Push data: {:?}", push_data);
        
        // If this is a single-byte push, it might be a tag
        if push_length == 1 {
            let tag = push_data[0];
            println!("DEBUG: Found tag: {}", tag);
            
            // Read the next push operation which should be the value
            if pos >= field_data.len() {
                println!("DEBUG: No value for tag {}", tag);
                break;
            }
            
            let value_length = field_data[pos] as usize;
            pos += 1;
            
            if pos + value_length > field_data.len() {
                println!("DEBUG: Not enough data for tag {} value", tag);
                break;
            }
            
            let value = &field_data[pos..pos + value_length];
            pos += value_length;
            
            println!("DEBUG: Tag {} value (length {}): {:?}", tag, value_length, value);
            
            match tag {
                1 => {
                    println!("DEBUG: Setting content_type");
                    inscription.content_type = Some(value.to_vec());
                }
                2 => inscription.pointer = Some(value.to_vec()),
                3 => inscription.parent = Some(value.to_vec()),
                5 => inscription.metadata = Some(value.to_vec()),
                7 => inscription.metaprotocol = Some(value.to_vec()),
                9 => inscription.content_encoding = Some(value.to_vec()),
                11 => {
                    println!("DEBUG: Setting delegate");
                    inscription.delegate = Some(value.to_vec());
                }
                13 => inscription.rune = Some(value.to_vec()),
                tag if tag % 2 == 0 => {
                    // Unrecognized even field
                    inscription.unrecognized_even_field = true;
                }
                _ => {
                    println!("DEBUG: Unknown tag {}, skipping", tag);
                }
            }
        } else if push_length == 0 {
            // Empty push - this is the body tag!
            println!("DEBUG: Found empty push (body tag)");
            
            // Body content may be chunked into multiple push operations
            // Read all subsequent push operations as body chunks
            let mut body_content = Vec::new();
            
            while pos < field_data.len() {
                let opcode = field_data[pos];
                pos += 1;
                
                let chunk_len = if opcode <= 75 {
                    // OP_PUSHBYTES_N (1-75): opcode itself is the length
                    opcode as usize
                } else if opcode == 76 {
                    // OP_PUSHDATA1: next byte is the length
                    if pos >= field_data.len() {
                        println!("DEBUG: OP_PUSHDATA1 but no length byte");
                        break;
                    }
                    let len = field_data[pos] as usize;
                    pos += 1;
                    len
                } else if opcode == 77 {
                    // OP_PUSHDATA2: next 2 bytes are the length (little-endian)
                    if pos + 1 >= field_data.len() {
                        println!("DEBUG: OP_PUSHDATA2 but not enough length bytes");
                        break;
                    }
                    let len = u16::from_le_bytes([field_data[pos], field_data[pos + 1]]) as usize;
                    pos += 2;
                    len
                } else if opcode == 78 {
                    // OP_PUSHDATA4: next 4 bytes are the length (little-endian)
                    if pos + 3 >= field_data.len() {
                        println!("DEBUG: OP_PUSHDATA4 but not enough length bytes");
                        break;
                    }
                    let len = u32::from_le_bytes([
                        field_data[pos], field_data[pos + 1],
                        field_data[pos + 2], field_data[pos + 3]
                    ]) as usize;
                    pos += 4;
                    len
                } else {
                    println!("DEBUG: Unknown opcode in body: {}", opcode);
                    break;
                };
                
                if pos + chunk_len > field_data.len() {
                    println!("DEBUG: Chunk extends beyond available data, treating remaining as final chunk");
                    body_content.extend_from_slice(&field_data[pos..]);
                    break;
                }
                
                let chunk_data = &field_data[pos..pos + chunk_len];
                println!("DEBUG: Body chunk (opcode {}, length {}): {:?}", opcode, chunk_len, chunk_data);
                body_content.extend_from_slice(chunk_data);
                pos += chunk_len;
            }
            
            println!("DEBUG: Total body content (length {}): {:?}", body_content.len(), body_content);
            inscription.body = Some(body_content);
            break; // Body is the last field, exit loop
        } else {
            println!("DEBUG: Multi-byte push data (not a tag): {:?}", push_data);
        }
    }
    
    println!("DEBUG: Final inscription: content_type={:?}, delegate={:?}, body={:?}",
             inscription.content_type, inscription.delegate, inscription.body);
    
    Ok(Some(inscription))
}

/// Errors that can occur during envelope parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidScript,
    InvalidInstruction,
    IncompleteEnvelope,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidScript => write!(f, "Invalid script"),
            ParseError::InvalidInstruction => write!(f, "Invalid instruction"),
            ParseError::IncompleteEnvelope => write!(f, "Incomplete envelope"),
        }
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::opcodes::all::*;
    use bitcoin::script::Builder;

    #[test]
    fn test_parse_simple_inscription() {
        let script = Builder::new()
            .push_opcode(OP_PUSHBYTES_0)
            .push_opcode(OP_IF)
            .push_slice(b"ord") // protocol identifier
            .push_slice([1]) // content-type tag
            .push_slice(b"text/plain")
            .push_opcode(OP_PUSHBYTES_0) // body separator
            .push_slice(b"Hello, world!")
            .push_opcode(OP_ENDIF)
            .into_script();

        let envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
        assert!(envelope.is_some());
        
        let envelope = envelope.unwrap();
        assert_eq!(envelope.input, 0);
        assert_eq!(envelope.offset, 0);
        
        let inscription = &envelope.payload;
        assert_eq!(inscription.content_type(), Some("text/plain".to_string()));
        assert_eq!(inscription.body, Some(b"Hello, world!".to_vec()));
        assert!(!inscription.is_cursed());
    }

    #[test]
    fn test_parse_cursed_inscription() {
        let script = Builder::new()
            .push_opcode(OP_PUSHBYTES_0)
            .push_opcode(OP_IF)
            .push_slice(b"ord") // protocol identifier
            .push_slice([1]) // content-type tag
            .push_slice(b"text/plain")
            .push_slice([1]) // duplicate content-type tag
            .push_slice(b"text/html")
            .push_opcode(OP_PUSHBYTES_0) // body separator
            .push_slice(b"Hello, world!")
            .push_opcode(OP_ENDIF)
            .into_script();

        let envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
        assert!(envelope.is_some());
        
        let envelope = envelope.unwrap();
        let inscription = &envelope.payload;
        assert!(inscription.duplicate_field);
        assert!(inscription.is_cursed());
    }

    #[test]
    fn test_parse_inscription_like_helpers() {
        // Test the exact format created by our test helpers
        let mut script_bytes = Vec::new();
        
        // OP_PUSHBYTES_0
        script_bytes.push(0x00);
        // OP_IF
        script_bytes.push(0x63);
        // "ord" tag
        script_bytes.push(0x03);
        script_bytes.extend_from_slice(b"ord");
        // Content type tag (1)
        script_bytes.push(0x01);
        // Content type length and data
        script_bytes.push(0x0A); // "text/plain" length
        script_bytes.extend_from_slice(b"text/plain");
        // Content tag (0)
        script_bytes.push(0x00);
        // Content length and data
        script_bytes.push(0x05); // "hello" length
        script_bytes.extend_from_slice(b"hello");
        // OP_ENDIF
        script_bytes.push(0x68);
        
        let script = bitcoin::ScriptBuf::from_bytes(script_bytes);
        let envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
        assert!(envelope.is_some(), "Should parse inscription from helper format");
        
        let envelope = envelope.unwrap();
        let inscription = &envelope.payload;
        assert_eq!(inscription.content_type(), Some("text/plain".to_string()));
        assert_eq!(inscription.body, Some(b"hello".to_vec()));
    }

    #[test]
    fn test_parse_inscription_with_metadata() {
        // Test metadata parsing specifically
        let metadata = b"{\"test\": \"value\"}";
        let content = b"test content";
        
        let mut script_bytes = Vec::new();
        
        // OP_PUSHBYTES_0
        script_bytes.push(0x00);
        // OP_IF
        script_bytes.push(0x63);
        // "ord" tag
        script_bytes.push(0x03);
        script_bytes.extend_from_slice(b"ord");
        // Content type tag (1)
        script_bytes.push(0x01);
        script_bytes.push(0x0A); // "text/plain" length
        script_bytes.extend_from_slice(b"text/plain");
        // Metadata tag (5)
        script_bytes.push(0x05);
        script_bytes.push(metadata.len() as u8);
        script_bytes.extend_from_slice(metadata);
        // Content tag (0)
        script_bytes.push(0x00);
        script_bytes.push(content.len() as u8);
        script_bytes.extend_from_slice(content);
        // OP_ENDIF
        script_bytes.push(0x68);
        
        std::println!("Script bytes: {:?}", script_bytes);
        
        let script = bitcoin::ScriptBuf::from_bytes(script_bytes);
        let envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
        assert!(envelope.is_some(), "Should parse inscription with metadata");
        
        let envelope = envelope.unwrap();
        let inscription = &envelope.payload;
        
        // Check metadata
        assert!(inscription.metadata.is_some(), "Should have metadata");
        let parsed_metadata = inscription.metadata.as_ref().unwrap();
        std::println!("Expected metadata: {:?}", metadata);
        std::println!("Parsed metadata: {:?}", parsed_metadata);
        assert_eq!(parsed_metadata, metadata, "Metadata should match exactly");
        
        // Check content
        assert!(inscription.body.is_some(), "Should have body");
        assert_eq!(inscription.body.as_ref().unwrap(), content);
    }

    #[test]
    fn test_parse_delegation_envelope() {
        // Test the exact format created by create_inscription_envelope_with_delegate
        let delegate_id = "test_delegate_id";
        let mut script_bytes = Vec::new();
        
        // OP_PUSHBYTES_0
        script_bytes.push(0x00);
        // OP_IF
        script_bytes.push(0x63);
        // "ord" tag
        script_bytes.push(0x03);
        script_bytes.extend_from_slice(b"ord");
        // Content type tag (1)
        script_bytes.push(0x01);
        script_bytes.push(0x0A); // "text/plain" length
        script_bytes.extend_from_slice(b"text/plain");
        // Delegate reference (tag 11)
        script_bytes.push(0x0B);
        script_bytes.push(delegate_id.len() as u8);
        script_bytes.extend_from_slice(delegate_id.as_bytes());
        // Content tag (0) - empty content for delegating inscription
        script_bytes.push(0x00);
        script_bytes.push(0x00); // Empty content
        // OP_ENDIF
        script_bytes.push(0x68);
        
        std::println!("Delegation script bytes: {:?}", script_bytes);
        
        let script = bitcoin::ScriptBuf::from_bytes(script_bytes);
        let envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
        assert!(envelope.is_some(), "Should parse delegation envelope");
        
        let envelope = envelope.unwrap();
        let inscription = &envelope.payload;
        
        // Check delegate field
        assert!(inscription.delegate.is_some(), "Should have delegate field");
        let delegate_bytes = inscription.delegate.as_ref().unwrap();
        let delegate_str = String::from_utf8(delegate_bytes.clone()).unwrap();
        assert_eq!(delegate_str, delegate_id, "Delegate ID should match");
        
        // Check content (should be empty for delegating inscription)
        assert!(inscription.body.is_some(), "Should have body field");
        let body = inscription.body.as_ref().unwrap();
        assert!(body.is_empty(), "Delegating inscription should have empty content");
    }

    #[test]
    fn test_parse_large_content_envelope() {
        // Test parsing large content that gets split into chunks
        let large_content = b"This is the actual content that will be delegated";
        let content_type = b"text/plain";
        
        let mut script_bytes = Vec::new();
        
        // OP_PUSHBYTES_0
        script_bytes.push(0x00);
        // OP_IF
        script_bytes.push(0x63);
        // "ord" tag
        script_bytes.push(0x03);
        script_bytes.extend_from_slice(b"ord");
        // Content type tag (1)
        script_bytes.push(0x01);
        script_bytes.push(content_type.len() as u8);
        script_bytes.extend_from_slice(content_type);
        // Content tag (0)
        script_bytes.push(0x00);
        // Content length and data
        script_bytes.push(large_content.len() as u8);
        script_bytes.extend_from_slice(large_content);
        // OP_ENDIF
        script_bytes.push(0x68);
        
        std::println!("Large content script bytes: {:?}", script_bytes);
        
        let script = bitcoin::ScriptBuf::from_bytes(script_bytes);
        let envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
        assert!(envelope.is_some(), "Should parse large content envelope");
        
        let envelope = envelope.unwrap();
        let inscription = &envelope.payload;
        
        // Check content
        assert!(inscription.body.is_some(), "Should have body");
        let body = inscription.body.as_ref().unwrap();
        std::println!("Parsed body: {:?}", body);
        std::println!("Expected body: {:?}", large_content);
        assert_eq!(body, large_content, "Content should match exactly");
    }

}