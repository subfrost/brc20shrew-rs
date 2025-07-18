#[allow(unused_imports)]
use {
    metashrew_core::metashrew_println::{println},
    std::fmt::Write
};

use crate::{
    envelope::{parse_inscriptions_from_transaction, Envelope},
    inscription::{Charm, InscriptionEntry, InscriptionId, Rarity, SatPoint},
    tables::*,
    brc20::Brc20Indexer,
    programmable_brc20::ProgrammableBrc20Indexer,
    utils::get_address_from_txout,
};
use bitcoin::{Block, OutPoint, Transaction, Txid, Network};
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use std::collections::HashMap;
use std::sync::Arc;

/// Main indexer for processing Bitcoin blocks and extracting inscriptions
pub struct InscriptionIndexer {
    pub height: u32,
    pub block_hash: bitcoin::BlockHash,
    pub block_time: u32,
    pub network: Network,
    pub sequence_counter: u32,
    pub blessed_counter: i32,
    pub cursed_counter: i32,
    pub jubilee_height: u32,
}

impl InscriptionIndexer {
    pub fn new() -> Self {
        Self {
            height: 0,
            block_hash: bitcoin::BlockHash::all_zeros(),
            block_time: 0,
            network: Network::Bitcoin,
            sequence_counter: 0,
            blessed_counter: 0,
            cursed_counter: -1,
            jubilee_height: 824544, // Bitcoin block height where cursed inscriptions become blessed
        }
    }

    /// Initialize indexer state from database
    pub fn load_state(&mut self) -> Result<(), IndexError> {
        // Load counters from database
        let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
        if !seq_bytes.is_empty() {
            self.sequence_counter = u32::from_le_bytes(
                seq_bytes[..4].try_into().map_err(|_| IndexError::InvalidData)?,
            );
        }

        let blessed_bytes = BLESSED_INSCRIPTION_COUNTER.get();
        if !blessed_bytes.is_empty() {
            self.blessed_counter = i32::from_le_bytes(
                blessed_bytes[..4].try_into().map_err(|_| IndexError::InvalidData)?,
            );
        }

        let cursed_bytes = CURSED_INSCRIPTION_COUNTER.get();
        if !cursed_bytes.is_empty() {
            self.cursed_counter = i32::from_le_bytes(
                cursed_bytes[..4].try_into().map_err(|_| IndexError::InvalidData)?,
            );
        }

        Ok(())
    }

    /// Save indexer state to database
    pub fn save_state(&self) -> Result<(), IndexError> {
        GLOBAL_SEQUENCE_COUNTER.clone().set(Arc::new(self.sequence_counter.to_le_bytes().to_vec()));
        BLESSED_INSCRIPTION_COUNTER.clone().set(Arc::new(self.blessed_counter.to_le_bytes().to_vec()));
        CURSED_INSCRIPTION_COUNTER.clone().set(Arc::new(self.cursed_counter.to_le_bytes().to_vec()));
        Ok(())
    }

    /// Process a Bitcoin block and index all inscriptions
    pub fn index_block(&mut self, block: &Block, height: u32) -> Result<BlockIndexResult, IndexError> {
        self.height = height;
        self.block_hash = block.block_hash();
        self.block_time = block.header.time;

        // Store block metadata
        HEIGHT_TO_BLOCK_HASH.select(&height.to_le_bytes().to_vec()).set(Arc::new(self.block_hash.as_byte_array().to_vec()));
        BLOCK_HASH_TO_HEIGHT.select(&self.block_hash.as_byte_array().to_vec()).set(Arc::new(height.to_le_bytes().to_vec()));

        let mut result = BlockIndexResult::new(height, self.block_hash);
        let mut sat_ranges = SatRanges::new();

        // Calculate sat ranges for all transaction inputs and outputs
        for (tx_index, tx) in block.txdata.iter().enumerate() {
            sat_ranges.process_transaction(tx, tx_index == 0)?;
        }

        // Process transactions for inscriptions
        for (tx_index, tx) in block.txdata.iter().enumerate() {
            let tx_result = self.index_transaction(tx, tx_index, &sat_ranges)?;
            result.merge(tx_result);
        }

        // Update height-based indexes
        if !result.inscriptions.is_empty() {
            let inscription_ids: Vec<_> = result.inscriptions.iter().map(|e| e.id.to_bytes()).collect();
            // Store each inscription ID separately since set_list doesn't exist
            for (i, inscription_id) in inscription_ids.iter().enumerate() {
                let key = format!("{}:{}", height, i);
                HEIGHT_TO_INSCRIPTIONS.select(&key.as_bytes().to_vec()).set(Arc::new(inscription_id.clone()));
            }
        }

        self.save_state()?;
        Ok(result)
    }

