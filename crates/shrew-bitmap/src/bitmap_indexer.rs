use crate::tables::*;
use shrew_support::inscription::InscriptionEntry;
use shrew_ord::tables::{
    SEQUENCE_TO_INSCRIPTION_ENTRY, INSCRIPTION_CONTENT, GLOBAL_SEQUENCE_COUNTER,
};
use bitcoin::Block;
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::Arc;

pub struct BitmapIndexer;

impl BitmapIndexer {
    pub fn new() -> Self { Self }

    pub fn index_block(&self, _block: &Block, height: u32) {
        // Scan all blessed inscriptions created at this height.
        // We iterate over new inscriptions by checking recent sequence numbers.
        // A bitmap inscription has text/plain content matching ^[0-9]+\.bitmap$
        // The number must be <= block_height, no leading zeros, and first-wins.

        let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
        if seq_bytes.is_empty() { return; }
        let max_seq = u32::from_le_bytes(seq_bytes[..4].try_into().unwrap_or([0; 4]));

        // We scan the last batch of inscriptions (those created at this height)
        // In practice, we'd track the starting sequence for this block.
        // For now, scan all sequences and check height.
        for seq in 1..=max_seq {
            let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq.to_le_bytes().to_vec()).get();
            if entry_bytes.is_empty() { continue; }
            let entry = match InscriptionEntry::from_bytes(&entry_bytes) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Only process inscriptions at this height
            if entry.height != height { continue; }

            // Only blessed inscriptions
            if entry.number < 0 { continue; }

            // Check content type is text/plain
            match &entry.content_type {
                Some(ct) if ct.starts_with("text/plain") => {}
                _ => continue,
            }

            // Get content
            let inscription_id_str = entry.id.to_string();
            let content_bytes = INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).get();
            if content_bytes.is_empty() { continue; }

            let content = match std::str::from_utf8(&content_bytes) {
                Ok(s) => s.trim(),
                Err(_) => continue,
            };

            // Match pattern: ^[0-9]+\.bitmap$
            if !content.ends_with(".bitmap") { continue; }
            let number_str = &content[..content.len() - 7]; // strip ".bitmap"

            // No leading zeros (except "0" itself)
            if number_str.len() > 1 && number_str.starts_with('0') { continue; }

            // Parse as integer
            let bitmap_number: u64 = match number_str.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Must be <= current block height
            if bitmap_number > height as u64 { continue; }

            // First-wins: check if already registered
            let existing = BITMAP_NUMBER_TO_ID.select(&bitmap_number.to_le_bytes().to_vec()).get();
            if !existing.is_empty() { continue; }

            // Register
            let id_bytes = entry.id.to_bytes();
            BITMAP_NUMBER_TO_ID.select(&bitmap_number.to_le_bytes().to_vec()).set(Arc::new(id_bytes.clone()));
            BITMAP_ID_TO_NUMBER.select(&id_bytes).set(Arc::new(bitmap_number.to_le_bytes().to_vec()));
            BITMAP_HEIGHT_TO_ENTRIES.select(&height.to_le_bytes().to_vec()).append(Arc::new(id_bytes));
        }
    }
}
