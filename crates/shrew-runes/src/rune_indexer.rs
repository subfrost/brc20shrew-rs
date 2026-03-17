use crate::balance_sheet::{BalanceSheet, RuneId};
use crate::tables::*;
use bitcoin::{Block, Transaction, OutPoint};
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use ordinals::{Artifact, Runestone};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Metadata for a deployed rune
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneEntry {
    pub id: RuneId,
    pub name: String,
    pub spaced_name: String,
    pub divisibility: u8,
    pub symbol: Option<char>,
    pub spacers: u32,
    pub premine: u128,
    pub terms: Option<RuneTerms>,
    pub turbo: bool,
    pub mints: u128,
    pub supply: u128,
    pub etching_height: u32,
    pub etching_txid: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneTerms {
    pub amount: Option<u128>,
    pub cap: Option<u128>,
    pub height_start: Option<u64>,
    pub height_end: Option<u64>,
    pub offset_start: Option<u64>,
    pub offset_end: Option<u64>,
}

/// Event types matching OPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneEvent {
    pub event_type: u32, // 0=input, 1=new-allocation, 2=mint, 3=output, 4=burn
    pub rune_id: RuneId,
    pub amount: u128,
    pub txid: [u8; 32],
    pub vout: u32,
    pub block_height: u32,
}

/// Convert ordinals::RuneId to our RuneId
fn from_ordinals_rune_id(id: ordinals::RuneId) -> RuneId {
    RuneId::new(id.block, id.tx)
}

pub struct RuneIndexer {
    height: u32,
}

impl RuneIndexer {
    pub fn new() -> Self {
        Self { height: 0 }
    }

    pub fn index_block(&mut self, block: &Block, height: u32) {
        self.height = height;

        // Runes only activate at height 840000
        if height < shrew_support::constants::RUNES_ACTIVATION_HEIGHT {
            return;
        }

        let mut events = Vec::new();

        for (tx_index, tx) in block.txdata.iter().enumerate() {
            let tx_events = self.index_transaction(tx, tx_index as u32, height);
            events.extend(tx_events);
        }

        // Store events for this height
        if !events.is_empty() {
            let events_bytes = bincode::serialize(&events).unwrap_or_default();
            HEIGHT_TO_RUNE_EVENTS.select(&height.to_le_bytes().to_vec()).set(Arc::new(events_bytes));
        }
    }

