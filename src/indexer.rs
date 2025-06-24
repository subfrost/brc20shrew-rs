use crate::{
    envelope::{parse_inscriptions_from_transaction, Envelope},
    inscription::{Charm, InscriptionEntry, InscriptionId, Rarity, SatPoint},
    tables::TABLES,
};
use bitcoin::{Block, OutPoint, Transaction, Txid};
use metashrew_core::index_pointer::IndexPointer;
use std::collections::{BTreeMap, HashMap};

/// Main indexer for processing Bitcoin blocks and extracting inscriptions
pub struct InscriptionIndexer {
    pub height: u32,
    pub block_hash: bitcoin::BlockHash,
    pub block_time: u32,
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
            sequence_counter: 0,
            blessed_counter: 0,
            cursed_counter: -1,
            jubilee_height: 824544, // Bitcoin block height where cursed inscriptions become blessed
        }
    }

    /// Initialize indexer state from database
    pub fn load_state(&mut self) -> Result<(), IndexError> {
        // Load counters from database
        if let Some(seq_bytes) = TABLES.GLOBAL_SEQUENCE_COUNTER.get() {
            self.sequence_counter = u32::from_le_bytes(
                seq_bytes.try_into().map_err(|_| IndexError::InvalidData)?,
            );
        }

        if let Some(blessed_bytes) = TABLES.BLESSED_INSCRIPTION_COUNTER.get() {
            self.blessed_counter = i32::from_le_bytes(
                blessed_bytes.try_into().map_err(|_| IndexError::InvalidData)?,
            );
        }

        if let Some(cursed_bytes) = TABLES.CURSED_INSCRIPTION_COUNTER.get() {
            self.cursed_counter = i32::from_le_bytes(
                cursed_bytes.try_into().map_err(|_| IndexError::InvalidData)?,
            );
        }

        Ok(())
    }

    /// Save indexer state to database
    pub fn save_state(&self) -> Result<(), IndexError> {
        TABLES.GLOBAL_SEQUENCE_COUNTER.set(&self.sequence_counter.to_le_bytes());
        TABLES.BLESSED_INSCRIPTION_COUNTER.set(&self.blessed_counter.to_le_bytes());
        TABLES.CURSED_INSCRIPTION_COUNTER.set(&self.cursed_counter.to_le_bytes());
        Ok(())
    }

    /// Process a Bitcoin block and index all inscriptions
    pub fn index_block(&mut self, block: &Block, height: u32) -> Result<BlockIndexResult, IndexError> {
        self.height = height;
        self.block_hash = block.block_hash();
        self.block_time = block.header.time;

        // Store block metadata
        TABLES.HEIGHT_TO_BLOCK_HASH.select(&height.to_le_bytes()).set(&self.block_hash.to_byte_array());
        TABLES.BLOCK_HASH_TO_HEIGHT.select(&self.block_hash.to_byte_array()).set(&height.to_le_bytes());

        let mut result = BlockIndexResult::new(height, self.block_hash);
        let mut sat_ranges = SatRanges::new();

        // Calculate sat ranges for all transaction inputs and outputs
        for (tx_index, tx) in block.txs.iter().enumerate() {
            sat_ranges.process_transaction(tx, tx_index == 0)?;
        }

        // Process transactions for inscriptions
        for (tx_index, tx) in block.txs.iter().enumerate() {
            let tx_result = self.index_transaction(tx, tx_index, &sat_ranges)?;
            result.merge(tx_result);
        }

        // Update height-based indexes
        if !result.inscriptions.is_empty() {
            let inscription_ids: Vec<_> = result.inscriptions.iter().map(|e| e.id.to_bytes()).collect();
            TABLES.HEIGHT_TO_INSCRIPTIONS.select(&height.to_le_bytes()).set_list(&inscription_ids);
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

        // Parse inscription envelopes from transaction
        let envelopes = parse_inscriptions_from_transaction(tx)
            .map_err(|_| IndexError::ParseError)?;

        if envelopes.is_empty() {
            return Ok(result);
        }

        // Process each inscription envelope
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
        if TABLES.INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.to_bytes()).get().is_some() {
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

        Ok(InscriptionIndexResult {
            inscription: entry,
            envelope: envelope.clone(),
        })
    }

    /// Store inscription and related data in database
    fn store_inscription(&self, entry: &InscriptionEntry, envelope: &Envelope) -> Result<(), IndexError> {
        let id_bytes = entry.id.to_bytes();
        let sequence_bytes = entry.sequence.to_le_bytes();
        let entry_bytes = entry.to_bytes();

        // Core mappings
        TABLES.INSCRIPTION_ID_TO_SEQUENCE.select(&id_bytes).set(&sequence_bytes);
        TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).set(&entry_bytes);
        TABLES.INSCRIPTION_NUMBER_TO_SEQUENCE.select(&entry.number.to_le_bytes()).set(&sequence_bytes);

        // Location tracking
        TABLES.SEQUENCE_TO_SATPOINT.select(&sequence_bytes).set(&entry.satpoint.to_bytes());
        
        if let Some(sat) = entry.sat {
            TABLES.SAT_TO_SEQUENCE.select(&sat.to_le_bytes()).set(&sequence_bytes);
            TABLES.INSCRIPTION_TO_SAT.select(&sequence_bytes).set(&sat.to_le_bytes());
        }

        // Outpoint tracking
        let outpoint_bytes = entry.satpoint.outpoint.txid.to_byte_array()
            .iter()
            .chain(entry.satpoint.outpoint.vout.to_le_bytes().iter())
            .copied()
            .collect::<Vec<u8>>();
        TABLES.OUTPOINT_TO_INSCRIPTIONS.select(&outpoint_bytes).append(&sequence_bytes);

        // Parent-child relationships
        if let Some(parent_id) = &entry.parent {
            let parent_id_bytes = parent_id.to_bytes();
            if let Some(parent_seq_bytes) = TABLES.INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id_bytes).get() {
                TABLES.SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).append(&sequence_bytes);
                TABLES.SEQUENCE_TO_PARENTS.select(&sequence_bytes).append(&parent_seq_bytes);
            }
        }

        // Content type indexing
        if let Some(content_type) = &entry.content_type {
            TABLES.CONTENT_TYPE_TO_INSCRIPTIONS.select(content_type.as_bytes()).append(&sequence_bytes);
        }

        // Metaprotocol indexing
        if let Some(metaprotocol) = &entry.metaprotocol {
            TABLES.METAPROTOCOL_TO_INSCRIPTIONS.select(metaprotocol.as_bytes()).append(&sequence_bytes);
        }

        // Transaction tracking
        let txid_bytes = entry.id.txid.to_byte_array();
        TABLES.TXID_TO_INSCRIPTIONS.select(&txid_bytes).append(&sequence_bytes);
        TABLES.INSCRIPTION_TO_TXID.select(&sequence_bytes).set(&txid_bytes);

        // Store content if present
        if let Some(body) = &envelope.payload.body {
            TABLES.INSCRIPTION_CONTENT.select(&sequence_bytes).set(body);
        }

        // Store metadata if present
        if let Some(metadata) = &envelope.payload.metadata {
            TABLES.INSCRIPTION_METADATA.select(&sequence_bytes).set(metadata);
        }

        Ok(())
    }

    /// Check if inscription is cursed by context (not just envelope content)
    fn is_cursed_by_context(&self, envelope: &Envelope, tx_index: usize) -> bool {
        // Inscriptions in coinbase transactions are cursed
        tx_index == 0
    }

    /// Calculate satpoint for inscription
    fn calculate_satpoint(&self, tx: &Transaction, envelope: &Envelope, sat_ranges: &SatRanges) -> Result<SatPoint, IndexError> {
        if envelope.input >= tx.input.len() {
            return Err(IndexError::InvalidInput);
        }

        let input = &tx.input[envelope.input];
        let outpoint = input.previous_output;

        // For now, use offset 0 - in a full implementation, this would calculate
        // the exact sat offset based on the inscription's position in the input
        let offset = envelope.payload.pointer_value().unwrap_or(0);

        Ok(SatPoint::new(outpoint, offset))
    }

    /// Calculate sat number for a satpoint
    fn calculate_sat_number(&self, satpoint: &SatPoint, sat_ranges: &SatRanges) -> Option<u64> {
        // This would require full sat tracking implementation
        // For now, return None
        None
    }

    /// Calculate transaction fee
    fn calculate_fee(&self, tx: &Transaction) -> u64 {
        // This would require input value calculation
        // For now, return 0
        0
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

    pub fn process_transaction(&mut self, tx: &Transaction, is_coinbase: bool) -> Result<(), IndexError> {
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
    use bitcoin::{consensus::deserialize, hex};

    #[test]
    fn test_indexer_initialization() {
        let mut indexer = InscriptionIndexer::new();
        assert_eq!(indexer.sequence_counter, 0);
        assert_eq!(indexer.blessed_counter, 0);
        assert_eq!(indexer.cursed_counter, -1);
    }

    #[test]
    fn test_sat_ranges() {
        let mut ranges = SatRanges::new();
        // Test would require actual transaction data
    }
}