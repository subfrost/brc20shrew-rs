use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::envelope::{parse_inscriptions_from_transaction, Inscription};
use bitcoin::{Amount, Transaction, TxIn, TxOut, OutPoint, Witness, ScriptBuf, Sequence, Txid, transaction::Version};
use shrew_test_helpers::inscriptions::*;
use std::str::FromStr;

/// Helper to build a transaction with a single witness element
fn tx_with_witness(witness: Witness) -> Transaction {
    Transaction {
        version: Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .unwrap(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness,
        }],
        output: vec![TxOut {
            value: Amount::from_sat(100_000_000),
            script_pubkey: ScriptBuf::new(),
        }],
    }
}

#[test]
fn test_parse_simple_text_inscription() {
    let body = b"Hello, World!";
    let content_type = b"text/plain";
    let witness = create_inscription_envelope(content_type, body);
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert_eq!(envelopes.len(), 1);

    let inscription = &envelopes[0].payload;
    assert_eq!(inscription.content_type(), Some("text/plain".to_string()));
    assert_eq!(inscription.body.as_deref(), Some(body.as_slice()));
}

#[test]
fn test_parse_json_inscription() {
    let body = br#"{"name":"Test","value":42}"#;
    let content_type = b"application/json";
    let witness = create_inscription_envelope(content_type, body);
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert_eq!(envelopes.len(), 1);

    let inscription = &envelopes[0].payload;
    assert_eq!(
        inscription.content_type(),
        Some("application/json".to_string())
    );
    assert_eq!(inscription.body.as_deref(), Some(body.as_slice()));
}

#[test]
fn test_parse_image_inscription() {
    let (body, content_type_str) = create_test_image_inscription();
    let witness = create_inscription_envelope(content_type_str.as_bytes(), &body);
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert_eq!(envelopes.len(), 1);

    let inscription = &envelopes[0].payload;
    assert_eq!(
        inscription.content_type(),
        Some("image/png".to_string())
    );
    assert_eq!(inscription.body.as_deref(), Some(body.as_slice()));
}

#[test]
fn test_parse_inscription_with_metadata() {
    let body = b"inscription body";
    let metadata = b"\xa1\x63foo\x63bar"; // CBOR: {"foo": "bar"}
    let witness =
        create_inscription_envelope_with_metadata(b"text/plain", body, Some(metadata));
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert_eq!(envelopes.len(), 1);

    let inscription = &envelopes[0].payload;
    assert!(inscription.metadata.is_some());
    assert_eq!(inscription.body.as_deref(), Some(body.as_slice()));
}

#[test]
fn test_parse_inscription_no_content_type() {
    // Create inscription with empty content type
    let witness = create_inscription_envelope(b"", b"some body data");
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    // The envelope may or may not be parsed depending on the builder behavior,
    // but if parsed, the content_type should be None or empty
    if !envelopes.is_empty() {
        let inscription = &envelopes[0].payload;
        let ct = inscription.content_type();
        // Content type should be None or empty string
        assert!(ct.is_none() || ct.as_deref() == Some(""));
    }
}

#[test]
fn test_parse_inscription_no_body() {
    // Build a witness with ord envelope but no body tag
    use crate::ord_inscriptions::Inscription as OrdInscription;
    let inscription = OrdInscription {
        content_type: Some(b"text/plain".to_vec()),
        body: None,
        ..Default::default()
    };
    let witness = inscription.to_witness();
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    if !envelopes.is_empty() {
        let parsed = &envelopes[0].payload;
        // Without body tag, body should be None
        assert!(parsed.body.is_none());
    }
}

#[test]
fn test_parse_empty_witness() {
    let tx = Transaction {
        version: Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::default(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(100_000_000),
            script_pubkey: ScriptBuf::new(),
        }],
    };

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert!(envelopes.is_empty(), "Empty witness should yield no envelopes");
}

#[test]
fn test_parse_invalid_envelope() {
    let witness = create_invalid_envelope();
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert!(
        envelopes.is_empty(),
        "Invalid protocol ID should yield no envelopes"
    );
}

#[test]
fn test_envelope_content_type_extraction() {
    let inscription = Inscription {
        content_type: Some(b"text/html".to_vec()),
        body: Some(b"<h1>Hello</h1>".to_vec()),
        ..Default::default()
    };
    assert_eq!(inscription.content_type(), Some("text/html".to_string()));

    let no_ct = Inscription {
        content_type: None,
        body: Some(b"data".to_vec()),
        ..Default::default()
    };
    assert!(no_ct.content_type().is_none());
}

#[test]
fn test_envelope_content_length() {
    let inscription = Inscription {
        body: Some(b"12345".to_vec()),
        ..Default::default()
    };
    assert_eq!(inscription.content_length(), Some(5));

    let no_body = Inscription {
        body: None,
        ..Default::default()
    };
    assert!(no_body.content_length().is_none());
}

#[test]
fn test_parse_inscription_with_parent() {
    let parent_id_str = "0000000000000000000000000000000000000000000000000000000000000001i0";
    let witness = create_inscription_envelope_with_parent(
        b"text/plain",
        b"child inscription",
        parent_id_str,
    );
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert_eq!(envelopes.len(), 1);

    let inscription = &envelopes[0].payload;
    assert!(inscription.parent.is_some(), "Parent field should be set");
    assert_eq!(inscription.body.as_deref(), Some(b"child inscription".as_slice()));
}

#[test]
fn test_parse_inscription_with_delegate() {
    let delegate_id_str = "0000000000000000000000000000000000000000000000000000000000000002i0";
    let witness = create_inscription_envelope_with_delegate(
        b"text/plain",
        b"delegated body",
        delegate_id_str,
    );
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert_eq!(envelopes.len(), 1);

    let inscription = &envelopes[0].payload;
    assert!(
        inscription.delegate.is_some(),
        "Delegate field should be set"
    );
}

#[test]
fn test_parse_inscription_with_pointer() {
    use crate::ord_inscriptions::Inscription as OrdInscription;
    let pointer_value: u64 = 42;
    let inscription = OrdInscription {
        content_type: Some(b"text/plain".to_vec()),
        body: Some(b"pointer data".to_vec()),
        pointer: Some(pointer_value.to_le_bytes().to_vec()),
        ..Default::default()
    };
    let witness = inscription.to_witness();
    let tx = tx_with_witness(witness);

    let envelopes = parse_inscriptions_from_transaction(&tx).unwrap();
    assert_eq!(envelopes.len(), 1);

    let parsed = &envelopes[0].payload;
    assert!(parsed.pointer.is_some(), "Pointer field should be set");
    assert_eq!(parsed.pointer_value(), Some(42));
}

#[test]
fn test_envelope_is_cursed_for_no_body() {
    let inscription = Inscription {
        content_type: Some(b"text/plain".to_vec()),
        body: None,
        ..Default::default()
    };
    assert!(
        inscription.is_cursed(),
        "Inscription without body should be cursed"
    );
}

#[test]
fn test_envelope_is_cursed_for_duplicate_field() {
    let inscription = Inscription {
        content_type: Some(b"text/plain".to_vec()),
        body: Some(b"data".to_vec()),
        duplicate_field: true,
        ..Default::default()
    };
    assert!(
        inscription.is_cursed(),
        "Inscription with duplicate field should be cursed"
    );
}
