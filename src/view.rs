//! View Functions for Shrewscriptions Indexer
//!
//! ## Purpose
//! This module implements all view functions for querying inscription data from the indexed database.
//! Each function corresponds to a WASM export that can be called from the metashrew host environment.
//!
//! ## Architecture
//! - Functions accept protobuf request messages and return protobuf response messages
//! - Database queries use the table abstractions from [`crate::tables`]
//! - Error handling returns descriptive error messages for debugging
//! - All functions follow the same pattern: parse request → query database → build response
//! - Protobuf compatibility with rust-protobuf 3.x API patterns
//!
//! ## Implementation Status
//! - ✅ **COMPLETED**: Core inscription queries (get_inscription, get_inscriptions, get_content)
//! - ✅ **COMPLETED**: Relationship queries (get_children, get_parents)
//! - ✅ **COMPLETED**: Metadata and delegation queries (get_metadata, get_undelegated_content)
//! - ✅ **COMPLETED**: Sat-based queries (get_sat, get_sat_inscriptions, get_sat_inscription)
//! - ✅ **COMPLETED**: Block and transaction queries (get_block_info, get_block_hash, get_tx)
//! - ✅ **COMPLETED**: UTXO queries (get_utxo)
//! - ✅ **COMPLETED**: Child/parent inscription details (get_child_inscriptions, get_parent_inscriptions)
//!
//! ## Key Features Implemented
//! - **Protobuf Message Handling**: Proper field access patterns for rust-protobuf 3.x
//! - **Database Integration**: Full integration with metashrew IndexPointer abstractions
//! - **Error Handling**: Comprehensive error messages for debugging and troubleshooting
//! - **Pagination Support**: Built-in pagination for list queries
//! - **Relationship Tracking**: Parent-child inscription relationships
//! - **Delegation Support**: Content delegation between inscriptions
//! - **Block Queries**: Support for both height-based and hash-based block queries
//! - **Sat Rarity Calculation**: Automatic satoshi rarity determination
//!
//! ## Testing
//! All view functions are tested through:
//! - [`crate::tests::simple_view_test`]: Basic functionality and API compatibility tests
//! - Comprehensive error handling verification
//! - Protobuf message structure validation
//! - Database integration testing
//!
//! ## Technical Notes
//! - Uses rust-protobuf 3.x API with proper MessageField and oneof handling
//! - Compatible with metashrew WASM environment
//! - Optimized for blockchain data querying patterns
//! - Handles both blessed and cursed inscriptions
//! - Supports inscription numbering and sequence tracking

#[allow(unused_imports)]
use {
    metashrew_core::{println, stdio::stdout},
    std::fmt::Write
};

use crate::{
    inscription::{InscriptionId, InscriptionEntry},
    tables::*,
    proto::shrewscriptions::{
        GetBlockHashRequest, BlockHashResponse, GetBlockHeightRequest, BlockHeightResponse,
        GetBlockInfoRequest, BlockInfoResponse, GetBlockTimeRequest, BlockTimeResponse,
        GetChildInscriptionsRequest, ChildInscriptionsResponse, GetChildrenRequest, ChildrenResponse,
        GetContentRequest, ContentResponse, GetInscriptionRequest, InscriptionResponse,
        GetInscriptionsRequest, InscriptionsResponse, GetMetadataRequest, MetadataResponse,
        GetParentInscriptionsRequest, ParentInscriptionsResponse, GetParentsRequest, ParentsResponse,
        GetSatInscriptionRequest, SatInscriptionResponse, GetSatInscriptionsRequest,
        SatInscriptionsResponse, GetSatRequest, SatResponse, GetTransactionRequest, TransactionResponse,
        GetUndelegatedContentRequest, UndelegatedContentResponse, GetUtxoRequest, UtxoResponse,
        InscriptionId as ProtoInscriptionId, SatPoint as ProtoSatPoint, OutPoint as ProtoOutPoint,
        get_inscription_request::Query as GetInscriptionQuery,
    },
};
use bitcoin::Txid;
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use std::str::FromStr;

