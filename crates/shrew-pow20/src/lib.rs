#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod pow20_indexer;
pub mod tables;
pub mod view;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/shrew_pow20.rs"));
}

pub use pow20_indexer::Pow20Indexer;

#[cfg(feature = "entrypoint")]
#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        let mut inscription_indexer = shrew_ord::indexer::InscriptionIndexer::new();
        let _ = inscription_indexer.load_state();
        let _ = inscription_indexer.index_block(&block, height);

        let indexer = Pow20Indexer::new();
        indexer.index_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn getpow20balance(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetPow20BalanceRequest = from_slice(input)?;
    Ok(to_vec(&view::get_pow20_balance(&req)?)?)
}

#[metashrew_core::view]
pub fn getpow20events(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetPow20EventsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_pow20_events(&req)?)?)
}

#[cfg(test)]
mod tests;
