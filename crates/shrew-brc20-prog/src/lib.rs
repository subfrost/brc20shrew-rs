#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod prog_indexer;
pub mod controller;
pub mod trace_hash;
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

/// Debug view: returns the last processed inscription content and EVM execution result.
/// Call via metashrew_view ["debug", "0x", "latest"]
#[metashrew_core::view]
pub fn debug(_input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use metashrew_support::index_pointer::KeyValuePointer;
    let last_inscription = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_inscription").get();
    let last_result = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_result").get();
    let response = serde_json::json!({
        "last_inscription": String::from_utf8_lossy(&last_inscription).to_string(),
        "last_result": String::from_utf8_lossy(&last_result).to_string(),
    });
    Ok(serde_json::to_vec(&response)?)
}

#[cfg(test)]
mod tests;
