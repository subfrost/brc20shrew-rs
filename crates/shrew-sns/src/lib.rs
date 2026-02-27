#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod sns_indexer;
pub mod tables;
pub mod view;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/shrew_sns.rs"));
}

pub use sns_indexer::SnsIndexer;

#[cfg(feature = "entrypoint")]
#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        let mut inscription_indexer = shrew_ord::indexer::InscriptionIndexer::new();
        let _ = inscription_indexer.load_state();
        let _ = inscription_indexer.index_block(&block, height);

        let indexer = SnsIndexer::new();
        indexer.index_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn getsnsname(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSnsNameRequest = from_slice(input)?;
    Ok(to_vec(&view::get_sns_name(&req)?)?)
}

#[metashrew_core::view]
pub fn getsnsnamespace(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSnsNamespaceRequest = from_slice(input)?;
    Ok(to_vec(&view::get_sns_namespace(&req)?)?)
}

#[metashrew_core::view]
pub fn getsnsnamesbyheight(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSnsNamesByHeightRequest = from_slice(input)?;
    Ok(to_vec(&view::get_sns_names_by_height(&req)?)?)
}

#[cfg(test)]
mod tests;
