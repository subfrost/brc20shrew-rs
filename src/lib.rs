use bitcoin::block::Block;  
use bitcoin::transaction::Transaction;
use bitcoin::hash_types::BlockHash;
use bitcoin::OutPoint;
use bitcoin::{Txid};
use metashrew_support::utils::{consensus_encode};
use bitcoin::hashes::{Hash};
use metashrew_support::index_pointer::KeyValuePointer;
use metashrew_support::byte_view::ByteView;
use std::sync::Arc;
use std::ptr::addr_of_mut;
use std::fmt;
use std::error::Error;
use serde::{Serialize, Deserialize};
use bitcoin::hex::DisplayHex;
use std::collections::{HashMap, HashSet};
use bitcoin::TxOut;
use bincode;
use metashrew::index_pointer::IndexPointer;
use anyhow::{Result};

#[derive(Debug)]
pub enum InscriptionError {
    InvalidInput(String),
    DatabaseError(String),
    TransactionError(String),
    EncodingError(String),
    ValidationError(String),
}

fn value_to_option<T: ByteView>(v: T) -> Option<T> {
  if T::zero() == v {
    None
  } else {
    Some(v)
  }
}

fn to_option_value<T: KeyValuePointer, U: ByteView>(v: T) -> Option<U> {
  if v.get() == 0 {
    None
  } else {
    Some(v.get_value::<U>())
  }
}

fn to_option(v: Arc<Vec<u8>>) -> Option<&Vec<u8>> {
  if v.len() == 0 {
    None
  } else {
    Some(v.as_ref())
  }
}

impl From<anyhow::Error> for InscriptionError {
    fn from(err: anyhow::Error) -> Self {
        InscriptionError::EncodingError(err.to_string()) 
    }
}
impl fmt::Display for InscriptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            Self::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl Error for InscriptionError {}


pub mod bst;
pub mod tables;

use crate::bst::BST;
use crate::tables::{INSCRIPTIONS, InscriptionTable};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inscription {
    pub media_type: Option<Arc<Vec<u8>>>,
    pub content_bytes: Arc<Vec<u8>>, 
    pub parent: Option<Arc<Vec<u8>>>,
    pub metadata: Option<Arc<Vec<u8>>>,
    pub number: i64,
    pub sequence_number: u64,
    pub fee: u64,
    pub height: u32,
    pub timestamp: u32,
    pub cursed: bool,
    pub unbound: bool,
    pub pointer: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InscriptionEntry {
    pub id: Vec<u8>,
    pub number: i64,
    pub sequence_number: u64,
    pub timestamp: u32,
    pub height: u32,
    pub fee: u64,
    pub sat: Option<u64>,
    pub parents: Vec<Vec<u8>>,
    pub children: Vec<Vec<u8>>,
    pub cursed: bool,
    pub blessed: bool,
    pub unbound: bool,
    pub charms: u32, // Add charms bitfield
    pub media_type: Option<Arc<Vec<u8>>>, // Add media type
    pub content_length: Option<u64>,       // Add content length
    pub delegate: Option<Vec<u8>>,         // Add delegate support
}
#[derive(Debug, Copy, Clone)]
pub enum Charm {
    Cursed = 0,
    Reinscription = 1, 
    Unbound = 2,
    Lost = 3,
    Burned = 4,
    Vindicated = 5,
}

impl Charm {
    pub fn is_set(self, charms: u32) -> bool {
        charms & (1 << self as u32) != 0
    }

    pub fn set(self, charms: &mut u32) {
        *charms |= 1 << self as u32;
    }

    pub fn unset(self, charms: &mut u32) {
        *charms &= !(1 << self as u32);
    }
    
    pub fn charms(charms: u32) -> Vec<&'static str> {
        let mut result = Vec::new();
        if Charm::Cursed.is_set(charms) { result.push("cursed"); }
        if Charm::Reinscription.is_set(charms) { result.push("reinscription"); }
        if Charm::Unbound.is_set(charms) { result.push("unbound"); }
        if Charm::Lost.is_set(charms) { result.push("lost"); }
        if Charm::Burned.is_set(charms) { result.push("burned"); }
        if Charm::Vindicated.is_set(charms) { result.push("vindicated"); }
        result
    }
}

