use anyhow::Result;
use bitcoin::block::Block;
use bitcoin::hashes::Hash;
use bitcoin::transaction::Transaction;
use bitcoin::OutPoint;
use bitcoin::Txid;
use metashrew::{flush, input};
use metashrew_support::byte_view::ByteView;
use metashrew_support::index_pointer::KeyValuePointer;
use metashrew_support::utils::consensus_encode;
use serde::{Deserialize, Serialize};
use ordinals::{Height};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct Flotsam {
    inscription_id: Vec<u8>,
    sequence_number: u64,
    offset: u64,
    origin: Origin,
}

#[derive(Debug, Clone)]
enum Origin {
    New {
        cursed: bool,
        fee: u64,
        hidden: bool,
        parents: Vec<Vec<u8>>,
        reinscription: bool,
        unbound: bool,
        vindicated: bool,
    },
    Old {
        sequence_number: u32,
        old_satpoint: String,
    },
}


#[derive(Debug)]
pub enum InscriptionError {
    InvalidInput(String),
    DatabaseError(String),
    TransactionError(String),
    EncodingError(String),
    ValidationError(String),
    DuplicateField(String),
    IncompleteField(String),
    UnrecognizedEvenField(String),
    NotAtOffsetZero(String),
    NotInFirstInput(String),
}

fn value_to_option<T: ByteView + std::cmp::PartialEq>(v: T) -> Option<T> {
    if T::zero() == v {
        None
    } else {
        Some(v)
    }
}

fn to_option_value<T: KeyValuePointer, U: ByteView>(v: T) -> Option<U> {
    if v.get().len() == 0 {
        None
    } else {
        Some(v.get_value::<U>())
    }
}

pub trait Lengthable {
    fn length(&self) -> usize;
}

impl Lengthable for Arc<Vec<u8>> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl Lengthable for Vec<u8> {
    fn length(&self) -> usize {
        self.len()
    }
}

fn to_option<T: Lengthable>(v: T) -> Option<T> {
    if v.length() == 0 {
        None
    } else {
        Some(v)
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
            Self::DuplicateField(msg) => write!(f, "Duplicate field: {}", msg),
            Self::IncompleteField(msg) => write!(f, "Incomplete field: {}", msg),
            Self::UnrecognizedEvenField(msg) => write!(f, "Unrecognized even field: {}", msg),
            Self::NotAtOffsetZero(msg) => write!(f, "Not at offset zero: {}", msg),
            Self::NotInFirstInput(msg) => write!(f, "Not in first input: {}", msg),
        }
    }
}

impl Error for InscriptionError {}

pub mod bst;
pub mod tables;

use crate::bst::BST;
use crate::tables::{InscriptionTable, INSCRIPTIONS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inscription {
    pub media_type: Option<Vec<u8>>,
    pub content_bytes: Vec<u8>,
    pub parent: Option<Vec<u8>>,
    pub metadata: Option<Vec<u8>>,
    pub number: i64,
    pub sequence_number: u64,
    pub fee: u64,
    pub height: u32,
    pub timestamp: u32,
    pub cursed: bool,
    pub unbound: bool,
    pub pointer: Option<u64>,
    pub hidden: bool,
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
    pub charms: u32,                 // Add charms bitfield
    pub media_type: Option<Vec<u8>>, // Add media type
    pub content_length: Option<u64>, // Add content length
    pub delegate: Option<Vec<u8>>,   // Add delegate support
}
#[derive(Debug, Copy, Clone)]
pub enum Charm {
    Cursed = 0,
    Reinscription = 1,
    Unbound = 2,
    Lost = 3,
    Burned = 4,
    Vindicated = 5,
    Hidden = 6,
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
        if Charm::Cursed.is_set(charms) {
            result.push("cursed");
        }
        if Charm::Reinscription.is_set(charms) {
            result.push("reinscription");
        }
        if Charm::Unbound.is_set(charms) {
            result.push("unbound");
        }
        if Charm::Lost.is_set(charms) {
            result.push("lost");
        }
        if Charm::Burned.is_set(charms) {
            result.push("burned");
        }
        if Charm::Vindicated.is_set(charms) {
            result.push("vindicated");
        }
        result
    }
}

#[derive(Debug, Default, Clone)]
pub struct Index {
    tables: InscriptionTable,
    pub blessed_inscription_count: u64,
    pub cursed_inscription_count: u64,
    pub height: u32,
    pub timestamp: u32,
    pub next_sequence_number: u64,
    pub lost_sats: u64,
    pub reward: u64,
}

