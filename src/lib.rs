use bitcoin::{Block, consensus::deserialize};
use serde_json::{from_slice, to_vec};

// Re-export modules
pub mod tables;
pub mod inscription;
pub mod envelope;
pub mod indexer;
pub mod view;
pub mod message;
pub mod ord_inscriptions;
pub mod brc20;
pub mod utils;
pub mod programmable_brc20;

// Re-export protobuf types
pub mod proto;

// Re-export view functions for testing
pub use view::*;

// Test modules
#[cfg(any(feature = "test-utils", test))]
pub mod tests;

#[metashrew_core::main]
fn main_logic(height: u32, block_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(block) = deserialize::<Block>(block_data) {
        let mut indexer = indexer::InscriptionIndexer::new();
        let _ = indexer.load_state();
        let _ = indexer.index_block(&block, height);
    }
    Ok(())
}

#[metashrew_core::view]
pub fn getinscription(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetInscriptionRequest = from_slice(req)?;
    Ok(to_vec(&view::get_inscription(&req)?)?)
}

#[metashrew_core::view]
pub fn getinscriptions(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetInscriptionsRequest = from_slice(req)?;
    Ok(to_vec(&view::get_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getchildren(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetChildrenRequest = from_slice(req)?;
    Ok(to_vec(&view::get_children(&req)?)?)
}

#[metashrew_core::view]
pub fn getparents(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetParentsRequest = from_slice(req)?;
    Ok(to_vec(&view::get_parents(&req)?)?)
}

#[metashrew_core::view]
pub fn getcontent(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetContentRequest = from_slice(req)?;
    Ok(to_vec(&view::get_content(&req)?)?)
}

#[metashrew_core::view]
pub fn getmetadata(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetMetadataRequest = from_slice(req)?;
    Ok(to_vec(&view::get_metadata(&req)?)?)
}

#[metashrew_core::view]
pub fn getsat(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatRequest = from_slice(req)?;
    Ok(to_vec(&view::get_sat(&req)?)?)
}

#[metashrew_core::view]
pub fn getsatinscriptions(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatInscriptionsRequest = from_slice(req)?;
    Ok(to_vec(&view::get_sat_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getsatinscription(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatInscriptionRequest = from_slice(req)?;
    Ok(to_vec(&view::get_sat_inscription(&req)?)?)
}

#[metashrew_core::view]
pub fn getchildinscriptions(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetChildInscriptionsRequest = from_slice(req)?;
    Ok(to_vec(&view::get_child_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getparentinscriptions(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetParentInscriptionsRequest = from_slice(req)?;
    Ok(to_vec(&view::get_parent_inscriptions(&req)?)?)
}

#[metashrew_core::view]
pub fn getundelegatedcontent(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetUndelegatedContentRequest = from_slice(req)?;
    Ok(to_vec(&view::get_undelegated_content(&req)?)?)
}

#[metashrew_core::view]
pub fn getutxo(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetUtxoRequest = from_slice(req)?;
    Ok(to_vec(&view::get_utxo(&req)?)?)
}

#[metashrew_core::view]
pub fn getblockhash(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockHashRequest = from_slice(req)?;
    Ok(to_vec(&view::get_block_hash(&req)?)?)
}

#[metashrew_core::view]
pub fn getblockheight(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockHeightRequest = from_slice(req)?;
    Ok(to_vec(&view::get_block_height(&req)?)?)
}

#[metashrew_core::view]
pub fn getblocktime(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockTimeRequest = from_slice(req)?;
    Ok(to_vec(&view::get_block_time(&req)?)?)
}

#[metashrew_core::view]
pub fn getblockinfo(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockInfoRequest = from_slice(req)?;
    Ok(to_vec(&view::get_block_info(&req)?)?)
}

#[metashrew_core::view]
pub fn gettransaction(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetTransactionRequest = from_slice(req)?;
    Ok(to_vec(&view::get_tx(&req)?)?)
}

#[metashrew_core::view]
pub fn getbalance(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBalanceRequest = from_slice(req)?;
    Ok(to_vec(&view::get_balance(&req)?)?)
}

#[metashrew_core::view]
pub fn getbrc20events(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBrc20EventsRequest = from_slice(req)?;
    Ok(to_vec(&view::get_brc20_events(&req)?)?)
}

#[metashrew_core::view]
pub fn call(req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::CallRequest = from_slice(req)?;
    Ok(to_vec(&view::call(&req)?)?)
}
