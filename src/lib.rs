use bitcoin::{Block, consensus::deserialize};

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
fn getinscription(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_inscription(&req)?)?)
}

#[metashrew_core::view]
fn getinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_inscriptions(&req)?)?)
}

#[metashrew_core::view]
fn getchildren(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_children(&req)?)?)
}

#[metashrew_core::view]
fn getparents(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_parents(&req)?)?)
}

#[metashrew_core::view]
fn getcontent(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_content(&req)?)?)
}

#[metashrew_core::view]
fn getmetadata(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_metadata(&req)?)?)
}

#[metashrew_core::view]
fn getsat(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_sat(&req)?)?)
}

#[metashrew_core::view]
fn getsatinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_sat_inscriptions(&req)?)?)
}

#[metashrew_core::view]
fn getsatinscription(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_sat_inscription(&req)?)?)
}

#[metashrew_core::view]
fn getchildinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_child_inscriptions(&req)?)?)
}

#[metashrew_core::view]
fn getparentinscriptions(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_parent_inscriptions(&req)?)?)
}

#[metashrew_core::view]
fn getundelegatedcontent(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_undelegated_content(&req)?)?)
}

#[metashrew_core::view]
fn getutxo(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_utxo(&req)?)?)
}

#[metashrew_core::view]
fn getblockhash(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_block_hash(&req)?)?)
}

#[metashrew_core::view]
fn getblockheight(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_block_height(&req)?)?)
}

#[metashrew_core::view]
fn getblocktime(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_block_time(&req)?)?)
}

#[metashrew_core::view]
fn getblockinfo(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_block_info(&req)?)?)
}

#[metashrew_core::view]
fn gettransaction(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_tx(&req)?)?)
}

#[metashrew_core::view]
fn getbalance(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_balance(&req)?)?)
}

#[metashrew_core::view]
fn getbrc20events(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req = serde_json::from_slice(input)?;
    Ok(serde_json::to_vec(&view::get_brc20_events(&req)?)?)
}