impl Index {
    pub fn new() -> Self {
        Self {
            tables: InscriptionTable::new(),
            blessed_inscription_count: 0,
            cursed_inscription_count: 0,
            height: 0,
            timestamp: 0,
            next_sequence_number: 0,
            lost_sats: 0,
            reward: 0,
        }
    }

    fn validate_inscription(&self, inscription: &mut Inscription, input_index: usize, input_offset: u64) -> Result<()> {
        // Validate content type if present
        if let Some(ref media_type) = inscription.media_type {
            if media_type.len() > 255 {
                return Err(InscriptionError::ValidationError("Media type too long".to_string()).into());
            }
        }

        // Validate content length
        if inscription.content_bytes.len() > 1024 * 1024 {
            // 1MB limit
            return Err(InscriptionError::ValidationError("Content too large".to_string()).into());
        }

        // Apply curses
        if input_index != 0 {
            inscription.cursed = true;
            return Err(InscriptionError::NotInFirstInput("Inscription not in first input".to_string()).into());
        }

        if input_offset != 0 {
            inscription.cursed = true;
            return Err(InscriptionError::NotAtOffsetZero("Inscription not at offset zero".to_string()).into());
        }

        if inscription.pointer.is_some() {
            inscription.cursed = true;
        }

        Ok(())
    }

