use shrew_support::inscription::{Charm, InscriptionEntry, InscriptionId, Rarity, SatPoint};
use crate::envelope::{parse_inscriptions_from_transaction, Envelope};
use crate::tables::*;
use bitcoin::{Block, OutPoint, Transaction, Txid, Network};
use bitcoin::consensus::serialize;
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
            jubilee_height: shrew_support::constants::JUBILEE_HEIGHT,
        }
    }

    pub fn load_state(&mut self) -> Result<(), IndexError> {
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

    pub fn save_state(&self) -> Result<(), IndexError> {
        GLOBAL_SEQUENCE_COUNTER.clone().set(Arc::new(self.sequence_counter.to_le_bytes().to_vec()));
        BLESSED_INSCRIPTION_COUNTER.clone().set(Arc::new(self.blessed_counter.to_le_bytes().to_vec()));
        CURSED_INSCRIPTION_COUNTER.clone().set(Arc::new(self.cursed_counter.to_le_bytes().to_vec()));
        Ok(())
    }

    pub fn index_block(&mut self, block: &Block, height: u32) -> Result<BlockIndexResult, IndexError> {
        self.height = height;
        self.block_hash = block.block_hash();
        self.block_time = block.header.time;

        HEIGHT_TO_BLOCK_HASH.select(&height.to_le_bytes().to_vec()).set(Arc::new(self.block_hash.as_byte_array().to_vec()));
        BLOCK_HASH_TO_HEIGHT.select(&self.block_hash.as_byte_array().to_vec()).set(Arc::new(height.to_le_bytes().to_vec()));

        // Index all transactions for BRC20-prog precompile lookups
        for tx in &block.txdata {
            let txid_bytes = tx.compute_txid().as_byte_array().to_vec();
            let raw_tx = serialize(tx);
            TXID_TO_RAW_TX.select(&txid_bytes).set(Arc::new(raw_tx));
            TXID_TO_BLOCK_HEIGHT.select(&txid_bytes).set(Arc::new(height.to_le_bytes().to_vec()));
        }

        let mut result = BlockIndexResult::new(height, self.block_hash);
        let mut sat_ranges = SatRanges::new();

        for (tx_index, tx) in block.txdata.iter().enumerate() {
            sat_ranges.process_transaction(tx, tx_index == 0)?;
        }

        for (tx_index, tx) in block.txdata.iter().enumerate() {
            let tx_result = self.index_transaction(tx, tx_index, &sat_ranges)?;
            result.merge(tx_result);
        }

        if !result.inscriptions.is_empty() {
            let inscription_ids: Vec<_> = result.inscriptions.iter().map(|e| e.id.to_bytes()).collect();
            for (i, inscription_id) in inscription_ids.iter().enumerate() {
                let key = format!("{}:{}", height, i);
                HEIGHT_TO_INSCRIPTIONS.select(&key.as_bytes().to_vec()).set(Arc::new(inscription_id.clone()));
            }
        }

        self.save_state()?;
        Ok(result)
    }

    fn index_transaction(
        &mut self,
        tx: &Transaction,
        tx_index: usize,
        sat_ranges: &SatRanges,
    ) -> Result<TransactionIndexResult, IndexError> {
        let mut result = TransactionIndexResult::new(tx.compute_txid());

        let envelopes = parse_inscriptions_from_transaction(tx)
            .map_err(|_| IndexError::ParseError)?;

        if envelopes.is_empty() {
            return Ok(result);
        }

        for envelope in envelopes {
            let inscription_result = self.process_inscription_envelope(tx, tx_index, &envelope, sat_ranges)?;
            result.merge(inscription_result);
        }

        Ok(result)
    }

    fn process_inscription_envelope(
        &mut self,
        tx: &Transaction,
        tx_index: usize,
        envelope: &Envelope,
        sat_ranges: &SatRanges,
    ) -> Result<InscriptionIndexResult, IndexError> {
        let inscription_id = InscriptionId::new(tx.compute_txid(), envelope.input as u32);

        if !INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.to_bytes()).get().is_empty() {
            return Err(IndexError::DuplicateInscription);
        }

        let is_cursed = envelope.payload.is_cursed() || self.is_cursed_by_context(envelope, tx_index);
        let number = if is_cursed && self.height < self.jubilee_height {
            self.cursed_counter -= 1;
            self.cursed_counter
        } else {
            self.blessed_counter += 1;
            self.blessed_counter
        };

        self.sequence_counter += 1;
        let sequence = self.sequence_counter;
        let satpoint = self.calculate_satpoint(tx, envelope, sat_ranges)?;

        let mut entry = InscriptionEntry::new(
            inscription_id.clone(), number, sequence, satpoint.clone(),
            self.height, self.calculate_fee(tx), self.block_time,
        );

        if let Some(content_type) = envelope.payload.content_type() { entry.content_type = Some(content_type); }
        if let Some(content_length) = envelope.payload.content_length() { entry.content_length = Some(content_length as u64); }
        if let Some(metaprotocol) = envelope.payload.metaprotocol() { entry.metaprotocol = Some(metaprotocol); }
        if let Some(parent_id) = envelope.payload.parent_id() { entry.parent = Some(parent_id); }
        if let Some(delegate_id) = envelope.payload.delegate_id() { entry.delegate = Some(delegate_id); }
        if let Some(pointer) = envelope.payload.pointer_value() { entry.pointer = Some(pointer); }

        if let Some(sat) = self.calculate_sat_number(&satpoint, sat_ranges) {
            entry.sat = Some(sat);
            let rarity = Rarity::from_sat(sat);
            match rarity {
                Rarity::Uncommon => entry.set_charm(Charm::Uncommon),
                Rarity::Rare => entry.set_charm(Charm::Rare),
                Rarity::Epic => entry.set_charm(Charm::Epic),
                Rarity::Legendary => entry.set_charm(Charm::Legendary),
                _ => {}
            }
        }

        if is_cursed { entry.set_charm(Charm::Cursed); }
        if envelope.payload.body.is_none() { entry.set_charm(Charm::Unbound); }

        self.store_inscription(&entry, envelope)?;

        Ok(InscriptionIndexResult {
            inscription: entry,
            envelope: envelope.clone(),
        })
    }

    fn store_inscription(&self, entry: &InscriptionEntry, envelope: &Envelope) -> Result<(), IndexError> {
        let id_bytes = entry.id.to_bytes();
        let sequence_bytes = entry.sequence.to_le_bytes().to_vec();
        let entry_bytes = entry.to_bytes();

        INSCRIPTION_ID_TO_SEQUENCE.select(&id_bytes).set(Arc::new(sequence_bytes.clone()));
        SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).set(Arc::new(entry_bytes));
        INSCRIPTION_NUMBER_TO_SEQUENCE.select(&entry.number.to_le_bytes().to_vec()).set(Arc::new(sequence_bytes.clone()));
        SEQUENCE_TO_SATPOINT.select(&sequence_bytes).set(Arc::new(entry.satpoint.to_bytes()));

        if let Some(sat) = entry.sat {
            SAT_TO_SEQUENCE.select(&sat.to_le_bytes().to_vec()).set(Arc::new(sequence_bytes.clone()));
            INSCRIPTION_TO_SAT.select(&sequence_bytes).set(Arc::new(sat.to_le_bytes().to_vec()));
        }

        let outpoint_bytes = entry.satpoint.outpoint.txid.as_byte_array()
            .iter().chain(entry.satpoint.outpoint.vout.to_le_bytes().iter()).copied().collect::<Vec<u8>>();
        OUTPOINT_TO_INSCRIPTIONS.select(&outpoint_bytes).append(Arc::new(sequence_bytes.clone()));

        if let Some(parent_id) = &entry.parent {
            let parent_id_bytes = parent_id.to_bytes();
            let parent_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id_bytes).get();
            if !parent_seq_bytes.is_empty() {
                SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).append(Arc::new(sequence_bytes.clone()));
                SEQUENCE_TO_PARENTS.select(&sequence_bytes).append(Arc::new(parent_seq_bytes.to_vec()));
            }
        }

        if let Some(content_type) = &entry.content_type {
            CONTENT_TYPE_TO_INSCRIPTIONS.select(&content_type.as_bytes().to_vec()).append(Arc::new(sequence_bytes.clone()));
        }
        if let Some(metaprotocol) = &entry.metaprotocol {
            METAPROTOCOL_TO_INSCRIPTIONS.select(&metaprotocol.as_bytes().to_vec()).append(Arc::new(sequence_bytes.clone()));
        }

        let txid_bytes = entry.id.txid.as_byte_array();
        TXID_TO_INSCRIPTIONS.select(&txid_bytes.to_vec()).append(Arc::new(sequence_bytes.clone()));
        INSCRIPTION_TO_TXID.select(&sequence_bytes).set(Arc::new(txid_bytes.to_vec()));

        if let Some(body) = &envelope.payload.body {
            let inscription_id_str = format!("{}i{}", entry.id.txid, entry.id.index);
            INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).set(Arc::new(body.to_vec()));
        }
        if let Some(metadata) = &envelope.payload.metadata {
            let inscription_id_str = format!("{}i{}", entry.id.txid, entry.id.index);
            INSCRIPTION_METADATA.select(&inscription_id_str.as_bytes().to_vec()).set(Arc::new(metadata.to_vec()));
        }

        Ok(())
    }

    fn is_cursed_by_context(&self, _envelope: &Envelope, tx_index: usize) -> bool {
        tx_index == 0
    }

    fn calculate_satpoint(&self, tx: &Transaction, envelope: &Envelope, _sat_ranges: &SatRanges) -> Result<SatPoint, IndexError> {
        let vout = envelope.payload.pointer_value().unwrap_or(0) as u32;
        let outpoint = OutPoint { txid: tx.compute_txid(), vout };
        Ok(SatPoint::new(outpoint, 0))
    }

    fn calculate_sat_number(&self, _satpoint: &SatPoint, _sat_ranges: &SatRanges) -> Option<u64> {
        None
    }

    fn calculate_fee(&self, _tx: &Transaction) -> u64 {
        0
    }
}

