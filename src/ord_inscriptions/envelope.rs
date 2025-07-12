//! Inscription envelope types and constants
//! 
//! Ported from ord/src/inscriptions/envelope.rs

use super::Inscription;
use bitcoin::{Transaction, Witness};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_ID: [u8; 3] = *b"ord";
pub const BODY_TAG: [u8; 0] = [];

pub type RawEnvelope = Envelope<Vec<Vec<u8>>>;
pub type ParsedEnvelope = Envelope<Inscription>;

#[derive(Default, PartialEq, Clone, Serialize, Deserialize, Debug, Eq)]
pub struct Envelope<T> {
    pub input: u32,
    pub offset: u32,
    pub payload: T,
    pub pushnum: bool,
    pub stutter: bool,
}

impl From<RawEnvelope> for ParsedEnvelope {
    fn from(envelope: RawEnvelope) -> Self {
        let body = envelope
            .payload
            .iter()
            .enumerate()
            .position(|(i, push)| i % 2 == 0 && push.is_empty());

        let mut fields: BTreeMap<&[u8], Vec<&[u8]>> = BTreeMap::new();

        let mut incomplete_field = false;

        for item in envelope.payload[..body.unwrap_or(envelope.payload.len())].chunks(2) {
            match item {
                [key, value] => fields.entry(key).or_default().push(value),
                _ => incomplete_field = true,
            }
        }

        let duplicate_field = fields.iter().any(|(_key, values)| values.len() > 1);

        let content_encoding = super::Tag::ContentEncoding.take(&mut fields);
        let content_type = super::Tag::ContentType.take(&mut fields);
        let delegate = super::Tag::Delegate.take(&mut fields);
        let metadata = super::Tag::Metadata.take(&mut fields);
        let metaprotocol = super::Tag::Metaprotocol.take(&mut fields);
        let parents = super::Tag::Parent.take_array(&mut fields);
        let pointer = super::Tag::Pointer.take(&mut fields);
        let properties = super::Tag::Properties.take(&mut fields);
        let rune = super::Tag::Rune.take(&mut fields);

        let unrecognized_even_field = fields
            .keys()
            .any(|tag| tag.first().map(|lsb| lsb % 2 == 0).unwrap_or_default());

        Self {
            payload: Inscription {
                body: body.map(|i| {
                    envelope.payload[i + 1..]
                        .iter()
                        .flatten()
                        .cloned()
                        .collect()
                }),
                content_encoding,
                content_type,
                delegate,
                duplicate_field,
                incomplete_field,
                metadata,
                metaprotocol,
                parents,
                pointer,
                properties,
                rune,
                unrecognized_even_field,
            },
            input: envelope.input,
            offset: envelope.offset,
            pushnum: envelope.pushnum,
            stutter: envelope.stutter,
        }
    }
}

impl ParsedEnvelope {
    pub fn from_transaction(transaction: &Transaction) -> Vec<Self> {
        RawEnvelope::from_transaction(transaction)
            .into_iter()
            .map(|envelope| envelope.into())
            .collect()
    }
}

impl RawEnvelope {
    pub fn from_transaction(_transaction: &Transaction) -> Vec<Self> {
        // Simplified implementation - we mainly need this for the constants and types
        // The full parsing implementation would go here if needed
        Vec::new()
    }
}