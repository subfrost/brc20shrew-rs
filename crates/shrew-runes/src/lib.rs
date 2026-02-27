#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod rune_indexer;
pub mod balance_sheet;
pub mod tables;
pub mod view;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/shrew_runes.rs"));
}

pub use rune_indexer::RuneIndexer;

#[cfg(feature = "entrypoint")]
#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        // First run inscription indexer for block metadata
        let mut inscription_indexer = shrew_ord::indexer::InscriptionIndexer::new();
        let _ = inscription_indexer.load_state();
        let _ = inscription_indexer.index_block(&block, height);

        // Then run rune indexer
        let mut rune_indexer = RuneIndexer::new();
        rune_indexer.index_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn getrune(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetRuneRequest = from_slice(input)?;
    Ok(to_vec(&view::get_rune(&req)?)?)
}

#[metashrew_core::view]
pub fn getrunebalance(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetRuneBalanceRequest = from_slice(input)?;
    Ok(to_vec(&view::get_rune_balance(&req)?)?)
}

#[metashrew_core::view]
pub fn getruneevents(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetRuneEventsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_rune_events(&req)?)?)
}

#[cfg(test)]
mod tests;
