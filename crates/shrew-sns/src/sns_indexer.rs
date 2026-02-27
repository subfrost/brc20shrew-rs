use crate::tables::*;
use shrew_support::inscription::{InscriptionEntry, InscriptionId};
use shrew_ord::tables::{
    SEQUENCE_TO_INSCRIPTION_ENTRY, INSCRIPTION_CONTENT, GLOBAL_SEQUENCE_COUNTER,
};
use bitcoin::Block;
use metashrew_support::index_pointer::KeyValuePointer;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct SnsOperation {
    p: String,
    op: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    ns: Option<String>,
}

pub struct SnsIndexer;

impl SnsIndexer {
    pub fn new() -> Self { Self }

    pub fn index_block(&self, _block: &Block, height: u32) {
        let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
        if seq_bytes.is_empty() { return; }
        let max_seq = u32::from_le_bytes(seq_bytes[..4].try_into().unwrap_or([0; 4]));

        for seq in 1..=max_seq {
            let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq.to_le_bytes().to_vec()).get();
            if entry_bytes.is_empty() { continue; }
            let entry = match InscriptionEntry::from_bytes(&entry_bytes) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.height != height { continue; }
            if entry.number < 0 { continue; }

            // Check content type
            match &entry.content_type {
                Some(ct) if ct.starts_with("text/plain") || ct.starts_with("application/json") => {}
                _ => continue,
            }

            let inscription_id_str = entry.id.to_string();
            let content_bytes = INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).get();
            if content_bytes.is_empty() { continue; }

            let content_str = match std::str::from_utf8(&content_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let op: SnsOperation = match serde_json::from_str(content_str) {
                Ok(o) => o,
                Err(_) => continue,
            };

            if op.p != "sns" { continue; }

            match op.op.as_str() {
                "reg" => {
                    if let Some(name) = op.name {
                        self.process_registration(&entry.id, &name, height);
                    }
                }
                "ns" => {
                    if let Some(ns) = op.ns {
                        self.process_namespace(&entry.id, &ns, height);
                    }
                }
                _ => {}
            }
        }
    }

    fn process_registration(&self, inscription_id: &InscriptionId, name: &str, height: u32) {
        // Lowercase, first whitespace token
        let name = name.to_lowercase();
        let name = name.split_whitespace().next().unwrap_or("");
        if name.is_empty() { return; }

        // Max 2048 UTF-8 bytes
        if name.len() > 2048 { return; }

        // Exactly one dot
        let dot_count = name.chars().filter(|&c| c == '.').count();
        if dot_count != 1 { return; }

        // First-wins
        let existing = SNS_NAME_TO_ID.select(&name.as_bytes().to_vec()).get();
        if !existing.is_empty() { return; }

        let id_bytes = inscription_id.to_bytes();
        SNS_NAME_TO_ID.select(&name.as_bytes().to_vec()).set(Arc::new(id_bytes.clone()));
        SNS_ID_TO_NAME.select(&id_bytes).set(Arc::new(name.as_bytes().to_vec()));
        SNS_HEIGHT_TO_NAMES.select(&height.to_le_bytes().to_vec()).append(Arc::new(id_bytes));
    }

    fn process_namespace(&self, inscription_id: &InscriptionId, ns: &str, _height: u32) {
        let ns = ns.to_lowercase();
        if ns.is_empty() { return; }
        if ns.len() > 2048 { return; }

        // Zero dots for namespace
        if ns.contains('.') { return; }

        // First-wins
        let existing = SNS_NAMESPACE_TO_ID.select(&ns.as_bytes().to_vec()).get();
        if !existing.is_empty() { return; }

        let id_bytes = inscription_id.to_bytes();
        SNS_NAMESPACE_TO_ID.select(&ns.as_bytes().to_vec()).set(Arc::new(id_bytes));
    }
}
