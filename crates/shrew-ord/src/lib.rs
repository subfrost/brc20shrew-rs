#[cfg(feature = "entrypoint")]
use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

pub mod tables;
pub mod envelope;
pub mod indexer;
pub mod view;
pub mod message;
pub mod ord_inscriptions;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/shrew_ord.rs"));
}

pub use shrew_support::inscription::{InscriptionId, SatPoint, InscriptionEntry, Charm, Rarity, Media};
pub use shrew_support::utils::get_address_from_txout;

// Re-export view functions
pub use view::*;

#[cfg(feature = "entrypoint")]
#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        let mut idx = indexer::InscriptionIndexer::new();
        let _ = idx.load_state();
        let _ = idx.index_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn getinscription(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetInscriptionRequest = from_slice(input)?;
    Ok(to_vec(&view::get_inscription(&req)?)?)
}

#[metashrew_core::view]
pub fn getinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetInscriptionsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getchildren(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetChildrenRequest = from_slice(input)?;
    Ok(to_vec(&view::get_children(&req)?)?)
}

#[metashrew_core::view]
pub fn getparents(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetParentsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_parents(&req)?)?)
}

#[metashrew_core::view]
pub fn getcontent(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetContentRequest = from_slice(input)?;
    Ok(to_vec(&view::get_content(&req)?)?)
}

#[metashrew_core::view]
pub fn getmetadata(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetMetadataRequest = from_slice(input)?;
    Ok(to_vec(&view::get_metadata(&req)?)?)
}

#[metashrew_core::view]
pub fn getsat(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatRequest = from_slice(input)?;
    Ok(to_vec(&view::get_sat(&req)?)?)
}

#[metashrew_core::view]
pub fn getsatinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatInscriptionsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_sat_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getsatinscription(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatInscriptionRequest = from_slice(input)?;
    Ok(to_vec(&view::get_sat_inscription(&req)?)?)
}

#[metashrew_core::view]
pub fn getchildinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetChildInscriptionsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_child_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getparentinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetParentInscriptionsRequest = from_slice(input)?;
    Ok(to_vec(&view::get_parent_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getundelegatedcontent(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetUndelegatedContentRequest = from_slice(input)?;
    Ok(to_vec(&view::get_undelegated_content(&req)?)?)
}

#[metashrew_core::view]
pub fn getutxo(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetUtxoRequest = from_slice(input)?;
    Ok(to_vec(&view::get_utxo(&req)?)?)
}

#[metashrew_core::view]
pub fn getblockhash(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockHashRequest = from_slice(input)?;
    Ok(to_vec(&view::get_block_hash(&req)?)?)
}

#[metashrew_core::view]
pub fn getblockheight(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockHeightRequest = from_slice(input)?;
    Ok(to_vec(&view::get_block_height(&req)?)?)
}

#[metashrew_core::view]
pub fn getblocktime(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockTimeRequest = from_slice(input)?;
    Ok(to_vec(&view::get_block_time(&req)?)?)
}

#[metashrew_core::view]
pub fn getblockinfo(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockInfoRequest = from_slice(input)?;
    Ok(to_vec(&view::get_block_info(&req)?)?)
}

#[metashrew_core::view]
pub fn gettransaction(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetTransactionRequest = from_slice(input)?;
    Ok(to_vec(&view::get_tx(&req)?)?)
}

#[cfg(test)]
mod tests;