    fn get_transaction(&self, txid: Txid) -> Result<Transaction> {
        let inscriptions = INSCRIPTIONS.read().map_err(|e| {
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e))
        })?;

        let tx_bytes = inscriptions
            .transaction_id_to_transaction
            .select(&txid.as_byte_array().to_vec())
            .get();

        if tx_bytes.is_empty() {
            return Err(InscriptionError::TransactionError(format!("Transaction not found: {}", txid)).into());
        }

        bitcoin::consensus::deserialize(&tx_bytes).map_err(|e| {
            InscriptionError::EncodingError(format!("Failed to deserialize transaction: {}", e))
                .into()
        })
    }

    pub fn index_block(&mut self, block: &Block) -> Result<()> {
        let height = self.height;
        let timestamp = block.header.time as u32;
        
        self.height = height + 1;
        self.timestamp = timestamp;

        for tx in block.txdata.iter() {
            self.index_transaction(tx, height, timestamp)?;
        }

        Ok(())
    }

    fn index_transaction(&mut self, tx: &Transaction, height: u32, timestamp: u32) -> Result<()> {
        // Index transaction outputs first
        self.index_transaction_values(tx)?;

        let is_coinbase = tx.input.first().map_or(false, |input| input.previous_output.is_null());
        if is_coinbase {
            self.reward = tx.output.iter().map(|o| o.value.to_sat()).sum::<u64>();
        }

        let mut flotsam = Vec::new();
        let mut total_input_value = 0u64;
        let total_output_value = tx.output.iter().map(|o| o.value.to_sat()).sum::<u64>();
        
        // First pass - collect inscriptions and track sat movement
        for (input_index, input) in tx.input.iter().enumerate() {
            if input.previous_output.is_null() {
                total_input_value += Height(height).subsidy();
                continue;
            }

            if let Some(mut inscription) = self.extract_inscription(input) {
                self.validate_inscription(&mut inscription, input_index, total_input_value)?;

                let value = self.get_input_value(input)?;
                total_input_value += value;

                flotsam.push(self.create_flotsam(
                    inscription,
                    total_input_value,
                    height,
                    timestamp,
                    total_output_value,
                    value == 0,
                ));
            }
        }

        // Update inscription locations and track lost sats
        if is_coinbase {
            self.process_coinbase_transaction(tx, flotsam, total_input_value, total_output_value)?;
        } else {
            self.process_normal_transaction(tx, flotsam, total_input_value, total_output_value)?;
        }

        Ok(())
    }

    fn index_transaction_values(&self, tx: &Transaction) -> Result<()> {
        let inscriptions = INSCRIPTIONS.write().unwrap();
        for (idx, output) in tx.output.iter().enumerate() {
            let outpoint = OutPoint::new(tx.compute_txid(), idx as u32);
            inscriptions
                .outpoint_to_value
                .select(&consensus_encode(&outpoint)?)
                .set_value(output.value.to_sat());
        }
        Ok(())
    }

    fn get_input_value(&self, input: &bitcoin::TxIn) -> Result<u64> {
        let inscriptions = INSCRIPTIONS.read().map_err(|e| {
            InscriptionError::DatabaseError(format!("Failed to acquire lock: {}", e))
        })?;

        Ok(inscriptions
            .outpoint_to_value
            .select(&consensus_encode(&input.previous_output)?)
            .get_value::<u64>())
    }

    fn create_flotsam(
        &mut self,
        mut inscription: Inscription,
        total_input_value: u64,
        height: u32,
        timestamp: u32,
        total_output_value: u64,
        unbound: bool,
    ) -> Flotsam {
        let fee = if total_input_value > total_output_value {
            (total_input_value - total_output_value) / total_input_value
        } else {
            0
        };

        // Create a new sequence number for this inscription
        let sequence_number = self.next_sequence_number;

        // Calculate the inscription number
        let inscription_number = if inscription.cursed {
            self.cursed_inscription_count += 1;
            -(self.cursed_inscription_count as i64)
        } else {
            self.blessed_inscription_count += 1;
            self.blessed_inscription_count as i64
        };

        inscription.number = inscription_number;
        inscription.sequence_number = sequence_number as u64;
        inscription.fee = fee;
        inscription.height = height;
        inscription.timestamp = timestamp;
        inscription.unbound = unbound;

        Flotsam {
            inscription_id: format!("{}:{}", self.height, sequence_number).into_bytes(),
            offset: total_input_value,
            origin: Origin::New {
                cursed: inscription.cursed,
                fee,
                hidden: inscription.hidden,
                parents: inscription.parent.map(|p| vec![p]).unwrap_or_default(),
                reinscription: false, // Will be set later if needed
                unbound,
                vindicated: inscription.cursed && self.is_jubilee_height(height),
            },
            sequence_number
        }
    }

    fn process_coinbase_transaction(
        &mut self,
        tx: &Transaction,
        mut flotsam: Vec<Flotsam>,
        total_input_value: u64,
        total_output_value: u64,
    ) -> Result<()> {
        // Sort flotsam by offset for deterministic ordering
        flotsam.sort_by_key(|f| f.offset);

        let mut output_value = 0u64;
        let mut current_output = 0usize;
        
        // Process all flotsam
        for mut flotsum in flotsam {
            while current_output < tx.output.len() && 
                  flotsum.offset >= output_value + tx.output[current_output].value.to_sat() {
                output_value += tx.output[current_output].value.to_sat();
                current_output += 1;
            }

            if current_output < tx.output.len() {
                // The inscription landed in an output
                let outpoint = OutPoint::new(tx.compute_txid(), current_output as u32);
                let offset = flotsum.offset - output_value;
                
                self.update_inscription_location(&mut flotsum, outpoint, offset)?;
            } else {
                // The inscription was lost to fees
                self.lost_sats += total_input_value - total_output_value;
                let null_outpoint = OutPoint::null();
                let offset = self.lost_sats + flotsum.offset - output_value;
                
                self.update_inscription_location(&mut flotsum, null_outpoint, offset)?;
            }
        }

        Ok(())
    }

    fn process_normal_transaction(
        &mut self,
        tx: &Transaction,
        mut flotsam: Vec<Flotsam>,
        total_input_value: u64,
        total_output_value: u64,
    ) -> Result<()> {
        // Sort flotsam by offset for deterministic ordering
        flotsam.sort_by_key(|f| f.offset);

        let mut output_value = 0u64;
        let mut current_output = 0usize;
        
        // Process all flotsam
        for mut flotsum in flotsam {
            while current_output < tx.output.len() && 
                  flotsum.offset >= output_value + tx.output[current_output].value.to_sat() {
                output_value += tx.output[current_output].value.to_sat();
                current_output += 1;
            }

            if current_output < tx.output.len() {
                // The inscription landed in an output
                let outpoint = OutPoint::new(tx.compute_txid(), current_output as u32);
                let offset = flotsum.offset - output_value;
                
                self.update_inscription_location(&mut flotsum, outpoint, offset)?;
            }
        }

        self.reward += total_input_value - total_output_value;
        Ok(())
    }

    fn update_inscription_location(
        &mut self,
        flotsum: &mut Flotsam,
        outpoint: OutPoint,
        offset: u64,
    ) -> Result<()> {
        let mut inscriptions = INSCRIPTIONS.write().unwrap();
        let satpoint = format!("{}:{}", outpoint, offset);

        // Check for reinscription
        if inscriptions
            .satpoint_to_inscription_id
            .select(&satpoint.as_bytes().to_vec())
            .get()
            .len() > 0
        {
            if let Origin::New { ref mut reinscription, .. } = flotsum.origin {
                *reinscription = true;
            }
        }

        // Update inscription mappings
        inscriptions
            .satpoint_to_inscription_id
            .select(&satpoint.as_bytes().to_vec())
            .set(Arc::new(flotsum.inscription_id.clone()));

        inscriptions
            .inscription_id_to_satpoint
            .select(&flotsum.inscription_id)
            .set(Arc::new(satpoint.as_bytes().to_vec()));

        // Update sequence number mappings
        inscriptions
            .sequence_number_to_inscription_id
            .select_value(flotsum.sequence_number)
            .set(Arc::new(flotsum.inscription_id.clone()));

        inscriptions
            .inscription_id_to_sequence_number
            .select(&flotsum.inscription_id)
            .set_value(flotsum.sequence_number);

        if let Origin::New { cursed, .. } = flotsum.origin {
            if cursed {
                let cursed_num = self.cursed_inscription_count;
                inscriptions
                    .cursed_inscription_numbers
                    .select_value((-(cursed_num as i64) as u64))
                    .set(Arc::new(flotsum.inscription_id.clone()));
            } else {
                let blessed_num = self.blessed_inscription_count;
                inscriptions
                    .blessed_inscription_numbers
                    .select_value(blessed_num)
                    .set(Arc::new(flotsum.inscription_id.clone()));
            }
        }

        Ok(())
    }

    fn extract_inscription(&self, input: &bitcoin::TxIn) -> Option<Inscription> {
        if input.witness.len() < 2 {
            return None;
        }
        
        let mut inscriptions = Vec::new();
        
        for witness_item in input.witness.iter() {
            if witness_item.len() < 4 {
                continue;
            }

            // Check for inscription marker
            if witness_item[0..4] != [0x00, 0x63, 0x03, 0x6f] {
                continue;
            }

            let mut media_type = None;
            let mut content = None;
            let mut parent = None;
            let mut metadata = None;
            let mut pointer = None;
            let mut hidden = false;
            let mut cursed = false;
            
            // Parse inscription fields
            let mut i = 4;
            while i < witness_item.len() {
                if i + 2 > witness_item.len() {
                    cursed = true;
                    break;
                }

                let field_type = witness_item[i];
                let len = witness_item[i + 1] as usize;
                i += 2;

                if i + len > witness_item.len() {
                    cursed = true;
                    break;
                }

                let field_content = witness_item[i..i + len].to_vec();
                i += len;

                match field_type {
                    0x01 => {
                        if media_type.is_some() {
                            cursed = true;
                            break;
                        }
                        media_type = Some(field_content);
                    }
                    0x02 => {
                        if content.is_some() {
                            cursed = true;
                            break;
                        }
                        content = Some(field_content);
                    }
                    0x03 => {
                        if parent.is_some() {
                            cursed = true;
                            break;
                        }
                        parent = Some(field_content);
                    }
                    0x04 => {
                        if metadata.is_some() {
                            cursed = true;
                            break;
                        }
                        metadata = Some(field_content);
                    }
                    0x05 => {
                        if pointer.is_some() {
                            cursed = true;
                            break;
                        }
                        if field_content.len() == 8 {
                            pointer = Some(u64::from_le_bytes(field_content.try_into().unwrap()));
                        }
                    }
                    0x06 => {
                        hidden = true;
                    }
                    _ if field_type % 2 == 0 => {
                        cursed = true;
                        break;
                    }
                    _ => continue,
                }
            }

            if let Some(content) = content {
                inscriptions.push(Inscription {
                    media_type,
                    content_bytes: content,
                    parent,
                    metadata,
                    number: 0, // Will be set later
                    sequence_number: 0, // Will be set later
                    fee: 0, // Will be set later
                    height: 0, // Will be set later
                    timestamp: 0, // Will be set later
                    cursed,
                    unbound: false, // Will be set later
                    pointer,
                    hidden,
                });
            }
        }

        inscriptions.into_iter().next()
    }

    fn is_jubilee_height(&self, height: u32) -> bool {
        // Implementation depends on the specific jubilee height configured
        height >= 1_050_000 // Mainnet jubilee height
    }
}