    fn index_transaction(&self, tx: &Transaction, tx_index: u32, height: u32) -> Vec<RuneEvent> {
        let mut events = Vec::new();
        let txid = *tx.compute_txid().as_byte_array();

        // Decipher the runestone from the transaction
        let artifact = Runestone::decipher(tx);

        let artifact = match artifact {
            Some(artifact) => artifact,
            None => return events,
        };

        match artifact {
            Artifact::Cenotaph(cenotaph) => {
                // Cenotaph: all input runes are burned
                let input_sheet = self.collect_input_runes(tx);
                for (rune_id, amount) in &input_sheet.balances {
                    events.push(RuneEvent {
                        event_type: 4, // burn
                        rune_id: *rune_id,
                        amount: *amount,
                        txid,
                        vout: 0,
                        block_height: height,
                    });
                }
                // Process etching name even in cenotaphs (marks rune as existing)
                if let Some(rune) = cenotaph.etching {
                    self.process_etching_name(rune, tx, tx_index, height);
                }
            }
            Artifact::Runestone(runestone) => {
                // Collect input runes
                let input_sheet = self.collect_input_runes(tx);
                for (rune_id, amount) in &input_sheet.balances {
                    events.push(RuneEvent {
                        event_type: 0, // input
                        rune_id: *rune_id,
                        amount: *amount,
                        txid,
                        vout: 0,
                        block_height: height,
                    });
                }

                let mut unallocated = input_sheet;

                // Process etching
                if let Some(etching) = runestone.etching {
                    let new_rune_id = RuneId::new(height as u64, tx_index);
                    let entry = self.process_etching(etching, &runestone, new_rune_id, tx, tx_index, height);

                    // Credit premine to unallocated
                    if entry.premine > 0 {
                        unallocated.credit(new_rune_id, entry.premine);
                        events.push(RuneEvent {
                            event_type: 1, // new-allocation (premine)
                            rune_id: new_rune_id,
                            amount: entry.premine,
                            txid,
                            vout: 0,
                            block_height: height,
                        });
                    }
                }

                // Process mints
                if let Some(mint_id) = runestone.mint {
                    let rune_id = from_ordinals_rune_id(mint_id);
                    if let Some(mint_amount) = self.try_mint(rune_id, height) {
                        unallocated.credit(rune_id, mint_amount);
                        events.push(RuneEvent {
                            event_type: 2, // mint
                            rune_id,
                            amount: mint_amount,
                            txid,
                            vout: 0,
                            block_height: height,
                        });
                    }
                }

                // Process edicts (transfers between outputs)
                let mut output_sheets: HashMap<u32, BalanceSheet> = HashMap::new();
                for edict in &runestone.edicts {
                    let rune_id = from_ordinals_rune_id(edict.id);
                    let amount = edict.amount;
                    let output = edict.output;

                    if output as usize >= tx.output.len() + 1 {
                        continue; // Invalid output index
                    }

                    let transfer_amount = if amount == 0 {
                        // Transfer all remaining
                        unallocated.get(&rune_id)
                    } else {
                        amount.min(unallocated.get(&rune_id))
                    };

                    if transfer_amount > 0 {
                        unallocated.debit(rune_id, transfer_amount);
                        if output as usize == tx.output.len() {
                            // Split equally among all non-OP_RETURN outputs
                            let valid_outputs: Vec<u32> = (0..tx.output.len() as u32)
                                .filter(|&i| !tx.output[i as usize].script_pubkey.is_op_return())
                                .collect();
                            if !valid_outputs.is_empty() {
                                let per_output = transfer_amount / valid_outputs.len() as u128;
                                let remainder = transfer_amount % valid_outputs.len() as u128;
                                for (idx, &vout) in valid_outputs.iter().enumerate() {
                                    let amt = per_output + if idx == 0 { remainder } else { 0 };
                                    output_sheets.entry(vout).or_default().credit(rune_id, amt);
                                }
                            }
                        } else {
                            output_sheets.entry(output).or_default().credit(rune_id, transfer_amount);
                        }
                    }
                }

                // Assign remaining unallocated runes to pointer output (or first non-OP_RETURN)
                let default_output = runestone.pointer.unwrap_or(0);
                if !unallocated.is_empty() {
                    for (rune_id, amount) in &unallocated.balances {
                        if *amount > 0 {
                            output_sheets.entry(default_output).or_default().credit(*rune_id, *amount);
                        }
                    }
                }

                // Store output balance sheets and emit events
                for (vout, sheet) in &output_sheets {
                    if sheet.is_empty() { continue; }
                    let outpoint = OutPoint { txid: tx.compute_txid(), vout: *vout };
                    let outpoint_bytes: Vec<u8> = outpoint.txid.as_byte_array().iter()
                        .chain(outpoint.vout.to_le_bytes().iter()).copied().collect();
                    RUNE_BALANCES_BY_OUTPOINT.select(&outpoint_bytes).set(Arc::new(sheet.to_bytes()));

                    for (rune_id, amount) in &sheet.balances {
                        events.push(RuneEvent {
                            event_type: 3, // output
                            rune_id: *rune_id,
                            amount: *amount,
                            txid,
                            vout: *vout,
                            block_height: height,
                        });
                    }
                }
            }
        }

        events
    }

    fn collect_input_runes(&self, tx: &Transaction) -> BalanceSheet {
        let mut sheet = BalanceSheet::new();
        for input in &tx.input {
            let outpoint_bytes: Vec<u8> = input.previous_output.txid.as_byte_array().iter()
                .chain(input.previous_output.vout.to_le_bytes().iter()).copied().collect();
            let data = RUNE_BALANCES_BY_OUTPOINT.select(&outpoint_bytes).get();
            if !data.is_empty() {
                if let Some(input_sheet) = BalanceSheet::from_bytes(&data) {
                    sheet.merge(&input_sheet);
                }
            }
        }
        sheet
    }

