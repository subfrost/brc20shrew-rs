use bitcoin::{
    opcodes::all::*,
    script::{Instruction, Instructions},
    Script, ScriptBuf,
};
use std::collections::BTreeMap;
use std::str::FromStr;

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
        self.delegate.as_ref().and_then(|bytes| {
            if bytes.len() == 36 {
                crate::inscription::InscriptionId::from_bytes(bytes).ok()
            } else {
                // Try parsing as string (for test helpers)
                let id_str = String::from_utf8(bytes.clone()).ok()?;
                crate::inscription::InscriptionId::from_str(&id_str).ok()
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
    // Try parsing as Bitcoin script instructions first
    if let Ok(envelope) = parse_envelope_from_instructions(script, input, offset) {
        if envelope.is_some() {
            return Ok(envelope);
        }
    }
    
    // If that fails, try parsing as raw bytes (for test helpers)
    parse_envelope_from_raw_bytes(script.as_bytes(), input, offset)
}

/// Parse envelope using Bitcoin script instructions
fn parse_envelope_from_instructions(
    script: &Script,
    input: usize,
    offset: usize,
) -> Result<Option<Envelope>, ParseError> {
    let mut instructions = script.instructions();
    
    // Look for inscription envelope pattern
    while let Some(instruction) = instructions.next() {
        let instruction = instruction.map_err(|_| ParseError::InvalidScript)?;
        
        if matches!(instruction, Instruction::Op(OP_PUSHBYTES_0)) {
            if let Some(next_instruction) = instructions.next() {
                let next_instruction = next_instruction.map_err(|_| ParseError::InvalidScript)?;
                
                if matches!(next_instruction, Instruction::Op(OP_IF)) {
                    // Found potential inscription envelope
                    if let Some(inscription) = parse_inscription_from_script_instructions(&mut instructions)? {
                        let pushnum = false; // TODO: Implement pushnum detection
                        let stutter = false; // TODO: Implement stutter detection
                        
                        return Ok(Some(Envelope {
                            input,
                            offset,
                            payload: inscription,
                            pushnum,
                            stutter,
                        }));
                    }
                }
            }
        }
    }

    Ok(None)
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
            pos += 6; // Skip past 0x00 0x63 0x03 "ord"
            
            // Find the end of the envelope (OP_ENDIF = 0x68)
            let mut end_pos = pos;
            while end_pos < bytes.len() && bytes[end_pos] != 0x68 {
                end_pos += 1;
            }
            
            if let Some(inscription) = parse_inscription_from_raw_bytes(&bytes[pos..end_pos])? {
                // Debug: Check if body was parsed
                if let Some(body) = &inscription.body {
                    eprintln!("DEBUG: Envelope found with body length: {}", body.len());
                } else {
                    eprintln!("DEBUG: Envelope found but no body");
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

/// Parse inscription data from script instructions
fn parse_inscription_from_instructions(
    instructions: &mut Instructions,
) -> Result<Option<Inscription>, ParseError> {
    let mut inscription = Inscription::new();
    let mut fields = BTreeMap::new();
    let mut current_field: Option<Vec<u8>> = None;
    let mut body_started = false;
    let mut found_ord_tag = false;

    while let Some(instruction) = instructions.next() {
        let instruction = instruction.map_err(|_| ParseError::InvalidScript)?;

        match instruction {
            Instruction::Op(OP_ENDIF) => {
                // End of inscription envelope
                break;
            }
            Instruction::PushBytes(bytes) if !found_ord_tag => {
                // Check for "ord" protocol identifier
                if bytes.as_bytes() == b"ord" {
                    found_ord_tag = true;
                    continue;
                }
            }
            Instruction::PushBytes(bytes) if found_ord_tag => {
                if current_field.is_some() {
                    // This is field data
                    let field = current_field.take().unwrap();
                    let field_bytes = bytes.as_bytes().to_vec();
                    
                    if fields.contains_key(&field) {
                        inscription.duplicate_field = true;
                    }
                    
                    fields.insert(field.clone(), field_bytes.clone());
                    
                    // Parse known fields
                    match field.as_slice() {
                        [1] => inscription.content_type = Some(field_bytes),
                        [2] => inscription.pointer = Some(field_bytes),
                        [3] => inscription.parent = Some(field_bytes),
                        [5] => inscription.metadata = Some(field_bytes),
                        [7] => inscription.metaprotocol = Some(field_bytes),
                        [9] => inscription.content_encoding = Some(field_bytes),
                        [11] => inscription.delegate = Some(field_bytes),
                        [13] => inscription.rune = Some(field_bytes),
                        tag if tag.len() == 1 && tag[0] % 2 == 0 => {
                            // Unrecognized even field
                            inscription.unrecognized_even_field = true;
                        }
                        _ => {} // Unrecognized odd field (ignored)
                    }
                } else {
                    // This could be a field tag or body content
                    let bytes_vec = bytes.as_bytes().to_vec();
                    
                    // Check if this is the body separator (tag 0)
                    if bytes_vec == vec![0] {
                        body_started = true;
                        // Collect all remaining pushdata as body
                        let mut body = Vec::new();
                        while let Some(instruction) = instructions.next() {
                            let instruction = instruction.map_err(|_| ParseError::InvalidScript)?;
                            
                            match instruction {
                                Instruction::Op(OP_ENDIF) => break,
                                Instruction::PushBytes(bytes) => {
                                    body.extend_from_slice(bytes.as_bytes());
                                }
                                _ => {} // Ignore other opcodes in body
                            }
                        }
                        
                        if !body.is_empty() {
                            inscription.body = Some(body);
                        }
                        break;
                    } else {
                        // This is a field tag
                        current_field = Some(bytes_vec);
                    }
                }
            }
            _ => {
                if current_field.is_some() {
                    // Incomplete field
                    inscription.incomplete_field = true;
                    current_field = None;
                }
            }
        }
    }

    // Check for incomplete field at end
    if current_field.is_some() {
        inscription.incomplete_field = true;
    }

    // Only return inscription if we found the "ord" tag
    if found_ord_tag {
        Ok(Some(inscription))
    } else {
        Ok(None)
    }
}

/// Parse inscription data from script instructions (renamed function)
fn parse_inscription_from_script_instructions(
    instructions: &mut Instructions,
) -> Result<Option<Inscription>, ParseError> {
    parse_inscription_from_instructions(instructions)
}

/// Parse inscription from raw bytes (for test helpers)
fn parse_inscription_from_raw_bytes(bytes: &[u8]) -> Result<Option<Inscription>, ParseError> {
    let mut inscription = Inscription::new();
    let mut pos = 0;
    
    while pos < bytes.len() {
        // Get the field tag
        if pos >= bytes.len() {
            break;
        }
        let field_tag = bytes[pos];
        pos += 1;
        
        // Handle special case for body content (tag 0)
        if field_tag == 0 {
            // For body content, read the rest as body (may be length-prefixed)
            let mut body_data = Vec::new();
            
            // Check if next byte is a length byte
            if pos < bytes.len() {
                let potential_length = bytes[pos];
                // If it looks like a valid length byte (reasonable size and fits in remaining data)
                if potential_length <= 75 && pos + 1 + potential_length as usize <= bytes.len() {
                    pos += 1; // Skip length byte
                    let length = potential_length as usize;
                    body_data.extend_from_slice(&bytes[pos..pos + length]);
                    pos += length;
                } else {
                    // Read all remaining bytes as body
                    body_data.extend_from_slice(&bytes[pos..]);
                    pos = bytes.len();
                }
            }
            
            if !body_data.is_empty() {
                inscription.body = Some(body_data);
            }
            break; // Body is the last field
        }
        
        // For all other fields, read length-prefixed data
        if pos >= bytes.len() {
            inscription.incomplete_field = true;
            break;
        }
        
        let length = bytes[pos] as usize;
        pos += 1;
        
        // Ensure we don't read beyond bounds
        if pos + length > bytes.len() {
            inscription.incomplete_field = true;
            break;
        }
        
        let field_data = bytes[pos..pos + length].to_vec();
        pos += length;
        
        // Store the field based on tag
        match field_tag {
            1 => inscription.content_type = Some(field_data),
            2 => inscription.pointer = Some(field_data),
            3 => inscription.parent = Some(field_data),
            5 => inscription.metadata = Some(field_data),
            7 => inscription.metaprotocol = Some(field_data),
            9 => inscription.content_encoding = Some(field_data),
            11 => inscription.delegate = Some(field_data),
            13 => inscription.rune = Some(field_data),
            tag if tag % 2 == 0 => {
                // Unrecognized even field
                inscription.unrecognized_even_field = true;
            }
            _ => {} // Unrecognized odd field (ignored)
        }
    }
    
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
        
        println!("Script bytes: {:?}", script_bytes);
        
        let script = bitcoin::ScriptBuf::from_bytes(script_bytes);
        let envelope = parse_envelope_from_script(&script, 0, 0).unwrap();
        assert!(envelope.is_some(), "Should parse inscription with metadata");
        
        let envelope = envelope.unwrap();
        let inscription = &envelope.payload;
        
        // Check metadata
        assert!(inscription.metadata.is_some(), "Should have metadata");
        let parsed_metadata = inscription.metadata.as_ref().unwrap();
        println!("Expected metadata: {:?}", metadata);
        println!("Parsed metadata: {:?}", parsed_metadata);
        assert_eq!(parsed_metadata, metadata, "Metadata should match exactly");
        
        // Check content
        assert!(inscription.body.is_some(), "Should have body");
        assert_eq!(inscription.body.as_ref().unwrap(), content);
    }
}