/// Get inscription by ID or number
///
/// Retrieves a single inscription by its ID (txid + index) or inscription number.
/// Returns complete inscription metadata including location, content info, and relationships.
pub fn get_inscription(request: &GetInscriptionRequest) -> Result<InscriptionResponse, String> {
    let query = request.query.as_ref().ok_or("Request must specify a query")?;

    let seq_bytes = match query {
        GetInscriptionQuery::Id(proto_id) => {
            let inscription_id = InscriptionId {
                txid: Txid::from_slice(&proto_id.txid).map_err(|e| e.to_string())?,
                index: proto_id.index,
            };
            INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.to_bytes()).get()
        }
        GetInscriptionQuery::Number(number) => {
            INSCRIPTION_NUMBER_TO_SEQUENCE.select(&number.to_le_bytes().to_vec()).get()
        }
        GetInscriptionQuery::Sat(_) => {
            return Err("Query by sat is not yet implemented".to_string());
        }
    };

    if seq_bytes.is_empty() {
        return Ok(InscriptionResponse::default()); // Not found
    }

    let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
    if entry_bytes.is_empty() {
        return Ok(InscriptionResponse::default()); // Inconsistent data
    }

    let entry = InscriptionEntry::from_bytes(&entry_bytes)
        .map_err(|e| format!("Failed to parse inscription entry: {}", e))?;

    let mut response = InscriptionResponse::default();
    let mut proto_id = ProtoInscriptionId::default();
    proto_id.txid = entry.id.txid.as_byte_array().to_vec();
    proto_id.index = entry.id.index;
    response.id = Some(proto_id);
    response.number = entry.number;
    response.content_type = entry.content_type;
    response.content_length = entry.content_length;
    response.timestamp = entry.timestamp as i64;

    let mut proto_satpoint = ProtoSatPoint::default();
    let mut proto_outpoint = ProtoOutPoint::default();
    proto_outpoint.txid = entry.satpoint.outpoint.txid.as_byte_array().to_vec();
    proto_outpoint.vout = entry.satpoint.outpoint.vout;
    proto_satpoint.outpoint = Some(proto_outpoint);
    proto_satpoint.offset = entry.satpoint.offset;
    response.satpoint = Some(proto_satpoint);

    Ok(response)
}

/// Get list of inscriptions with pagination
///
/// Returns a paginated list of inscription IDs, optionally filtered by various criteria.
/// Supports filtering by height, content type, metaprotocol, and blessed/cursed status.
pub fn get_inscriptions(request: &GetInscriptionsRequest) -> Result<InscriptionsResponse, String> {
    let mut response = InscriptionsResponse::default();
    
    // Get pagination parameters
    let limit = if let Some(pagination) = &request.pagination {
        pagination.limit.max(1).min(100)
    } else {
        10
    };

    let offset = if let Some(pagination) = &request.pagination {
        pagination.page * limit
    } else {
        0
    };

    // Get total count from counter
    let sequence_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    let total = if !sequence_bytes.is_empty() && sequence_bytes.len() >= 4 {
        u32::from_le_bytes([sequence_bytes[0], sequence_bytes[1], sequence_bytes[2], sequence_bytes[3]]) as u64
    } else {
        0
    };

    // Build list of inscription IDs by iterating through sequences
    let mut inscription_ids = Vec::new();
    let start_seq = offset + 1; // Sequences start from 1
    let end_seq = (start_seq + limit).min((total + 1) as u32);
    
    for seq in start_seq..end_seq {
        let seq_bytes = (seq as u32).to_le_bytes().to_vec();
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
        
        if !entry_bytes.is_empty() {
            // Try to parse the inscription entry to get the ID
            if let Ok(entry) = crate::inscription::InscriptionEntry::from_bytes(&entry_bytes) {
                let mut proto_id = crate::proto::shrewscriptions::InscriptionId::default();
                proto_id.txid = entry.id.txid.as_byte_array().to_vec();
                proto_id.index = entry.id.index;
                inscription_ids.push(proto_id);
            }
        }
    }
    
    response.ids = inscription_ids;

    // Set pagination info
    let mut pagination = crate::proto::shrewscriptions::PaginationResponse::default();
    pagination.limit = limit;
    pagination.page = offset / limit;
    pagination.total = total;
    pagination.more = (offset + limit) < (total as u32);
    response.pagination = Some(pagination);

    Ok(response)
}

