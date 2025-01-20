use bitcoin::block::Block;  
use bitcoin::transaction::Transaction;
use bitcoin::hash_types::BlockHash;
use bitcoin::OutPoint;
use bitcoin::{Txid};
use metashrew_support::utils::{consensus_encode};
use bitcoin::hashes::{Hash};
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::Arc;
use std::ptr::addr_of_mut;

pub mod bst;
pub mod tables;

use crate::bst::BST;
use crate::tables::{INSCRIPTIONS, InscriptionTable};

pub struct Inscription {
    pub media_type: Option<Arc<Vec<u8>>>,
    pub content_bytes: Arc<Vec<u8>>,
    pub parent: Option<Arc<Vec<u8>>>,
}

pub struct Index(());

impl Index {
    pub fn new() -> Self {
        Self(())
    }

    fn index_transaction_inscriptions(
        &self,
        tx: &Transaction, 
        height: u32,
        tx_id: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut inscriptions = INSCRIPTIONS.write().unwrap();
        let mut offset: u64 = 0;
        let mut output_index: u32 = 0;

        for (_input_index, input) in tx.input.iter().enumerate() {
            if output_index as usize >= tx.output.len() {
                break;
            }
            
            // Get inscription from input if exists
            if let Some(inscription) = self.extract_inscription(input) {
                let sequence_num = inscriptions.next_sequence_number.get_value::<u64>() + 1;
                let outpoint = OutPoint::new(Txid::from_byte_array(<Vec<u8> as AsRef<[u8]>>::as_ref(&tx_id).try_into()?), output_index); 
                let sat_point = format!("{}:{}", outpoint, offset);

                // Get sat position
                let _sat = inscriptions.outpoint_to_sat
                    .select(&consensus_encode(&outpoint)?)
                    .select_index(0)
                    .get_value::<u64>();

                // Update inscription mappings
                inscriptions.inscription_id_to_inscription
                    .select(&format!("{}:{}", sat_point.clone(), 0).into_bytes())
                    .set(Arc::new(inscription.content_bytes.to_vec()));
                
                inscriptions.satpoint_to_inscription_id
                    .select(&sat_point.clone().into_bytes())
                    .set(Arc::new(format!("{}:{}", sat_point.clone(), 0).into_bytes()));

                inscriptions.inscription_id_to_satpoint
                    .select(&format!("{}:{}", sat_point, 0).into_bytes())
                    .set(Arc::new(sat_point.clone().into_bytes()));

                inscriptions.inscription_id_to_blockheight
                    .select(&format!("{}:{}", sat_point.clone(), 0).into_bytes())
                    .set_value(height);

                inscriptions.height_to_inscription_ids
                    .select_value(height)
                    .append(Arc::new(format!("{}:{}", sat_point, 0).into_bytes()));

                inscriptions.sequence_number_to_inscription_id
                    .select_value::<u64>(sequence_num)
                    .set(Arc::new(format!("{}:{}", sat_point, 0).into_bytes()));

                inscriptions.inscription_id_to_sequence_number
                    .select(&format!("{}:{}", sat_point, 0).into_bytes())
                    .set_value::<u64>(sequence_num);

                // Update next sequence number
                inscriptions.next_sequence_number.set_value::<u64>(sequence_num);
            }

            // Track sat movement from input
            let value = inscriptions.outpoint_to_value
                .select(&consensus_encode(&input.previous_output)?)
                .get_value::<u64>();

            offset += value;
            if offset >= tx.output[output_index as usize].value.to_sat() {
                output_index += 1;
                offset = 0;
            }
        }
        Ok(())
    }

    pub fn index_block(
        &self,
        block: &Block,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut inscriptions = INSCRIPTIONS.write().unwrap();
        
        // Index block header info
        inscriptions.height_to_blockhash
            .select_value(height)
            .set(Arc::new(block.block_hash().as_byte_array().to_vec()));

        inscriptions.blockhash_to_height
            .select(&block.block_hash().as_byte_array().to_vec())
            .set_value(height);

        // Index transactions
        let mut starting_sat = inscriptions.starting_sat.get_value::<u64>();
        let reward = block_reward(height);
        inscriptions.starting_sat.set_value(starting_sat + reward);

        // Index coinbase transaction 
        self.index_sat_ranges(&block.txdata[0], starting_sat, reward)?;

        // Index remaining transactions
        for tx in block.txdata.iter().skip(1) {
            let tx_id = tx.compute_txid().as_byte_array().to_vec();
            
            // Index inscription data
            self.index_transaction_inscriptions(tx, height, tx_id.clone())?;
            
            // Index sat ranges and values
            self.index_transaction_values(tx)?;
            self.index_sat_ranges(tx, starting_sat, 0)?;
        }

        Ok(())
    }

    fn index_sat_ranges(
        &self,
        tx: &Transaction,
        starting_sat: u64,
        reward: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        // TODO: Implement inscription extraction from input witness
        None
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

    pub fn consume(&mut self, mut source: SatSource, inscriptions: &InscriptionTable) -> Result<(), Box<dyn std::error::Error>> {
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

    pub fn from_inputs(tx: &Transaction, inscriptions: &InscriptionTable) -> Result<Self, Box<dyn std::error::Error>> {
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
