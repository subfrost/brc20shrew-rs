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

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getinscription(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetInscriptionRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_inscription(&req)?)?)
}
#[cfg(test)]
pub fn getinscription(req: &proto::GetInscriptionRequest) -> Result<proto::InscriptionResponse, Box<dyn std::error::Error>> {
    Ok(view::get_inscription(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getinscriptions(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetInscriptionsRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_inscriptions(&req)?)?)
}
#[cfg(test)]
pub fn getinscriptions(req: &proto::GetInscriptionsRequest) -> Result<proto::InscriptionsResponse, Box<dyn std::error::Error>> {
    Ok(view::get_inscriptions(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getchildren(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetChildrenRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_children(&req)?)?)
}
#[cfg(test)]
pub fn getchildren(req: &proto::GetChildrenRequest) -> Result<proto::ChildrenResponse, Box<dyn std::error::Error>> {
    Ok(view::get_children(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getparents(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetParentsRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_parents(&req)?)?)
}
#[cfg(test)]
pub fn getparents(req: &proto::GetParentsRequest) -> Result<proto::ParentsResponse, Box<dyn std::error::Error>> {
    Ok(view::get_parents(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getcontent(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetContentRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_content(&req)?)?)
}
#[cfg(test)]
pub fn getcontent(req: &proto::GetContentRequest) -> Result<proto::ContentResponse, Box<dyn std::error::Error>> {
    Ok(view::get_content(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getmetadata(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetMetadataRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_metadata(&req)?)?)
}
#[cfg(test)]
pub fn getmetadata(req: &proto::GetMetadataRequest) -> Result<proto::MetadataResponse, Box<dyn std::error::Error>> {
    Ok(view::get_metadata(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getsat(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_sat(&req)?)?)
}
#[cfg(test)]
pub fn getsat(req: &proto::GetSatRequest) -> Result<proto::SatResponse, Box<dyn std::error::Error>> {
    Ok(view::get_sat(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getsatinscriptions(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatInscriptionsRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_sat_inscriptions(&req)?)?)
}
#[cfg(test)]
pub fn getsatinscriptions(req: &proto::GetSatInscriptionsRequest) -> Result<proto::SatInscriptionsResponse, Box<dyn std::error::Error>> {
    Ok(view::get_sat_inscriptions(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getsatinscription(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetSatInscriptionRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_sat_inscription(&req)?)?)
}
#[cfg(test)]
pub fn getsatinscription(req: &proto::GetSatInscriptionRequest) -> Result<proto::SatInscriptionResponse, Box<dyn std::error::Error>> {
    Ok(view::get_sat_inscription(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getchildinscriptions(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetChildInscriptionsRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_child_inscriptions(&req)?)?)
}
#[cfg(test)]
pub fn getchildinscriptions(req: &proto::GetChildInscriptionsRequest) -> Result<proto::ChildInscriptionsResponse, Box<dyn std::error::Error>> {
    Ok(view::get_child_inscriptions(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getparentinscriptions(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetParentInscriptionsRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_parent_inscriptions(&req)?)?)
}
#[cfg(test)]
pub fn getparentinscriptions(req: &proto::GetParentInscriptionsRequest) -> Result<proto::ParentInscriptionsResponse, Box<dyn std::error::Error>> {
    Ok(view::get_parent_inscriptions(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getundelegatedcontent(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetUndelegatedContentRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_undelegated_content(&req)?)?)
}
#[cfg(test)]
pub fn getundelegatedcontent(req: &proto::GetUndelegatedContentRequest) -> Result<proto::UndelegatedContentResponse, Box<dyn std::error::Error>> {
    Ok(view::get_undelegated_content(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getutxo(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetUtxoRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_utxo(&req)?)?)
}
#[cfg(test)]
pub fn getutxo(req: &proto::GetUtxoRequest) -> Result<proto::UtxoResponse, Box<dyn std::error::Error>> {
    Ok(view::get_utxo(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getblockhash(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockHashRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_block_hash(&req)?)?)
}
#[cfg(test)]
pub fn getblockhash(req: &proto::GetBlockHashRequest) -> Result<proto::BlockHashResponse, Box<dyn std::error::Error>> {
    Ok(view::get_block_hash(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getblockheight(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockHeightRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_block_height(&req)?)?)
}
#[cfg(test)]
pub fn getblockheight(req: &proto::GetBlockHeightRequest) -> Result<proto::BlockHeightResponse, Box<dyn std::error::Error>> {
    Ok(view::get_block_height(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getblocktime(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockTimeRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_block_time(&req)?)?)
}
#[cfg(test)]
pub fn getblocktime(req: &proto::GetBlockTimeRequest) -> Result<proto::BlockTimeResponse, Box<dyn std::error::Error>> {
    Ok(view::get_block_time(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getblockinfo(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBlockInfoRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_block_info(&req)?)?)
}
#[cfg(test)]
pub fn getblockinfo(req: &proto::GetBlockInfoRequest) -> Result<proto::BlockInfoResponse, Box<dyn std::error::Error>> {
    Ok(view::get_block_info(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn gettransaction(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetTransactionRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_tx(&req)?)?)
}
#[cfg(test)]
pub fn gettransaction(req: &proto::GetTransactionRequest) -> Result<proto::TransactionResponse, Box<dyn std::error::Error>> {
    Ok(view::get_tx(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getbalance(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBalanceRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_balance(&req)?)?)
}
#[cfg(test)]
pub fn getbalance(req: &proto::GetBalanceRequest) -> Result<proto::BalanceResponse, Box<dyn std::error::Error>> {
    Ok(view::get_balance(req)?)
}

#[cfg(not(test))]
#[metashrew_core::view]
pub fn getbrc20events(raw_req: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let req: proto::GetBrc20EventsRequest = from_slice(raw_req)?;
    Ok(to_vec(&view::get_brc20_events(&req)?)?)
}
#[cfg(test)]
pub fn getbrc20events(req: &proto::GetBrc20EventsRequest) -> Result<proto::Brc20EventsResponse, Box<dyn std::error::Error>> {
    Ok(view::get_brc20_events(req)?)
}