    /// Process a single transaction for inscriptions
    fn index_transaction(
        &mut self,
        tx: &Transaction,
        tx_index: usize,
        sat_ranges: &SatRanges,
    ) -> Result<TransactionIndexResult, IndexError> {
        let mut result = TransactionIndexResult::new(tx.txid());

        // Process BRC20 transfers first when inputs are spent
        self.process_brc20_transfers(tx)?;

        // Parse new inscription envelopes from transaction
        let envelopes = parse_inscriptions_from_transaction(tx)
            .map_err(|_| IndexError::ParseError)?;

        if envelopes.is_empty() {
            return Ok(result);
        }

        // Process each new inscription envelope
        for envelope in envelopes {
            let inscription_result = self.process_inscription_envelope(
                tx,
                tx_index,
                &envelope,
                sat_ranges,
            )?;
            result.merge(inscription_result);
        }

        Ok(result)
    }

    /// Process a single inscription envelope
    fn process_inscription_envelope(
        &mut self,
        tx: &Transaction,
        tx_index: usize,
        envelope: &Envelope,
        sat_ranges: &SatRanges,
    ) -> Result<InscriptionIndexResult, IndexError> {
        let inscription_id = InscriptionId::new(tx.txid(), envelope.input as u32);
        
        // Check if inscription already exists
        if !INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.to_bytes()).get().is_empty() {
            return Err(IndexError::DuplicateInscription);
        }

        // Determine if inscription is cursed
        let is_cursed = envelope.payload.is_cursed() || self.is_cursed_by_context(envelope, tx_index);

        // Assign inscription number
        let number = if is_cursed && self.height < self.jubilee_height {
            self.cursed_counter -= 1;
            self.cursed_counter
        } else {
            self.blessed_counter += 1;
            self.blessed_counter
        };

        // Get sequence number
        self.sequence_counter += 1;
        let sequence = self.sequence_counter;

        // Calculate satpoint
        let satpoint = self.calculate_satpoint(tx, envelope, sat_ranges)?;

        // Create inscription entry
        let mut entry = InscriptionEntry::new(
            inscription_id.clone(),
            number,
            sequence,
            satpoint.clone(),
            self.height,
            self.calculate_fee(tx),
            self.block_time,
        );

        // Set inscription properties from envelope
        if let Some(content_type) = envelope.payload.content_type() {
            entry.content_type = Some(content_type);
        }

        if let Some(content_length) = envelope.payload.content_length() {
            entry.content_length = Some(content_length as u64);
        }

        if let Some(metaprotocol) = envelope.payload.metaprotocol() {
            entry.metaprotocol = Some(metaprotocol);
        }

        if let Some(parent_id) = envelope.payload.parent_id() {
            entry.parent = Some(parent_id);
        }

        if let Some(delegate_id) = envelope.payload.delegate_id() {
            entry.delegate = Some(delegate_id);
        }

        if let Some(pointer) = envelope.payload.pointer_value() {
            entry.pointer = Some(pointer);
        }

        // Calculate sat number if available
        if let Some(sat) = self.calculate_sat_number(&satpoint, sat_ranges) {
            entry.sat = Some(sat);
            
            // Set rarity-based charms
            let rarity = Rarity::from_sat(sat);
            match rarity {
                Rarity::Uncommon => entry.set_charm(Charm::Uncommon),
                Rarity::Rare => entry.set_charm(Charm::Rare),
                Rarity::Epic => entry.set_charm(Charm::Epic),
                Rarity::Legendary => entry.set_charm(Charm::Legendary),
                _ => {}
            }
        }

        // Set other charms
        if is_cursed {
            entry.set_charm(Charm::Cursed);
        }

        if envelope.payload.body.is_none() {
            entry.set_charm(Charm::Unbound);
        }

        // Store inscription in database
        self.store_inscription(&entry, envelope)?;

        // Process BRC20 operations for the new inscription
        self.process_brc20_inscription(tx, &entry, envelope)?;

        // Process programmable BRC-20 operations
        let mut prog_indexer = ProgrammableBrc20Indexer::new();
        prog_indexer.index_programmable_inscription(&entry, &envelope.payload);

