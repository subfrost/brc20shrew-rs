#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod brc20;
pub mod tables;
pub mod view;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/shrew_brc20.rs"));
}

pub use brc20::{Brc20Indexer, Brc20Operation, Ticker, Balance, TransferInfo};


#[cfg(feature = "entrypoint")]
#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        // First run inscription indexer
        let mut inscription_indexer = shrew_ord::indexer::InscriptionIndexer::new();
        let _ = inscription_indexer.load_state();
        let _ = inscription_indexer.index_block(&block, height);

        // Then process BRC20 operations from inscribed content
        let brc20_indexer = Brc20Indexer::new();
        brc20_indexer.process_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn getbalance(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBalanceRequest = from_slice(input)?;
    Ok(to_vec(&view::get_balance(&req)?)?)
}

#[metashrew_core::view]
pub fn getbrc20events(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBrc20EventsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_brc20_events(&req)?)?)
}

#[cfg(test)]
mod tests;