    fn process_etching(
        &self,
        etching: ordinals::Etching,
        runestone: &ordinals::Runestone,
        rune_id: RuneId,
        tx: &Transaction,
        _tx_index: u32,
        height: u32,
    ) -> RuneEntry {
        let name = etching.rune.map(|r| r.to_string()).unwrap_or_default();
        let spaced_name = etching.rune.map(|r| {
            let spacers = etching.spacers.unwrap_or(0);
            let mut s = String::new();
            for (i, c) in r.to_string().chars().enumerate() {
                if i > 0 && (spacers >> (i - 1)) & 1 == 1 {
                    s.push('.');
                }
                s.push(c);
            }
            s
        }).unwrap_or_default();

        let terms = etching.terms.map(|t| RuneTerms {
            amount: t.amount,
            cap: t.cap,
            height_start: t.height.0,
            height_end: t.height.1,
            offset_start: t.offset.0,
            offset_end: t.offset.1,
        });

        let premine = etching.premine.unwrap_or(0);

        let entry = RuneEntry {
            id: rune_id,
            name: name.clone(),
            spaced_name,
            divisibility: etching.divisibility.unwrap_or(0),
            symbol: etching.symbol,
            spacers: etching.spacers.unwrap_or(0),
            premine,
            terms: terms.clone(),
            turbo: runestone.etching.as_ref().map(|e| e.turbo).unwrap_or(false),
            mints: 0,
            supply: premine,
            etching_height: height,
            etching_txid: *tx.compute_txid().as_byte_array(),
        };

        // Store the rune entry
        let entry_bytes = bincode::serialize(&entry).unwrap_or_default();
        RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).set(Arc::new(entry_bytes));

        // Store name -> id mapping
        RUNE_NAME_TO_ID.select(&name.to_uppercase().as_bytes().to_vec()).set(Arc::new(rune_id.to_bytes()));

        // Store etching -> rune id mapping
        let etching_bytes = tx.compute_txid().as_byte_array().to_vec();
        ETCHING_TO_RUNE_ID.select(&etching_bytes).set(Arc::new(rune_id.to_bytes()));
        RUNE_ID_TO_ETCHING.select(&rune_id.to_bytes()).set(Arc::new(etching_bytes));

        // Store mint terms if applicable
        if let Some(ref t) = terms {
            if let Some(cap) = t.cap {
                RUNE_CAP.select(&rune_id.to_bytes()).set(Arc::new(cap.to_le_bytes().to_vec()));
                RUNE_MINTS_REMAINING.select(&rune_id.to_bytes()).set(Arc::new(cap.to_le_bytes().to_vec()));
            }
        }

        entry
    }

    fn process_etching_name(&self, rune: ordinals::Rune, _tx: &Transaction, tx_index: u32, height: u32) {
        let rune_id = RuneId::new(height as u64, tx_index);
        let name = rune.to_string();
        RUNE_NAME_TO_ID.select(&name.to_uppercase().as_bytes().to_vec()).set(Arc::new(rune_id.to_bytes()));
    }

    fn try_mint(&self, rune_id: RuneId, height: u32) -> Option<u128> {
        // Load rune entry
        let entry_bytes = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
        if entry_bytes.is_empty() { return None; }
        let mut entry: RuneEntry = bincode::deserialize(&entry_bytes).ok()?;

        let terms = entry.terms.as_ref()?;
        let mint_amount = terms.amount?;

        // Check height bounds
        if let Some(start) = terms.height_start {
            if (height as u64) < start { return None; }
        }
        if let Some(end) = terms.height_end {
            if (height as u64) >= end { return None; }
        }

        // Check offset bounds
        if let Some(start) = terms.offset_start {
            if (height as u64) < entry.etching_height as u64 + start { return None; }
        }
        if let Some(end) = terms.offset_end {
            if (height as u64) >= entry.etching_height as u64 + end { return None; }
        }

        // Check remaining supply
        let remaining_bytes = RUNE_MINTS_REMAINING.select(&rune_id.to_bytes()).get();
        if !remaining_bytes.is_empty() {
            let remaining = u128::from_le_bytes(remaining_bytes[..16].try_into().ok()?);
            if remaining == 0 { return None; }
            RUNE_MINTS_REMAINING.select(&rune_id.to_bytes()).set(
                Arc::new((remaining - 1).to_le_bytes().to_vec())
            );
        }

        // Update entry
        entry.mints += 1;
        entry.supply += mint_amount;
        let updated_bytes = bincode::serialize(&entry).unwrap_or_default();
        RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).set(Arc::new(updated_bytes));

        Some(mint_amount)
    }
}