        Ok(InscriptionIndexResult {
            inscription: entry,
            envelope: envelope.clone(),
        })
    }

    /// Store inscription and related data in database
    fn store_inscription(&self, entry: &InscriptionEntry, envelope: &Envelope) -> Result<(), IndexError> {
        let id_bytes = entry.id.to_bytes();
        let sequence_bytes = entry.sequence.to_le_bytes().to_vec();
        let entry_bytes = entry.to_bytes();

        // Core mappings
        INSCRIPTION_ID_TO_SEQUENCE.select(&id_bytes).set(Arc::new(sequence_bytes.clone()));
        SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).set(Arc::new(entry_bytes));
        INSCRIPTION_NUMBER_TO_SEQUENCE.select(&entry.number.to_le_bytes().to_vec()).set(Arc::new(sequence_bytes.clone()));

        // Location tracking
        SEQUENCE_TO_SATPOINT.select(&sequence_bytes).set(Arc::new(entry.satpoint.to_bytes()));
        
        if let Some(sat) = entry.sat {
            SAT_TO_SEQUENCE.select(&sat.to_le_bytes().to_vec()).set(Arc::new(sequence_bytes.clone()));
            INSCRIPTION_TO_SAT.select(&sequence_bytes).set(Arc::new(sat.to_le_bytes().to_vec()));
        }

        // Outpoint tracking
        let outpoint_bytes = entry.satpoint.outpoint.txid.as_byte_array()
            .iter()
            .chain(entry.satpoint.outpoint.vout.to_le_bytes().iter())
            .copied()
            .collect::<Vec<u8>>();
        OUTPOINT_TO_INSCRIPTIONS.select(&outpoint_bytes).append(Arc::new(sequence_bytes.clone()));

        // Parent-child relationships
        if let Some(parent_id) = &entry.parent {
            let parent_id_bytes = parent_id.to_bytes();
            let parent_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id_bytes).get();
            if !parent_seq_bytes.is_empty() {
                SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).append(Arc::new(sequence_bytes.clone()));
                SEQUENCE_TO_PARENTS.select(&sequence_bytes).append(Arc::new(parent_seq_bytes.to_vec()));
            }
        }

        // Content type indexing
        if let Some(content_type) = &entry.content_type {
            CONTENT_TYPE_TO_INSCRIPTIONS.select(&content_type.as_bytes().to_vec()).append(Arc::new(sequence_bytes.clone()));
        }

        // Metaprotocol indexing
        if let Some(metaprotocol) = &entry.metaprotocol {
            METAPROTOCOL_TO_INSCRIPTIONS.select(&metaprotocol.as_bytes().to_vec()).append(Arc::new(sequence_bytes.clone()));
        }

        // Transaction tracking
        let txid_bytes = entry.id.txid.as_byte_array();
        TXID_TO_INSCRIPTIONS.select(&txid_bytes.to_vec()).append(Arc::new(sequence_bytes.clone()));
        INSCRIPTION_TO_TXID.select(&sequence_bytes).set(Arc::new(txid_bytes.to_vec()));

        // Store content if present
        if let Some(body) = &envelope.payload.body {
            // Store content using inscription ID string as key (for view function compatibility)
            let inscription_id_str = format!("{}i{}", entry.id.txid, entry.id.index);
            INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).set(Arc::new(body.to_vec()));
        }

        // Store metadata if present
        if let Some(metadata) = &envelope.payload.metadata {
            // Store metadata using inscription ID string as key (for view function compatibility)
            let inscription_id_str = format!("{}i{}", entry.id.txid, entry.id.index);
            INSCRIPTION_METADATA.select(&inscription_id_str.as_bytes().to_vec()).set(Arc::new(metadata.to_vec()));
        }

        Ok(())
    }

    /// Check if inscription is cursed by context (not just envelope content)
    fn is_cursed_by_context(&self, _envelope: &Envelope, tx_index: usize) -> bool {
        // Inscriptions in coinbase transactions are cursed
        tx_index == 0
    }

    /// Calculate satpoint for inscription
    fn calculate_satpoint(&self, tx: &Transaction, envelope: &Envelope, _sat_ranges: &SatRanges) -> Result<SatPoint, IndexError> {
        // An inscription is made on the first sat of the first output of its reveal transaction.
        // The ord spec allows a pointer to move the inscription to a different output.
        let vout = envelope.payload.pointer_value().unwrap_or(0) as u32;
        let offset = 0; // Simplification: offset is within the output, not across all outputs

        let outpoint = OutPoint {
            txid: tx.txid(),
            vout,
        };

        Ok(SatPoint::new(outpoint, offset))
    }

    /// Calculate sat number for a satpoint
    fn calculate_sat_number(&self, _satpoint: &SatPoint, _sat_ranges: &SatRanges) -> Option<u64> {
        // This would require full sat tracking implementation
        // For now, return None
        None
    }

    /// Calculate transaction fee
    fn calculate_fee(&self, _tx: &Transaction) -> u64 {
        // This would require input value calculation
        // For now, return 0
        0
    }

    /// Process a new inscription to see if it's a BRC20 operation
    fn process_brc20_inscription(&self, tx: &Transaction, entry: &InscriptionEntry, envelope: &Envelope) -> Result<(), IndexError> {
        if let Some(body) = &envelope.payload.body {
            if let Some(content_type) = &entry.content_type {
                if content_type.starts_with("text/plain") || content_type.starts_with("application/json") {
                    let brc20_indexer = Brc20Indexer::new();
                    if let Some(operation) = brc20_indexer.parse_operation(body) {
                        // The owner of a new inscription is the address of the first output
                        if let Some(first_output) = tx.output.get(0) {
                            if let Some(address) = get_address_from_txout(first_output, self.network) {
                                if let Err(e) = brc20_indexer.process_operation(&operation, &entry.id.to_string(), &address.to_string()) {
                                    println!("BRC20 Process Error: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Process a transaction to see if it's spending a BRC20 transfer inscription
    fn process_brc20_transfers(&self, tx: &Transaction) -> Result<(), IndexError> {
        let brc20_indexer = Brc20Indexer::new();
        let transferable_table = Brc20TransferableInscriptions::new();
        for input in &tx.input {
            let outpoint_bytes = input.previous_output.txid.as_byte_array()
                .iter()
                .chain(input.previous_output.vout.to_le_bytes().iter())
                .copied()
                .collect::<Vec<u8>>();
            
            let inscription_sequences = OUTPOINT_TO_INSCRIPTIONS.select(&outpoint_bytes).get_list();
            if inscription_sequences.is_empty() {
                continue;
            }

            for seq_bytes in inscription_sequences {
                let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
                if entry_bytes.is_empty() { continue; }

                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    let inscription_id_str = entry.id.to_string();
                    if let Some(transfer_info_bytes) = transferable_table.get(&inscription_id_str) {
                        if let Ok(transfer_info) = serde_json::from_slice::<crate::brc20::TransferInfo>(&transfer_info_bytes) {
                            if let Some(first_output) = tx.output.get(0) {
                                if let Some(new_owner) = get_address_from_txout(first_output, self.network) {
                                    brc20_indexer.claim_transfer(&new_owner.to_string(), &transfer_info).ok();
                                    transferable_table.delete(&inscription_id_str);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Sat range tracking for transactions
pub struct SatRanges {
    ranges: HashMap<OutPoint, (u64, u64)>, // (start_sat, end_sat)
}

impl SatRanges {
    pub fn new() -> Self {
        Self {
            ranges: HashMap::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: &Transaction, _is_coinbase: bool) -> Result<(), IndexError> {
        // This would implement full sat range tracking
        // For now, just store empty ranges
        for (vout, _output) in tx.output.iter().enumerate() {
            let outpoint = OutPoint {
                txid: tx.txid(),
                vout: vout as u32,
            };
            self.ranges.insert(outpoint, (0, 0));
        }
        Ok(())
    }

    pub fn get_range(&self, outpoint: &OutPoint) -> Option<(u64, u64)> {
        self.ranges.get(outpoint).copied()
    }
}

/// Result of indexing a block
#[derive(Debug)]
pub struct BlockIndexResult {
    pub height: u32,
    pub block_hash: bitcoin::BlockHash,
    pub inscriptions: Vec<InscriptionEntry>,
    pub transactions_processed: usize,
}

impl BlockIndexResult {
    pub fn new(height: u32, block_hash: bitcoin::BlockHash) -> Self {
        Self {
            height,
            block_hash,
            inscriptions: Vec::new(),
            transactions_processed: 0,
        }
    }

    pub fn merge(&mut self, tx_result: TransactionIndexResult) {
        self.inscriptions.extend(tx_result.inscriptions);
        self.transactions_processed += 1;
    }
}

/// Result of indexing a transaction
#[derive(Debug)]
pub struct TransactionIndexResult {
    pub txid: Txid,
    pub inscriptions: Vec<InscriptionEntry>,
}

impl TransactionIndexResult {
    pub fn new(txid: Txid) -> Self {
        Self {
            txid,
            inscriptions: Vec::new(),
        }
    }

    pub fn merge(&mut self, inscription_result: InscriptionIndexResult) {
        self.inscriptions.push(inscription_result.inscription);
    }
}

/// Result of indexing a single inscription
#[derive(Debug)]
pub struct InscriptionIndexResult {
    pub inscription: InscriptionEntry,
    pub envelope: Envelope,
}

/// Errors that can occur during indexing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexError {
    InvalidData,
    ParseError,
    DuplicateInscription,
    InvalidInput,
    DatabaseError,
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::InvalidData => write!(f, "Invalid data"),
            IndexError::ParseError => write!(f, "Parse error"),
            IndexError::DuplicateInscription => write!(f, "Duplicate inscription"),
            IndexError::InvalidInput => write!(f, "Invalid input"),
            IndexError::DatabaseError => write!(f, "Database error"),
        }
    }
}

impl std::error::Error for IndexError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexer_initialization() {
        let indexer = InscriptionIndexer::new();
        assert_eq!(indexer.sequence_counter, 0);
        assert_eq!(indexer.blessed_counter, 0);
        assert_eq!(indexer.cursed_counter, -1);
    }

    #[test]
    fn test_sat_ranges() {
        let _ranges = SatRanges::new();
        // Test would require actual transaction data
    }
}
