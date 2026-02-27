use bitcoin::Script;
use shrew_support::InscriptionId;

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
    pub fn new() -> Self { Self::default() }

    pub fn content_type(&self) -> Option<String> {
        self.content_type.as_ref().and_then(|bytes| String::from_utf8(bytes.clone()).ok())
    }

    pub fn metaprotocol(&self) -> Option<String> {
        self.metaprotocol.as_ref().and_then(|bytes| String::from_utf8(bytes.clone()).ok())
    }

    pub fn content_length(&self) -> Option<usize> {
        self.body.as_ref().map(|body| body.len())
    }

    pub fn delegate_id(&self) -> Option<InscriptionId> {
        self.delegate.as_ref().and_then(|bytes| {
            if bytes.len() == 36 {
                InscriptionId::from_bytes(bytes).ok()
            } else {
                let id_str = String::from_utf8(bytes.clone()).ok()?;
                id_str.parse().ok()
            }
        })
    }

    pub fn parent_id(&self) -> Option<InscriptionId> {
        self.parent.as_ref().and_then(|bytes| {
            if bytes.len() == 36 {
                InscriptionId::from_bytes(bytes).ok()
            } else {
                let id_str = String::from_utf8(bytes.clone()).ok()?;
                id_str.parse().ok()
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
        self.duplicate_field || self.incomplete_field || self.unrecognized_even_field || self.body.is_none()
    }
}

/// Parse inscriptions from a transaction's witness data
pub fn parse_inscriptions_from_transaction(
    tx: &bitcoin::Transaction,
) -> Result<Vec<Envelope>, ParseError> {
    let mut envelopes = Vec::new();
    for (input_index, input) in tx.input.iter().enumerate() {
        for (witness_index, witness_element) in input.witness.iter().enumerate() {
            let script = bitcoin::ScriptBuf::from_bytes(witness_element.to_vec());
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
    parse_envelope_from_raw_bytes(script.as_bytes(), input, offset)
}

fn parse_envelope_from_raw_bytes(
    bytes: &[u8],
    input: usize,
    offset: usize,
) -> Result<Option<Envelope>, ParseError> {
    let mut pos = 0;
    // Look for envelope pattern: 0x00 0x63 0x03 "ord"
    while pos + 5 < bytes.len() {
        if bytes[pos] == 0x00 && bytes[pos + 1] == 0x63
            && bytes[pos + 2] == 0x03 && &bytes[pos + 3..pos + 6] == b"ord"
        {
            pos += 6;
            let end_pos = bytes.len() - 1; // Exclude OP_ENDIF
            if let Some(inscription) = parse_inscription_fields(&bytes[pos..end_pos])? {
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

fn parse_inscription_fields(field_data: &[u8]) -> Result<Option<Inscription>, ParseError> {
    let mut inscription = Inscription::new();
    let mut pos = 0;

    while pos < field_data.len() {
        let push_length = field_data[pos] as usize;
        pos += 1;
        if pos + push_length > field_data.len() { break; }
        let push_data = &field_data[pos..pos + push_length];
        pos += push_length;

        if push_length == 1 {
            let tag = push_data[0];
            if pos >= field_data.len() { break; }
            let value_length = field_data[pos] as usize;
            pos += 1;
            if pos + value_length > field_data.len() { break; }
            let value = &field_data[pos..pos + value_length];
            pos += value_length;

            match tag {
                1 => inscription.content_type = Some(value.to_vec()),
                2 => inscription.pointer = Some(value.to_vec()),
                3 => inscription.parent = Some(value.to_vec()),
                5 => inscription.metadata = Some(value.to_vec()),
                7 => inscription.metaprotocol = Some(value.to_vec()),
                9 => inscription.content_encoding = Some(value.to_vec()),
                11 => inscription.delegate = Some(value.to_vec()),
                13 => inscription.rune = Some(value.to_vec()),
                tag if tag % 2 == 0 => inscription.unrecognized_even_field = true,
                _ => {}
            }
        } else if push_length == 0 {
            // Body tag - read all subsequent push operations as body chunks
            let mut body_content = Vec::new();
            while pos < field_data.len() {
                let opcode = field_data[pos];
                pos += 1;
                let chunk_len = if opcode <= 75 {
                    opcode as usize
                } else if opcode == 76 {
                    if pos >= field_data.len() { break; }
                    let len = field_data[pos] as usize;
                    pos += 1;
                    len
                } else if opcode == 77 {
                    if pos + 1 >= field_data.len() { break; }
                    let len = u16::from_le_bytes([field_data[pos], field_data[pos + 1]]) as usize;
                    pos += 2;
                    len
                } else if opcode == 78 {
                    if pos + 3 >= field_data.len() { break; }
                    let len = u32::from_le_bytes([
                        field_data[pos], field_data[pos + 1],
                        field_data[pos + 2], field_data[pos + 3]
                    ]) as usize;
                    pos += 4;
                    len
                } else {
                    break;
                };
                if pos + chunk_len > field_data.len() {
                    body_content.extend_from_slice(&field_data[pos..]);
                    break;
                }
                body_content.extend_from_slice(&field_data[pos..pos + chunk_len]);
                pos += chunk_len;
            }
            inscription.body = Some(body_content);
            break;
        }
    }
    Ok(Some(inscription))
}

/// Parse inscription from raw bytes (public for test helpers)
pub fn parse_inscription_from_raw_bytes(bytes: &[u8]) -> Result<Option<Inscription>, ParseError> {
    let mut pos = 0;
    while pos + 5 < bytes.len() {
        if bytes[pos] == 0x00 && bytes[pos + 1] == 0x63
            && bytes[pos + 2] == 0x03 && &bytes[pos + 3..pos + 6] == b"ord"
        {
            pos += 6;
            break;
        }
        pos += 1;
    }
    if pos + 5 >= bytes.len() { return Ok(None); }
    let mut end_pos = pos;
    while end_pos < bytes.len() && bytes[end_pos] != 0x68 { end_pos += 1; }
    if end_pos >= bytes.len() { return Ok(None); }
    parse_inscription_fields(&bytes[pos..end_pos])
}

/// Errors during envelope parsing
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