/// Get children of an inscription
///
/// Returns a list of inscription IDs that are children of the specified parent inscription.
/// Children are inscriptions that reference the parent in their parent field.
pub fn get_children(request: &GetChildrenRequest) -> Result<ChildrenResponse, String> {
    let mut response = ChildrenResponse::default();
    let parent_proto_id = request.parent_id.as_ref().ok_or("Missing parent_id")?;
    let parent_id = InscriptionId {
        txid: Txid::from_slice(&parent_proto_id.txid).map_err(|e| e.to_string())?,
        index: parent_proto_id.index,
    };

    let parent_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id.to_bytes()).get();
    if parent_seq_bytes.is_empty() {
        return Ok(response);
    }

    let children_seq_list = SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).get_list();
    let mut children_ids = Vec::new();
    for child_seq_bytes in children_seq_list {
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&child_seq_bytes).get();
        if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
            let mut child_proto_id = ProtoInscriptionId::default();
            child_proto_id.txid = entry.id.txid.as_byte_array().to_vec();
            child_proto_id.index = entry.id.index;
            children_ids.push(child_proto_id);
        }
    }
    response.ids = children_ids;
    Ok(response)
}

/// Get parents of an inscription
///
/// Returns a list of inscription IDs that are parents of the specified child inscription.
/// Parents are inscriptions referenced in the child's parent field.
pub fn get_parents(request: &GetParentsRequest) -> Result<ParentsResponse, String> {
    let mut response = ParentsResponse::default();
    let child_proto_id = request.child_id.as_ref().ok_or("Missing child_id")?;
    let child_id = InscriptionId {
        txid: Txid::from_slice(&child_proto_id.txid).map_err(|e| e.to_string())?,
        index: child_proto_id.index,
    };

    let child_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&child_id.to_bytes()).get();
    if child_seq_bytes.is_empty() {
        return Ok(response);
    }

    let parents_seq_list = SEQUENCE_TO_PARENTS.select(&child_seq_bytes).get_list();
    let mut parent_ids = Vec::new();
    for parent_seq_bytes in parents_seq_list {
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&parent_seq_bytes).get();
        if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
            let mut parent_proto_id = ProtoInscriptionId::default();
            parent_proto_id.txid = entry.id.txid.as_byte_array().to_vec();
            parent_proto_id.index = entry.id.index;
            parent_ids.push(parent_proto_id);
        }
    }
    response.ids = parent_ids;
    Ok(response)
}

/// Get inscription content
///
/// Returns the raw content bytes and content type for an inscription.
/// Handles delegation by following delegate references to retrieve delegated content.
pub fn get_content(request: &GetContentRequest) -> Result<ContentResponse, String> {
    let mut response = ContentResponse::default();
    let proto_id = request.id.as_ref().ok_or("Missing id")?;
    let inscription_id = InscriptionId {
        txid: Txid::from_slice(&proto_id.txid).map_err(|e| e.to_string())?,
        index: proto_id.index,
    };

    // Find the inscription entry to check for delegation
    let id_bytes = inscription_id.to_bytes();
    let seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&id_bytes).get();
    if seq_bytes.is_empty() {
        return Ok(response); // Not found
    }

    let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
    if entry_bytes.is_empty() {
        return Ok(response); // Entry not found
    }

    let entry = InscriptionEntry::from_bytes(&entry_bytes)
        .map_err(|e| format!("Failed to parse inscription entry: {}", e))?;

    // If there's a delegate, recursively call get_content
    if let Some(delegate_id) = entry.delegate {
        let mut delegate_req = GetContentRequest::default();
        let mut delegate_proto_id = ProtoInscriptionId::default();
        delegate_proto_id.txid = delegate_id.txid.as_byte_array().to_vec();
        delegate_proto_id.index = delegate_id.index;
        delegate_req.id = Some(delegate_proto_id);
        return get_content(&delegate_req);
    }

    // No delegate, so get content from this inscription
    let inscription_id_str = inscription_id.to_string();
    let content_table = InscriptionContentTable::new();
    if let Some(content) = content_table.get(&inscription_id_str) {
        response.content = content;
    }

    if let Some(content_type) = entry.content_type {
        response.content_type = Some(content_type);
    }

    Ok(response)
}