#[derive(Debug, Default, Clone)]
pub struct Index {
  tables: InscriptionTable
}
impl Index {
    pub fn new() -> Self {
        Self {
            tables: InscriptionTable::new()
        }
    }
    fn validate_inscription(&self, inscription: &Inscription) -> Result<()> {
        // Validate content type if present
        if let Some(ref media_type) = inscription.media_type {
            if media_type.len() > 255 {
                return Err(InscriptionError::ValidationError(
                    "Media type too long".to_string()
                ).into());
            }
        }

        // Validate content length
        if inscription.content_bytes.len() > 1024 * 1024 { // 1MB limit
            return Err(InscriptionError::ValidationError(
                "Content too large".to_string()
            ).into());
        }

        Ok(())
    }
    fn get_transaction(&self, txid: Txid) -> Result<Transaction> {
        let inscriptions = INSCRIPTIONS.read().map_err(|e|
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;
        
        let tx_bytes = inscriptions.transaction_id_to_transaction
            .select(&txid.as_byte_array().to_vec())
            .get();
            
        bitcoin::consensus::deserialize(&tx_bytes)
            .map_err(|e| InscriptionError::EncodingError(format!("Failed to deserialize transaction: {}", e)).into())
    }

    fn validate_inscription(&self, inscription: &Inscription) -> Result<()> {
        // Validate content type if present
        if let Some(ref media_type) = inscription.media_type {
            if media_type.len() > 255 {
                return Err(InscriptionError::ValidationError(
                    "Media type too long".to_string()
                ).into());
            }
        }

        // Validate content length
        if inscription.content_bytes.len() > 1024 * 1024 { // 1MB limit
            return Err(InscriptionError::ValidationError(
                "Content too large".to_string()
            ).into());
        }

        // Validate pointer
        if let Some(pointer) = inscription.pointer {
            // Pointer must point to valid output value range
            if pointer >= self.total_output_value {
                return Err(InscriptionError::ValidationError("Invalid pointer".to_string()).into());
            }
        }

        Ok(())
    }
    fn index_transaction_values(
        &self,
        tx: &Transaction,
    ) -> Result<()> {
        let mut inscriptions = INSCRIPTIONS.write().unwrap();
        for (idx, output) in tx.output.iter().enumerate() {
            let outpoint = OutPoint::new(tx.txid(), idx as u32);
            inscriptions.outpoint_to_value
                .select(&consensus_encode(&outpoint)?)
                .set_value(output.value.to_sat());
        }
        Ok(())
    }

    fn index_sat_ranges(
        &self,
        tx: &Transaction,
        starting_sat: u64,
        reward: u64,
    ) -> Result<()> {
        let mut sink = SatSink::new(tx);
        let inscriptions = INSCRIPTIONS.read().unwrap();
        let source = if reward > 0 {
            // Coinbase source
            SatSource::new(starting_sat, reward)
        } else {
            // Regular tx source from inputs
            SatSource::from_inputs(tx, &inscriptions)? 
        };

        sink.consume(source, &inscriptions)?;
        Ok(())
    }


    fn extract_inscription(&self, input: &bitcoin::TxIn) -> Option<Inscription> {
        if input.witness.len() < 2 {
            return None;
        }

        let mut content_type: Option<Arc<Vec<u8>>> = None;
        let mut content: Option<Vec<u8>> = None; 
        let mut parent: Option<Arc<Vec<u8>>> = None;
        let mut metadata: Option<Arc<Vec<u8>>> = None;
        let mut pointer: Option<u64> = None;
        let mut cursed = false;
        let mut unbound = false;

        let mut stack = input.witness.iter();
        
        while let Some(element) = stack.next() {
            if element == b"ord" {
                // Parse content type
                if let Some(ct) = stack.next() {
                    content_type = Some(Arc::new(ct.to_vec()));
                }

                // Parse content
                if let Some(data) = stack.next() {
                    content = Some(data.to_vec());
                }

                // Parse pointer if present
                if let Some(p) = stack.next() {
                    if p.starts_with(b"pointer=") {
                        if let Ok(ptr) = std::str::from_utf8(&p[8..])
                            .ok()
                            .and_then(|s| s.parse::<u64>().ok()) 
                        {
                            pointer = Some(ptr);
                        }
                    }
                }

                // Parse parent if present  
                if let Some(p) = stack.next() {
                    if p.starts_with(b"parent=") {
                        parent = Some(Arc::new(p[7..].to_vec()));
                    }
                }

                // Parse metadata if present
                if let Some(m) = stack.next() {
                    if m.starts_with(b"metadata=") {
                        metadata = Some(Arc::new(m[9..].to_vec()));
                    }
                }

                break;
            }
        }

        let content = content?;

        Some(Inscription {
            media_type: content_type,
            content_bytes: Arc::new(content),
            parent,
            metadata,
            number: 0,
            sequence_number: 0, 
            fee: 0,
            height: 0,
            timestamp: 0,
            cursed,
            unbound,
            pointer
        })
    }
    fn index_transaction_inscriptions(
        &self,
        tx: &Transaction,
        height: u32,
        tx_id: Vec<u8>,
        timestamp: u32,
    ) -> Result<()> {
        let mut inscriptions = INSCRIPTIONS.write().unwrap();
        let mut offset: u64 = 0;
        let mut output_index: u32 = 0;
        let mut inscription_inputs = HashMap::new();
        let mut potential_parents = HashSet::new();

        // First pass - collect parents and inscriptions from inputs
        for (input_index, input) in tx.input.iter().enumerate() {
            if let Some(inscription) = self.extract_inscription(input) {
                inscription_inputs.insert(input_index, inscription);
                
                if let Some(parent) = inscription.parent.as_ref() {
                    potential_parents.insert(parent.clone());
                }
            }
            
            // Track sat movement
            let value = inscriptions.outpoint_to_value
                .select(&consensus_encode(&input.previous_output)?)
                .get_value::<u64>();
                
            offset += value;
            if offset >= tx.output[output_index as usize].value.to_sat() {
                output_index += 1;
                offset = 0;
            }
        }

        let total_fee = tx.input.iter().fold(0u64, |acc, input| {
            acc + inscriptions.outpoint_to_value
                .select(&consensus_encode(&input.previous_output).unwrap())
                .get_value::<u64>()
        }) - tx.output.iter().fold(0u64, |acc, output| acc + output.value.to_sat());

        let fee_per_inscription = if !inscription_inputs.is_empty() {
            total_fee / inscription_inputs.len() as u64
        } else {
            0
        };

        // Second pass - process inscriptions and update indices
        for (input_index, inscription) in inscription_inputs {
            let sequence_num = inscriptions.next_sequence_number.get_value::<u64>() + 1;
            let outpoint = OutPoint::new(
                Txid::from_byte_array(<Vec<u8> as AsRef<[u8]>>::as_ref(&tx_id).try_into()?),
                output_index
            );
            let sat_point = format!("{}:{}", outpoint, offset);

            // Get sat position if not unbound
            let sat = if !inscription.unbound {
                Some(
                    inscriptions.outpoint_to_sat
                        .select(&consensus_encode(&outpoint)?)
                        .select_index(0)
                        .get_value::<u64>()
                )
            } else {
                None
            };

            // Create inscription entry
            let entry = InscriptionEntry {
                id: format!("{}:{}", sat_point, 0).into_bytes(),
                number: inscription.number,
                sequence_number: sequence_num,
                timestamp,
                height,
                fee: fee_per_inscription,
                sat,
                parents: inscription.parent.map(|p| vec![p.to_vec()]).unwrap_or_default(),
                children: Vec::new(),
                cursed: inscription.cursed,
                blessed: !inscription.cursed,
                unbound: inscription.unbound,
                charms: self.calculate_charms(&inscription, sat, outpoint),
                media_type: inscription.media_type.clone(),
                content_length: Some(inscription.content_bytes.len() as u64),
                delegate: None, // TODO: Add delegate extraction from inscription
            };

            // Update inscription mappings
            inscriptions.inscription_id_to_inscription
                .select(&entry.id)
                .set(Arc::new(inscription.content_bytes.to_vec()));
            
            if let Some(media_type) = inscription.media_type {
                inscriptions.inscription_id_to_media_type
                    .select(&entry.id)
                    .set(media_type);
            }

            if let Some(metadata) = inscription.metadata {
                inscriptions.inscription_id_to_metadata
                    .select(&entry.id)
                    .set(metadata);
            }
            
            inscriptions.satpoint_to_inscription_id
                .select(&sat_point.clone().into_bytes())
                .set(Arc::new(entry.id.clone()));

            inscriptions.inscription_id_to_satpoint
                .select(&entry.id)
                .set(Arc::new(sat_point.clone().into_bytes()));

            inscriptions.inscription_id_to_blockheight
                .select(&entry.id)
                .set_value(height);

            inscriptions.height_to_inscription_ids
                .select_value(height)
                .append(Arc::new(entry.id.clone()));

            inscriptions.sequence_number_to_inscription_id
                .select_value::<u64>(sequence_num)
                .set(Arc::new(entry.id.clone()));

            inscriptions.inscription_id_to_sequence_number
                .select(&entry.id)
                .set_value::<u64>(sequence_num);

            inscriptions.inscription_entries
                .select(&entry.id)
                .set(Arc::new(bincode::serialize(&entry).unwrap()));

            // Update parent-child relationships
            if let Some(parent_id) = inscription.parent {
                if let Some(parent_sequence) = to_option_value::<IndexPointer, u64>(inscriptions.inscription_id_to_sequence_number
                    .select(&parent_id)) {
                        inscriptions.sequence_number_to_children
                            .select_value(parent_sequence)
                            .append(Arc::new(entry.id.clone()));
                }
            }

            // Track blessed/cursed inscription numbers
            if inscription.cursed {
                let cursed_num = inscriptions.cursed_inscription_count.get_value::<i64>() + 1;
                inscriptions.cursed_inscription_count.set_value(cursed_num);
                inscriptions.cursed_inscription_numbers
                    .select_value(-cursed_num)
                    .set(Arc::new(entry.id.clone()));
            } else {
                let blessed_num = inscriptions.blessed_inscription_count.get_value::<i64>() + 1;
                inscriptions.blessed_inscription_count.set_value(blessed_num);
                inscriptions.blessed_inscription_numbers
                    .select_value(blessed_num)
                    .set(Arc::new(entry.id.clone()));
            }

            // Update next sequence number
            inscriptions.next_sequence_number.set_value::<u64>(sequence_num);
        }

        Ok(())
    }

    fn calculate_charms(
        &self,
        inscription: &Inscription,
        sat: Option<u64>,
        outpoint: OutPoint,
        sat_point: &str,
    ) -> u32 {
        let mut charms = 0;
        
        // Set basic charms
        if inscription.cursed {
            Charm::Cursed.set(&mut charms);
        }
        
        if inscription.unbound {
            Charm::Unbound.set(&mut charms);
        }
        
        // Check for lost inscriptions
        if outpoint == OutPoint::null() {
            Charm::Lost.set(&mut charms);
        }
        
        // Check for reinscription
        let inscriptions = INSCRIPTIONS.read().unwrap();
        if inscriptions.satpoint_to_inscription_id
            .select(sat_point.as_bytes())
            .get()
            .len() != 0 {
            Charm::Reinscription.set(&mut charms);
        }
        
        // Check for burned inscriptions (OP_RETURN outputs)
        if let Some(txout) = self.get_txout(&outpoint) {
            if txout.script_pubkey.is_op_return() {
                Charm::Burned.set(&mut charms);
            }
        }

        // Add vindicated charm for inscriptions that would be cursed at jubilee height
        if inscription.cursed && self.is_jubilee_height(inscription.height) {
            Charm::Vindicated.set(&mut charms);
            Charm::Cursed.unset(&mut charms); 
        }

        charms
    }
    fn get_txout(&self, outpoint: &OutPoint) -> Option<TxOut> {
        let inscriptions = INSCRIPTIONS.read().unwrap();
        inscriptions.transaction_id_to_transaction
            .select(&outpoint.txid.as_ref())
            .get()
            .and_then(|tx_bytes| {
                bitcoin::consensus::deserialize(&tx_bytes).ok()
            })
            .and_then(|tx: Transaction| {
                tx.output.get(outpoint.vout as usize).cloned()
            })
    }
    fn is_jubilee_height(&self, height: u32) -> bool {
        // Add jubilee height logic here
        // For now returning false
        false
    }
    pub fn extract_inscription(&self, input: &bitcoin::TxIn) -> Option<Inscription> {
        // Extract envelope from witness data
        if input.witness.len() < 2 {
            return None;
        }

        let mut content_type: Option<Arc<Vec<u8>>> = None;
        let mut content: Option<Vec<u8>> = None;
        let mut parent: Option<Arc<Vec<u8>>> = None;
        let mut metadata: Option<Arc<Vec<u8>>> = None;
        let mut cursed = false;
        let mut unbound = false;

        // Parse witness stack
        let mut stack = input.witness.iter();
        
        while let Some(element) = stack.next() {
            if element == b"ord" {
                // Found inscription marker
                
                // Parse content type
                if let Some(ct) = stack.next() {
                    content_type = Some(Arc::new(ct.to_vec()));
                }

                // Parse content
                if let Some(data) = stack.next() {
                    content = Some(data.to_vec());
                }

                // Parse optional parent
                if let Some(p) = stack.next() {
                    if p.starts_with(b"parent=") {
                        parent = Some(Arc::new(p[7..].to_vec()));
                    }
                }

                // Parse optional metadata
                if let Some(m) = stack.next() {
                    if m.starts_with(b"metadata=") {
                        metadata = Some(Arc::new(m[9..].to_vec()));  
                    }
                }

                break;
            }
        }

        // Require content to be present
        let content = content?;

        // Create inscription
        Some(Inscription {
            media_type: content_type,
            content_bytes: Arc::new(content),
            parent,
            metadata,
            number: 0, // Will be set later based on blessed/cursed status
            sequence_number: 0, // Will be set during indexing
            cursed,
            unbound
        })
    }
    pub fn get_inscription_by_id(&self, id: Vec<u8>) -> Result<Option<Inscription>> {
        let inscriptions = INSCRIPTIONS.read().unwrap();
        
        // Get inscription data
        let content = inscriptions.inscription_id_to_inscription
            .select(&id)
            .get_value::<Arc<Vec<u8>>>()
            .map(|bytes| Arc::new(bytes.to_vec()));

        if content.is_none() {
            return Ok(None);
        }

        // Get media type
        let media_type = inscriptions.inscription_id_to_media_type
            .select(&id)
            .get_value::<Arc<Vec<u8>>>();

        // Get metadata
        let metadata = inscriptions.inscription_id_to_metadata
            .select(&id)
            .get_value::<Arc<Vec<u8>>>();

        // Get entry
        let entry = inscriptions.inscription_entries
            .select(&id)
            .get()
            .and_then(|entry_bytes| {
                bincode::deserialize::<InscriptionEntry>(&entry_bytes).ok()
            });

        let entry = match entry {
            Some(e) => e,
            None => return Ok(None)
        };

        Ok(Some(Inscription {
            media_type,
            content_bytes: content.unwrap(),
            parent: entry.parents.first().map(|p| Arc::new(p.clone())),
            metadata,
            number: entry.number,
            sequence_number: entry.sequence_number,
            cursed: entry.cursed,
            unbound: entry.unbound
        }))
    }
    pub fn get_inscription_entry(&self, id: Vec<u8>) -> Result<Option<InscriptionEntry>> {
        let inscriptions = INSCRIPTIONS.read().unwrap();
        
        Ok(inscriptions.inscription_entries
            .select(&id)
            .get()
            .and_then(|bytes| bincode::deserialize(&bytes).ok()))
    }
    pub fn get_inscription_entries(&self, height: u32) -> Result<Vec<InscriptionEntry>> {
        let inscriptions = INSCRIPTIONS.read().map_err(|e| 
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;
        
        let mut entries = Vec::new();
        
        // Get inscription IDs for height
        let ids = inscriptions.height_to_inscription_ids
            .select_value(height)
            .get_list();

        // Get entries
        for id in ids {
            if let Some(entry_bytes) = inscriptions.inscription_entries.select(&id).get() {
                if let Ok(entry) = bincode::deserialize::<InscriptionEntry>(&entry_bytes) {
                    entries.push(entry);
                }
            }
        }

        entries.sort_by_key(|entry| entry.sequence_number);
        
        Ok(entries)
    }
    pub fn is_cursed(&self, id: &[u8]) -> Result<bool> {
        let inscriptions = INSCRIPTIONS.read().map_err(|e|
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;

        let entry_bytes = inscriptions.inscription_entries
            .select(&id.to_vec())
            .get();
            
        if entry_bytes.is_empty() {
            return Ok(false);
        }

        match bincode::deserialize::<InscriptionEntry>(&entry_bytes) {
            Ok(entry) => Ok(entry.cursed),
            Err(_) => Ok(false)
        }
    }

    fn update_delegate(&mut self, id: &[u8], delegate: Option<Vec<u8>>) -> Result<()> {
        let mut inscriptions = INSCRIPTIONS.write().map_err(|e|
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;

        let entry_bytes = inscriptions.inscription_entries.select(&id.to_vec()).get();
        if !entry_bytes.is_empty() {
            let mut entry: InscriptionEntry = bincode::deserialize(&entry_bytes)?;
            entry.delegate = delegate;
            inscriptions.inscription_entries.select(id).set(Arc::new(bincode::serialize(&entry)?));
        }

        Ok(())
    }

    pub fn parse_metadata(&self, metadata: &[u8]) -> Result<Option<serde_cbor::Value>> {
        if metadata.is_empty() {
            return Ok(None);
        }

        serde_cbor::from_slice(metadata)
            .map_err(|e| InscriptionError::EncodingError(format!("Invalid CBOR metadata: {}", e)))
            .map(Some)
    }

    pub fn get_children(&self, id: &[u8]) -> Result<Vec<InscriptionEntry>> {
        let inscriptions = INSCRIPTIONS.read().map_err(|e|
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;
            
        let seq_num_bytes = inscriptions.inscription_id_to_sequence_number
            .select(&id.to_vec())
            .get();
            
        if seq_num_bytes.is_empty() {
            return Err(InscriptionError::InvalidInput("Inscription not found".to_string()).into());
        }

        let sequence_number = u64::from_be_bytes(seq_num_bytes.as_slice().try_into()?);

        let mut children = Vec::new();
        let child_sequences = inscriptions.sequence_number_to_children
            .select_value(sequence_number)
            .get_list_values::<u64>();

        for child_seq in child_sequences {
            let entry_bytes = inscriptions.sequence_number_to_entry.select(&child_seq).get();
            if !entry_bytes.is_empty() {
                if let Ok(entry) = bincode::deserialize(&entry_bytes) {
                    children.push(entry);
                }
            }
        }

        Ok(children)
    }

    pub fn get_inscription_by_number(&self, number: i64) -> Result<Option<InscriptionEntry>> {
        let inscriptions = INSCRIPTIONS.read().map_err(|e|
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;
            
        let inscription_id = if number < 0 {
            inscriptions.cursed_inscription_numbers
                .select_value(number as u64)
                .get()
        } else {
            inscriptions.blessed_inscription_numbers
                .select_value(number as u64)
                .get()
        };

        if inscription_id.is_empty() {
            return Ok(None);
        }

        let seq_num_bytes = inscriptions.inscription_id_to_sequence_number
            .select(&inscription_id)
            .get();
            
        if seq_num_bytes.is_empty() {
            return Ok(None);
        }

        let sequence_number = u64::from_be_bytes(seq_num_bytes.as_slice().try_into()?);
        let entry_bytes = inscriptions.sequence_number_to_entry.select(&sequence_number).get();

        if entry_bytes.is_empty() {
            return Ok(None);
        }

        match bincode::deserialize(&entry_bytes) {
            Ok(entry) => Ok(Some(entry)),
            Err(_) => Ok(None)
        }
    }

    pub fn index_block(&self, block: &Block, height: u32, timestamp: u32) -> Result<()> {
        let mut inscriptions = INSCRIPTIONS.write().map_err(|e| 
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e)))?;

        // Track running counts
        let mut cursed_count = inscriptions.cursed_count.get_value::<u64>();
        let mut blessed_count = inscriptions.blessed_count.get_value::<u64>();
        let mut seq_num = inscriptions.next_sequence_number.get_value::<u64>();
        
        let mut inscription_updates = Vec::new();
        let reward = block_reward(height);
        let mut running_sats = 0u64;

        // Index transactions
        for (tx_index, tx) in block.txdata.iter().enumerate() {
            let is_coinbase = tx_index == 0;
            let tx_id = tx.txid().as_byte_array().to_vec();
            
            // Track sat ranges
            let mut input_value = if is_coinbase { reward } else { 0 };
            for input in &tx.input {
                if !input.previous_output.is_null() {
                    input_value += inscriptions.outpoint_to_value
                        .select(&consensus_encode(&input.previous_output)?)
                        .get_value::<u64>();
                }
            }

            let mut output_offset = 0u64;
            for (vout, output) in tx.output.iter().enumerate() {
                // Extract inscriptions
                for (input_index, input) in tx.input.iter().enumerate() {
                    if let Some(mut inscription) = self.extract_inscription(input) {
                        // Set inscription metadata
                        inscription.height = height;
                        inscription.timestamp = timestamp;
                        inscription.sequence_number = seq_num;
                        seq_num += 1;

                        // Calculate sat position
                        let sat_offset = if let Some(ptr) = inscription.pointer {
                            ptr 
                        } else {
                            running_sats + output_offset
                        };

                        // Determine cursed status
                        inscription.cursed = input_index != 0 || sat_offset != 0;
                        if inscription.cursed {
                            inscription.number = -(cursed_count as i64);
                            cursed_count += 1;
                        } else {
                            inscription.number = blessed_count as i64; 
                            blessed_count += 1;
                        }
                        // Calculate fees
                        let fee = if input_value > 0 {
                            (input_value - output_value) / tx.input.len() as u64
                        } else {
                            0
                        };
                        inscription.fee = fee;

                        // Create inscription entry
                        let entry = InscriptionEntry {
                            id: format!("{}i{}", tx_id.as_hex(), input_index).into_bytes(),
                            number: inscription.number,
                            sequence_number: inscription.sequence_number,
                            timestamp: inscription.timestamp,
                            height: inscription.height,
                            fee: inscription.fee,
                            sat: if inscription.unbound { None } else { Some(sat_offset) },
                            parents: inscription.parent.map(|p| vec![p.to_vec()]).unwrap_or_default(),
                            children: Vec::new(),
                            cursed: inscription.cursed,
                            blessed: !inscription.cursed,
                            unbound: inscription.unbound,
                            charms: self.calculate_charms(&inscription, Some(sat_offset), 
                                OutPoint::new(tx.txid(), vout as u32),
                                &format!("{}:{}", tx_id.as_hex(), vout)),
                            media_type: inscription.media_type.clone(),
                            content_length: Some(inscription.content_bytes.len() as u64),
                            delegate: None,
                        };

                        // Update inscription mappings
                        inscriptions.inscription_id_to_inscription
                            .select(&entry.id)
                            .set(Arc::new(inscription.content_bytes.to_vec()));

                        if let Some(media_type) = inscription.media_type {
                            inscriptions.inscription_id_to_media_type
                                .select(&entry.id)
                                .set(media_type);
                        }

                        if let Some(metadata) = inscription.metadata {
                            inscriptions.inscription_id_to_metadata
                                .select(&entry.id)
                                .set(metadata);
                        }

                        // Update satpoint mappings
                        let sat_point = format!("{}:{}", tx_id.as_hex(), output_offset).into_bytes();
                        inscriptions.satpoint_to_inscription_id
                            .select(&sat_point)
                            .set(Arc::new(entry.id.clone()));

                        inscriptions.inscription_id_to_satpoint
                            .select(&entry.id)
                            .set(Arc::new(sat_point));

                        // Update sequence number mappings
                        inscriptions.sequence_number_to_inscription_id
                            .select_value(entry.sequence_number)
                            .set(Arc::new(entry.id.clone()));

                        inscriptions.inscription_id_to_sequence_number
                            .select(&entry.id)
                            .set_value(entry.sequence_number);

                        // Update parent-child relationships
                        if let Some(parent_id) = inscription.parent {
                            if let Some(parent_seq) = to_option_value::<IndexPointer, u64>(inscriptions.inscription_id_to_sequence_number
                                .select(&parent_id)) {
                                    inscriptions.sequence_number_to_children
                                        .select_value(parent_seq)
                                        .append(Arc::new(entry.id.clone()));
                            }
                        }

                        // Update height mappings
                        inscriptions.height_to_inscription_ids
                            .select_value(height)
                            .append(Arc::new(entry.id.clone()));

                        // Store inscription entry
                        inscriptions.inscription_entries
                            .select(&entry.id)
                            .set(Arc::new(bincode::serialize(&entry).unwrap()));

                        // Track blessed/cursed numbers
                        if inscription.cursed {
                            inscriptions.cursed_inscription_numbers
                                .select_value(-entry.number as u64)
                                .set(Arc::new(entry.id.clone()));
                        } else {
                            inscriptions.blessed_inscription_numbers
                                .select_value(entry.number as u64)
                                .set(Arc::new(entry.id.clone()));
                        }

                        inscription_updates.push(entry);
                    }
                }
                output_offset += output.value.to_sat();
            }
            running_sats += input_value;
        }

        // Update counters
        inscriptions.cursed_count.set_value(cursed_count);
        inscriptions.blessed_count.set_value(blessed_count);
        inscriptions.next_sequence_number.set_value(seq_num);

        Ok(())
    }
    fn index_sat_ranges(
        &self,
        tx: &Transaction,
        starting_sat: u64,
        reward: u64,
    ) -> Result<()> {
        let mut sink = SatSink::new(tx);
        let inscriptions = INSCRIPTIONS.read().unwrap();
        let source = if reward > 0 {
            // Coinbase source
            SatSource::new(starting_sat, reward)
        } else {
            // Regular tx source from inputs
            SatSource::from_inputs(tx, &inscriptions)? 
        };

        sink.consume(source, &inscriptions)?;
        Ok(())
    }

    fn index_transaction_values(
        &self,
        tx: &Transaction,
    ) -> Result<()> {
        let mut inscriptions = INSCRIPTIONS.write().unwrap();
        for (idx, output) in tx.output.iter().enumerate() {
            let outpoint = OutPoint::new(tx.compute_txid(), idx as u32);
            inscriptions.outpoint_to_value
                .select(&consensus_encode(&outpoint)?)
                .set_value(output.value.to_sat());
        }
        Ok(())
    }

    fn extract_inscription(&self, input: &bitcoin::TxIn) -> Option<Inscription> {
        // Extract envelope from witness data
        if input.witness.len() < 2 {
            return None;
        }

        let mut content_type: Option<Arc<Vec<u8>>> = None;
        let mut content: Option<Vec<u8>> = None;
        let mut parent: Option<Arc<Vec<u8>>> = None;
        let mut metadata: Option<Arc<Vec<u8>>> = None;
        let mut cursed = false;
        let mut unbound = false;

        // Parse witness stack
        let mut stack = input.witness.iter();
        
        while let Some(element) = stack.next() {
            if element == b"ord" {
                // Found inscription marker
                
                // Parse content type
                if let Some(ct) = stack.next() {
                    content_type = Some(Arc::new(ct.to_vec()));
                }

                // Parse content
                if let Some(data) = stack.next() {
                    content = Some(data.to_vec());
                }

                // Parse optional parent
                if let Some(p) = stack.next() {
                    if p.starts_with(b"parent=") {
                        parent = Some(Arc::new(p[7..].to_vec()));
                    }
                }

                // Parse optional metadata  
                if let Some(m) = stack.next() {
                    if m.starts_with(b"metadata=") {
                        metadata = Some(Arc::new(m[9..].to_vec()));
                    }
                }

                break;
            }
        }

        // Require content to be present
        let content = content?;

        // Create inscription
        Some(Inscription {
            media_type: content_type,
            content_bytes: Arc::new(content),
            parent,
            metadata,
            number: 0, // Will be set later based on blessed/cursed status
            sequence_number: 0, // Will be set during indexing
            cursed,
            unbound
        })
    }
}

struct SatSink<'a> {
    tx: &'a Transaction,
    pointer: usize,
    offset: u64,
}

impl<'a> SatSink<'a> {
    pub fn new(tx: &'a Transaction) -> Self {
        Self {
            tx,
            pointer: 0,
            offset: 0,
        }
    }

    pub fn filled(&self) -> bool {
        self.pointer >= self.tx.output.len() || 
        (self.pointer == self.tx.output.len() - 1 && 
         self.offset >= self.tx.output[self.tx.output.len() - 1].value.to_sat())
    }

    pub fn current_outpoint(&self) -> OutPoint {
        OutPoint::new(self.tx.txid(), self.pointer as u32)
    }

    pub fn consume(&mut self, mut source: SatSource, inscriptions: &InscriptionTable) -> Result<()> {
        let mut inscriptions = INSCRIPTIONS.write().unwrap();
        
        while !source.consumed() && !self.filled() {
            let source_remaining = source.ranges.distances[source.pointer] - source.offset;
            let target_remaining = self.tx.output[self.pointer].value.to_sat() - self.offset;
            
            let outpoint = self.current_outpoint();
            let sat = source.ranges.sats[source.pointer] + source.offset;
            
            // Update sat mappings
            let serialized = consensus_encode(&outpoint)?;
            inscriptions.outpoint_to_sat.select(&serialized).append_value::<u64>(sat);
            inscriptions.sat_to_outpoint.set_value(sat, Arc::new(serialized));

            if target_remaining < source_remaining {
                self.pointer += 1;
                self.offset = 0;
                source.offset += target_remaining;
            } else if source_remaining < target_remaining {
                source.pointer += 1;
                source.offset = 0;
                self.offset += source_remaining;
            } else {
                source.pointer += 1;
                source.offset = 0;
                self.pointer += 1;
                self.offset = 0;
            }
        }
        Ok(())
    }
}
pub struct SatSource {
    ranges: SatRanges,
    pointer: usize,
    offset: u64,
}

impl SatSource {
    pub fn new(start_sat: u64, distance: u64) -> Self {
        Self {
            ranges: SatRanges::new(vec![start_sat], vec![distance]),
            pointer: 0,
            offset: 0,
        }
    }

    pub fn from_inputs(tx: &Transaction, inscriptions: &InscriptionTable) -> Result<Self> {
        let mut sats = Vec::new();
        
        for input in &tx.input {
            let outpoint_sats = inscriptions.outpoint_to_sat
                .select(&consensus_encode(&input.previous_output)?)
                .get_list_values::<u64>();
            sats.extend(outpoint_sats);
        }

        let inscriptions = INSCRIPTIONS.read().unwrap();
        Ok(Self {
            ranges: SatRanges::from_sats(sats, inscriptions.starting_sat.get_value::<u64>()),
            pointer: 0,
            offset: 0,
        })
    }

    pub fn consumed(&self) -> bool {
        self.pointer >= self.ranges.sats.len() ||
        (self.pointer == self.ranges.sats.len() - 1 && 
         self.offset >= self.ranges.distances[self.ranges.distances.len() - 1])
    }
}

pub struct SatRanges {
    sats: Vec<u64>,
    distances: Vec<u64>,
}

impl SatRanges {
    pub fn new(sats: Vec<u64>, distances: Vec<u64>) -> Self {
        Self { sats, distances }
    }

    pub fn from_sats(sats: Vec<u64>, range_end: u64) -> Self {
        let distances = sats.iter()
            .map(|sat| {
                range_length(&mut INSCRIPTIONS.write().unwrap().sat_to_outpoint, *sat, range_end)
            })
            .collect();
        Self::new(sats, distances)
    }
}

pub fn range_length(bst: &BST<impl KeyValuePointer>, key: u64, max: u64) -> u64 {
    let inscriptions = INSCRIPTIONS.read().unwrap();
    if let Some(greater) = bst.seek_greater(&key.to_be_bytes()) {
        let greater_val = u64::from_be_bytes(greater.try_into().unwrap());
        if greater_val > max {
            max - key
        } else {
            greater_val - key
        }
    } else {
        max - key
    }
}

pub fn block_reward(height: u32) -> u64 {
    50_0000_0000 >> (height / 210_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[wasm_bindgen_test]
    pub fn test_block_reward() {
        assert_eq!(block_reward(0), 50_0000_0000);
        assert_eq!(block_reward(210_000), 25_0000_0000);
        assert_eq!(block_reward(420_000), 12_5000_0000);
    }
}
