use bitcoin::{
    opcodes::{all::*, OP_FALSE},
    script::{Instruction, Instructions},
    Script, ScriptBuf,
};
use std::collections::BTreeMap;

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
                None
            }
        })
    }

    pub fn parent_id(&self) -> Option<crate::inscription::InscriptionId> {
        self.parent.as_ref().and_then(|bytes| {
            if bytes.len() == 36 {
                crate::inscription::InscriptionId::from_bytes(bytes).ok()
            } else {
                None
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
        if let Some(witness) = input.witness.as_ref() {
            for (witness_index, witness_element) in witness.iter().enumerate() {
                if let Ok(script) = ScriptBuf::from_bytes(witness_element.clone()) {
                    if let Some(envelope) = parse_envelope_from_script(&script, input_index, witness_index)? {
                        envelopes.push(envelope);
                    }
                }
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
    let mut instructions = script.instructions();
    
    // Look for inscription envelope pattern
    while let Some(instruction) = instructions.next() {
        let instruction = instruction.map_err(|_| ParseError::InvalidScript)?;
        
        if matches!(instruction, Instruction::Op(OP_FALSE)) {
            if let Some(next_instruction) = instructions.next() {
                let next_instruction = next_instruction.map_err(|_| ParseError::InvalidScript)?;
                
                if matches!(next_instruction, Instruction::Op(OP_IF)) {
                    // Found potential inscription envelope
                    if let Some(inscription) = parse_inscription_from_instructions(&mut instructions)? {
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

/// Parse inscription data from script instructions
fn parse_inscription_from_instructions(
    instructions: &mut Instructions,
) -> Result<Option<Inscription>, ParseError> {
    let mut inscription = Inscription::new();
    let mut fields = BTreeMap::new();
    let mut current_field: Option<Vec<u8>> = None;
    let mut body_started = false;

    while let Some(instruction) = instructions.next() {
        let instruction = instruction.map_err(|_| ParseError::InvalidScript)?;

        match instruction {
            Instruction::Op(OP_ENDIF) => {
                // End of inscription envelope
                break;
            }
            Instruction::Op(OP_0) if !body_started => {
                // Start of body
                body_started = true;
                let mut body = Vec::new();
                
                // Collect all remaining pushdata as body
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
            }
            Instruction::PushBytes(bytes) if !body_started => {
                if let Some(field) = current_field.take() {
                    // This is field data
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
                    // This is a field tag
                    current_field = Some(bytes.as_bytes().to_vec());
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
            .push_opcode(OP_FALSE)
            .push_opcode(OP_IF)
            .push_slice([1]) // content-type tag
            .push_slice(b"text/plain")
            .push_opcode(OP_0) // body separator
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
            .push_opcode(OP_FALSE)
            .push_opcode(OP_IF)
            .push_slice([1]) // content-type tag
            .push_slice(b"text/plain")
            .push_slice([1]) // duplicate content-type tag
            .push_slice(b"text/html")
            .push_opcode(OP_0) // body separator
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
}