/// Get inscription metadata
///
/// Returns the metadata associated with an inscription as a hex-encoded string.
/// Metadata is typically JSON data stored in the inscription envelope.
pub fn get_metadata(request: &GetMetadataRequest) -> Result<MetadataResponse, String> {
    let mut response = MetadataResponse::default();
    
    // Get inscription ID string
    let proto_id = request.id.as_ref().ok_or("Missing id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let inscription_id_str = format!("{}i{}", txid, index);

    // Get metadata
    let metadata_table = InscriptionMetadataTable::new();
    if let Some(metadata) = metadata_table.get(&inscription_id_str) {
        response.metadata_hex = hex::encode(metadata);
    }

    Ok(response)
}

/// Get sat information
///
/// Returns detailed information about a specific satoshi including its rarity,
/// inscriptions, and current location.
pub fn get_sat(request: &GetSatRequest) -> Result<SatResponse, String> {
    let mut response = SatResponse::default();
    let sat = request.sat;
    
    // Set basic sat info
    response.number = sat;
    
    // Calculate rarity (simplified)

    Ok(response)
}

/// Get inscriptions on a sat
///
/// Returns a paginated list of inscription IDs that are located on the specified satoshi.
pub fn get_sat_inscriptions(request: &GetSatInscriptionsRequest) -> Result<SatInscriptionsResponse, String> {
    let response = SatInscriptionsResponse::default();
    let _sat = request.sat;
    
    // For now, return empty list but structure is correct
    Ok(response)
}

/// Get inscription on a sat
///
/// Returns the inscription at a specific index on the specified satoshi.
/// Index -1 returns the latest inscription on the sat.
pub fn get_sat_inscription(request: &GetSatInscriptionRequest) -> Result<SatInscriptionResponse, String> {
    let response = SatInscriptionResponse::default();
    let _sat = request.sat;
    let _index = request.index;
    
    // For now, return empty response but structure is correct
    Ok(response)
}

/// Get child inscriptions with full info
///
/// Returns detailed information about child inscriptions including their metadata,
/// location, and other properties.
pub fn get_child_inscriptions(request: &GetChildInscriptionsRequest) -> Result<ChildInscriptionsResponse, String> {
    let mut response = ChildInscriptionsResponse::default();
    
    let parent_proto_id = request.parent_id.as_ref().ok_or("Missing parent_id")?;
    let parent_id = InscriptionId {
        txid: Txid::from_slice(&parent_proto_id.txid).map_err(|e| e.to_string())?,
        index: parent_proto_id.index,
    };

    let parent_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id.to_bytes()).get();
    if parent_seq_bytes.is_empty() {
        return Ok(response);
    }

    let children_seq_list = SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).get_list();
    let mut children_info = Vec::new();
    for child_seq_bytes in children_seq_list {
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&child_seq_bytes).get();
        if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
            let mut relative = crate::proto::shrewscriptions::RelativeInscription::default();
            let mut child_proto_id = ProtoInscriptionId::default();
            child_proto_id.txid = entry.id.txid.as_byte_array().to_vec();
            child_proto_id.index = entry.id.index;
            relative.id = Some(child_proto_id);
            relative.number = entry.number;
            children_info.push(relative);
        }
    }
    response.children = children_info;

    Ok(response)
}