/// Sat range tracking for transactions
pub struct SatRanges {
    ranges: HashMap<OutPoint, (u64, u64)>,
}

impl SatRanges {
    pub fn new() -> Self { Self { ranges: HashMap::new() } }

    pub fn process_transaction(&mut self, tx: &Transaction, _is_coinbase: bool) -> Result<(), IndexError> {
        for (vout, _output) in tx.output.iter().enumerate() {
            let outpoint = OutPoint { txid: tx.compute_txid(), vout: vout as u32 };
            self.ranges.insert(outpoint, (0, 0));
        }
        Ok(())
    }

    pub fn get_range(&self, outpoint: &OutPoint) -> Option<(u64, u64)> {
        self.ranges.get(outpoint).copied()
    }
}

#[derive(Debug)]
pub struct BlockIndexResult {
    pub height: u32,
    pub block_hash: bitcoin::BlockHash,
    pub inscriptions: Vec<InscriptionEntry>,
    pub transactions_processed: usize,
}

impl BlockIndexResult {
    pub fn new(height: u32, block_hash: bitcoin::BlockHash) -> Self {
        Self { height, block_hash, inscriptions: Vec::new(), transactions_processed: 0 }
    }
    pub fn merge(&mut self, tx_result: TransactionIndexResult) {
        self.inscriptions.extend(tx_result.inscriptions);
        self.transactions_processed += 1;
    }
}

#[derive(Debug)]
pub struct TransactionIndexResult {
    pub txid: Txid,
    pub inscriptions: Vec<InscriptionEntry>,
}

impl TransactionIndexResult {
    pub fn new(txid: Txid) -> Self { Self { txid, inscriptions: Vec::new() } }
    pub fn merge(&mut self, inscription_result: InscriptionIndexResult) {
        self.inscriptions.push(inscription_result.inscription);
    }
}

#[derive(Debug)]
pub struct InscriptionIndexResult {
    pub inscription: InscriptionEntry,
    pub envelope: Envelope,
}

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
