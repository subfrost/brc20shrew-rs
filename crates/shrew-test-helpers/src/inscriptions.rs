use bitcoin::Witness;
use shrew_ord::ord_inscriptions::Inscription;

/// Create a simple inscription envelope with content type and body
pub fn create_inscription_envelope(content_type: &[u8], body: &[u8]) -> Witness {
    let inscription = Inscription {
        content_type: if content_type.is_empty() { None } else { Some(content_type.to_vec()) },
        body: Some(body.to_vec()),
        ..Default::default()
    };
    inscription.to_witness()
}

/// Create inscription envelope with metadata
pub fn create_inscription_envelope_with_metadata(
    content_type: &[u8],
    body: &[u8],
    metadata: Option<&[u8]>,
) -> Witness {
    let inscription = Inscription {
        content_type: if content_type.is_empty() { None } else { Some(content_type.to_vec()) },
        metadata: metadata.map(|m| m.to_vec()),
        body: Some(body.to_vec()),
        ..Default::default()
    };
    inscription.to_witness()
}

/// Create inscription envelope with parent reference
pub fn create_inscription_envelope_with_parent(
    content_type: &[u8],
    body: &[u8],
    parent_id: &str,
) -> Witness {
    let inscription = Inscription {
        content_type: if content_type.is_empty() { None } else { Some(content_type.to_vec()) },
        parents: vec![parent_id.as_bytes().to_vec()],
        body: Some(body.to_vec()),
        ..Default::default()
    };
    inscription.to_witness()
}

/// Create inscription envelope with delegate reference
pub fn create_inscription_envelope_with_delegate(
    content_type: &[u8],
    body: &[u8],
    delegate_id: &str,
) -> Witness {
    let inscription = Inscription {
        content_type: if content_type.is_empty() { None } else { Some(content_type.to_vec()) },
        delegate: Some(delegate_id.as_bytes().to_vec()),
        body: Some(body.to_vec()),
        ..Default::default()
    };
    inscription.to_witness()
}

/// Create an invalid envelope (wrong protocol identifier)
pub fn create_invalid_envelope() -> Witness {
    let mut script_bytes = Vec::new();
    script_bytes.push(0x00); // OP_PUSHBYTES_0
    script_bytes.push(0x63); // OP_IF
    script_bytes.push(0x07); // push 7 bytes
    script_bytes.extend_from_slice(b"invalid");
    script_bytes.push(0x68); // OP_ENDIF
    Witness::from_slice(&[script_bytes, Vec::new()])
}

/// Create multiple envelopes in same input
pub fn create_multiple_envelopes_same_input() -> Witness {
    let mut script_bytes = Vec::new();
    // First envelope
    script_bytes.push(0x00);
    script_bytes.push(0x63);
    script_bytes.push(0x03);
    script_bytes.extend_from_slice(b"ord");
    script_bytes.push(0x00);
    script_bytes.push(0x05);
    script_bytes.extend_from_slice(b"first");
    script_bytes.push(0x68);
    // Second envelope
    script_bytes.push(0x00);
    script_bytes.push(0x63);
    script_bytes.push(0x03);
    script_bytes.extend_from_slice(b"ord");
    script_bytes.push(0x00);
    script_bytes.push(0x06);
    script_bytes.extend_from_slice(b"second");
    script_bytes.push(0x68);
    Witness::from_slice(&[script_bytes, Vec::new()])
}

/// Create test inscription content (text)
pub fn create_test_inscription_content() -> (&'static [u8], &'static str) {
    (b"Hello, Bitcoin Inscriptions!", "text/plain")
}

/// Create test JSON inscription content
pub fn create_test_json_inscription() -> (&'static [u8], &'static str) {
    (br#"{"name": "Test NFT", "description": "A test inscription"}"#, "application/json")
}

/// Create test image inscription content (mock PNG)
pub fn create_test_image_inscription() -> (Vec<u8>, &'static str) {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00,
    ];
    (png_header, "image/png")
}