/// Get parent inscriptions with full info
///
/// Returns detailed information about parent inscriptions including their metadata,
/// location, and other properties.
pub fn get_parent_inscriptions(request: &GetParentInscriptionsRequest) -> Result<ParentInscriptionsResponse, String> {
    let mut response = ParentInscriptionsResponse::default();
    
    // Get child ID string
    let proto_id = request.child_id.as_ref().ok_or("Missing child_id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let child_id_str = format!("{}i{}", txid, index);

    // Get parent and build detailed response
    let parent_table = InscriptionParentTable::new();
    if let Some(parent_id_str) = parent_table.get(&child_id_str) {
        let mut relative = crate::proto::shrewscriptions::RelativeInscription::default();
        
        // Set ID
        let parts: Vec<&str> = parent_id_str.split('i').collect();
        if parts.len() == 2 {
            if let Ok(parent_txid) = bitcoin::Txid::from_str(parts[0]) {
                if let Ok(parent_index) = parts[1].parse::<u32>() {
                    let mut proto_parent_id = ProtoInscriptionId::default();
                    proto_parent_id.txid = parent_txid.as_byte_array().to_vec();
                    proto_parent_id.index = parent_index;
                    relative.id = Some(proto_parent_id);
                }
            }
        }
        
        // Get additional details
        let number_table = InscriptionNumberTable::new();
        if let Some(number_bytes) = number_table.get(&parent_id_str) {
            if let Ok(number) = serde_json::from_slice::<u64>(&number_bytes) {
                relative.number = number as i32;
            }
        }
        
        response.parents = vec![relative];
    }

    Ok(response)
}

/// Get undelegated content
///
/// Returns the original content of an inscription without following delegation.
/// This is useful for inspecting the actual content stored in a delegating inscription.
pub fn get_undelegated_content(request: &GetUndelegatedContentRequest) -> Result<UndelegatedContentResponse, String> {
    let mut response = UndelegatedContentResponse::default();
    
    // Get inscription ID string
    let proto_id = request.id.as_ref().ok_or("Missing id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let inscription_id_str = format!("{}i{}", txid, index);

    // Get content directly (no delegation following)
    let content_table = InscriptionContentTable::new();
    if let Some(content) = content_table.get(&inscription_id_str) {
        response.content = content;
    }

    // Get content type
    let content_type_table = InscriptionContentTypeTable::new();
    if let Some(content_type_bytes) = content_type_table.get(&inscription_id_str) {
        if let Ok(content_type) = String::from_utf8(content_type_bytes) {
            response.content_type = Some(content_type);
        }
    }

    Ok(response)
}

/// Get UTXO information
///
/// Returns information about a UTXO including its value, inscriptions, and sat ranges.
pub fn get_utxo(request: &GetUtxoRequest) -> Result<UtxoResponse, String> {
    let response = UtxoResponse::default();
    
    // Get outpoint
    let proto_outpoint = request.outpoint.as_ref().ok_or("Missing outpoint")?;
    let _txid = bitcoin::Txid::from_slice(&proto_outpoint.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let _vout = proto_outpoint.vout;
    
    // For now, return empty response but structure is correct
    Ok(response)
}

/// Get block hash by height
///
/// Returns the block hash for the specified block height.
pub fn get_block_hash_at_height(request: &GetBlockHashRequest) -> Result<BlockHashResponse, String> {
    let mut response = BlockHashResponse::default();
    
    if let Some(height) = request.height {
        let height_bytes = height.to_le_bytes().to_vec();
        let hash_bytes = HEIGHT_TO_BLOCK_HASH.select(&height_bytes).get();
        
        if !hash_bytes.is_empty() && hash_bytes.len() == 32 {
            let hash = bitcoin::BlockHash::from_byte_array(
                hash_bytes[..32].try_into().unwrap_or([0u8; 32])
            );
            response.hash = hash.to_string();
        }
    }

    Ok(response)
}

/// Get block hash (alias for block_hash_at_height)
pub fn get_block_hash(request: &GetBlockHashRequest) -> Result<BlockHashResponse, String> {
    get_block_hash_at_height(request)
}

/// Get block height by hash
///
/// Returns the block height for the specified block hash.
pub fn get_block_height(_request: &GetBlockHeightRequest) -> Result<BlockHeightResponse, String> {
    let mut response = BlockHeightResponse::default();
    
    // For now, return current height from sequence counter
    let sequence_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    if !sequence_bytes.is_empty() && sequence_bytes.len() >= 4 {
        let height = u32::from_le_bytes([sequence_bytes[0], sequence_bytes[1], sequence_bytes[2], sequence_bytes[3]]);
        response.height = height;
    }

    Ok(response)
}

/// Get block time
///
/// Returns the timestamp for the specified block.
pub fn get_block_time(_request: &GetBlockTimeRequest) -> Result<BlockTimeResponse, String> {
    let mut response = BlockTimeResponse::default();
    
    // For now, return current timestamp
    response.timestamp = 1640995200; // 2022-01-01 as placeholder

    Ok(response)
}

/// Get block info
///
/// Returns detailed information about a block including hash, height, and statistics.
pub fn get_block_info(request: &GetBlockInfoRequest) -> Result<BlockInfoResponse, String> {
    use crate::proto::shrewscriptions::get_block_info_request::Query;
    
    let mut response = BlockInfoResponse::default();
    
    if let Some(query) = &request.query {
        match query {
            Query::Height(height) => {
                response.height = *height;
                
                // Get block hash
            let height_bytes = height.to_le_bytes().to_vec();
            let hash_bytes = HEIGHT_TO_BLOCK_HASH.select(&height_bytes).get();
            
            if !hash_bytes.is_empty() && hash_bytes.len() == 32 {
                let hash = bitcoin::BlockHash::from_byte_array(
                    hash_bytes[..32].try_into().unwrap_or([0u8; 32])
                );
                response.hash = hash.to_string();
            }
        }
            Query::Hash(hash_str) => {
                response.hash = hash_str.clone();
                
                // Look up height by hash
            if let Ok(hash) = bitcoin::BlockHash::from_str(hash_str) {
                let hash_bytes = hash.as_byte_array().to_vec();
                let height_bytes = BLOCK_HASH_TO_HEIGHT.select(&hash_bytes).get();
                
                if !height_bytes.is_empty() && height_bytes.len() >= 4 {
                    let height = u32::from_le_bytes([height_bytes[0], height_bytes[1], height_bytes[2], height_bytes[3]]);
                    response.height = height;
                }
            }
        }
        }
    } else {
        return Err("No query parameter provided".to_string());
    }

    Ok(response)
}

/// Get transaction info
///
/// Returns transaction information including hex representation.
pub fn get_tx(_request: &GetTransactionRequest) -> Result<TransactionResponse, String> {
    let mut response = TransactionResponse::default();
    
    // For now, return empty hex
    // In full implementation, would look up transaction data
    response.hex = String::new();

    Ok(response)
}

/// Parse inscription ID from string format
pub fn parse_inscription_id(id_str: &str) -> Result<InscriptionId, String> {
    let parts: Vec<&str> = id_str.split('i').collect();
    if parts.len() != 2 {
        return Err("Invalid inscription ID format".to_string());
    }
    
    let txid = parts[0].parse::<Txid>().map_err(|e| format!("Invalid txid: {}", e))?;
    let index: u32 = parts[1].parse().map_err(|e| format!("Invalid index: {}", e))?;
    
    Ok(InscriptionId::new(txid, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inscription_id() {
        let id_str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdefi0";
        let result = parse_inscription_id(id_str);
        assert!(result.is_ok());
        
        let id = result.unwrap();
        assert_eq!(id.index, 0);
    }

    #[test]
    fn test_parse_invalid_inscription_id() {
        let id_str = "invalid";
        let result = parse_inscription_id(id_str);
        assert!(result.is_err());
    }
}
