#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod prog_indexer;
pub mod controller;
pub mod view;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/shrew_brc20_prog.rs"));
}

pub use prog_indexer::ProgrammableBrc20Indexer;

#[cfg(feature = "entrypoint")]
#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        // Run inscription indexer
        let mut inscription_indexer = shrew_ord::indexer::InscriptionIndexer::new();
        let _ = inscription_indexer.load_state();
        let _ = inscription_indexer.index_block(&block, height);

        // Run BRC20 indexer
        let brc20_indexer = shrew_brc20::Brc20Indexer::new();
        brc20_indexer.process_block(&block, height);

        // Run programmable BRC20 indexer
        let mut prog_indexer = ProgrammableBrc20Indexer::new();
        prog_indexer.index_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn call(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::CallRequest = from_slice(input)?;
    Ok(to_vec(&view::call(&req)?)?)
}

#[cfg(test)]
mod tests;
