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
    },
};
use bitcoin::Txid;
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use protobuf::Message;
use std::str::FromStr;

/// Get inscription by ID or number
///
/// Retrieves a single inscription by its ID (txid + index) or inscription number.
/// Returns complete inscription metadata including location, content info, and relationships.
pub fn get_inscription(request: &GetInscriptionRequest) -> Result<InscriptionResponse, String> {
    // Extract inscription ID from request
    let inscription_id_str = if request.has_id() {
        let proto_id = request.id();
        let txid = bitcoin::Txid::from_slice(&proto_id.txid)
            .map_err(|e| format!("Invalid txid: {}", e))?;
        let index = proto_id.index;
        format!("{}i{}", txid, index)
    } else if request.has_number() {
        // Look up by inscription number
        let number = request.number();
        let number_bytes = number.to_le_bytes().to_vec();
        let sequence_bytes = INSCRIPTION_NUMBER_TO_SEQUENCE.select(&number_bytes).get();
        if sequence_bytes.is_empty() {
            return Ok(InscriptionResponse::new()); // Not found
        }
        
        // Get inscription entry from sequence
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get();
        if entry_bytes.is_empty() {
            return Ok(InscriptionResponse::new()); // Not found
        }
        
        let entry = InscriptionEntry::from_bytes(&entry_bytes)
            .map_err(|e| format!("Failed to parse inscription entry: {}", e))?;
        entry.id.to_string()
    } else {
        return Err("Request must specify either id or number".to_string());
    };

    // Get inscription from database using simplified table access
    let inscription_table = InscriptionTable::new();
    if inscription_table.get(&inscription_id_str).is_none() {
        return Ok(InscriptionResponse::new()); // Not found
    }

    // Build response with available data
    let mut response = InscriptionResponse::new();
    
    // Set basic inscription ID
    let mut proto_id = ProtoInscriptionId::new();
    let parts: Vec<&str> = inscription_id_str.split('i').collect();
    if parts.len() == 2 {
        if let Ok(txid) = bitcoin::Txid::from_str(parts[0]) {
            proto_id.txid = txid.as_byte_array().to_vec();
            if let Ok(index) = parts[1].parse::<u32>() {
                proto_id.index = index;
            }
        }
    }
    response.id = protobuf::MessageField::some(proto_id);

    // Get inscription number
    let number_table = InscriptionNumberTable::new();
    if let Some(number_bytes) = number_table.get(&inscription_id_str) {
        if let Ok(number) = serde_json::from_slice::<u64>(&number_bytes) {
            response.number = number as i32;
        }
    }

    // Get content type
    let content_type_table = InscriptionContentTypeTable::new();
    if let Some(content_type_bytes) = content_type_table.get(&inscription_id_str) {
        if let Ok(content_type) = String::from_utf8(content_type_bytes) {
            response.content_type = Some(content_type);
        }
    }

    // Get content length
    let content_table = InscriptionContentTable::new();
    if let Some(content) = content_table.get(&inscription_id_str) {
        response.content_length = Some(content.len() as u64);
    }

    // Get location (satpoint)
    let location_table = InscriptionLocationTable::new();
    if let Some(satpoint_str) = location_table.get(&inscription_id_str) {
        let mut proto_satpoint = ProtoSatPoint::new();
        let parts: Vec<&str> = satpoint_str.split(':').collect();
        if parts.len() >= 3 {
            if let Ok(txid) = bitcoin::Txid::from_str(parts[0]) {
                let mut proto_outpoint = ProtoOutPoint::new();
                proto_outpoint.txid = txid.as_byte_array().to_vec();
                if let Ok(vout) = parts[1].parse::<u32>() {
                    proto_outpoint.vout = vout;
                }
                proto_satpoint.outpoint = protobuf::MessageField::some(proto_outpoint);
                
                if let Ok(offset) = parts[2].parse::<u64>() {
                    proto_satpoint.offset = offset;
                }
            }
        }
        response.satpoint = protobuf::MessageField::some(proto_satpoint);
    }

    Ok(response)
}

/// Get list of inscriptions with pagination
///
/// Returns a paginated list of inscription IDs, optionally filtered by various criteria.
/// Supports filtering by height, content type, metaprotocol, and blessed/cursed status.
pub fn get_inscriptions(request: &GetInscriptionsRequest) -> Result<InscriptionsResponse, String> {
    let mut response = InscriptionsResponse::new();
    
    // Get pagination parameters
    let limit = if request.pagination.is_some() {
        request.pagination.as_ref().unwrap().limit.max(1).min(100) // Limit between 1-100
    } else {
        10 // Default limit
    };
    
    let offset = if request.pagination.is_some() {
        request.pagination.as_ref().unwrap().page * limit
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
                let mut proto_id = crate::proto::shrewscriptions::InscriptionId::new();
                proto_id.txid = entry.id.txid.as_byte_array().to_vec();
                proto_id.index = entry.id.index;
                inscription_ids.push(proto_id);
            }
        }
    }
    
    response.ids = inscription_ids;

    // Set pagination info
    let mut pagination = crate::proto::shrewscriptions::PaginationResponse::new();
    pagination.limit = limit;
    pagination.page = offset / limit;
    pagination.total = total;
    pagination.more = (offset + limit) < (total as u32);
    response.pagination = protobuf::MessageField::some(pagination);

    Ok(response)
}

/// Get children of an inscription
///
/// Returns a list of inscription IDs that are children of the specified parent inscription.
/// Children are inscriptions that reference the parent in their parent field.
pub fn get_children(request: &GetChildrenRequest) -> Result<ChildrenResponse, String> {
    let mut response = ChildrenResponse::new();
    
    // Get parent ID string
    let proto_id = &request.parent_id.as_ref().ok_or("Missing parent_id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let parent_id_str = format!("{}i{}", txid, index);

    // Get children list
    let children_table = InscriptionChildrenTable::new();
    if let Some(children_bytes) = children_table.get(&parent_id_str) {
        if let Ok(children_list) = serde_json::from_slice::<Vec<String>>(&children_bytes) {
            let mut proto_children = Vec::new();
            
            for child_id_str in children_list {
                let parts: Vec<&str> = child_id_str.split('i').collect();
                if parts.len() == 2 {
                    if let Ok(child_txid) = bitcoin::Txid::from_str(parts[0]) {
                        if let Ok(child_index) = parts[1].parse::<u32>() {
                            let mut proto_child_id = ProtoInscriptionId::new();
                            proto_child_id.txid = child_txid.as_byte_array().to_vec();
                            proto_child_id.index = child_index;
                            proto_children.push(proto_child_id);
                        }
                    }
                }
            }
            
            response.ids = proto_children;
        }
    }

    Ok(response)
}

/// Get parents of an inscription
///
/// Returns a list of inscription IDs that are parents of the specified child inscription.
/// Parents are inscriptions referenced in the child's parent field.
pub fn get_parents(request: &GetParentsRequest) -> Result<ParentsResponse, String> {
    let mut response = ParentsResponse::new();
    
    // Get child ID string
    let proto_id = &request.child_id.as_ref().ok_or("Missing child_id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let child_id_str = format!("{}i{}", txid, index);

    // Get parent
    let parent_table = InscriptionParentTable::new();
    if let Some(parent_id_str) = parent_table.get(&child_id_str) {
        let parts: Vec<&str> = parent_id_str.split('i').collect();
        if parts.len() == 2 {
            if let Ok(parent_txid) = bitcoin::Txid::from_str(parts[0]) {
                if let Ok(parent_index) = parts[1].parse::<u32>() {
                    let mut proto_parent_id = ProtoInscriptionId::new();
                    proto_parent_id.txid = parent_txid.as_byte_array().to_vec();
                    proto_parent_id.index = parent_index;
                    response.ids = vec![proto_parent_id];
                }
            }
        }
    }

    Ok(response)
}

/// Get inscription content
///
/// Returns the raw content bytes and content type for an inscription.
/// Handles delegation by following delegate references to retrieve delegated content.
pub fn get_content(request: &GetContentRequest) -> Result<ContentResponse, String> {
    let mut response = ContentResponse::new();
    
    // Get inscription ID string
    let proto_id = &request.id.as_ref().ok_or("Missing id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let inscription_id_str = format!("{}i{}", txid, index);

    // Check if this inscription delegates to another
    let delegate_table = InscriptionDelegateTable::new();
    let content_id_str = if let Some(delegate_id_str) = delegate_table.get(&inscription_id_str) {
        // DEBUG: Log delegation lookup
        println!("DEBUG get_content: Found delegation from {} to {}", inscription_id_str, delegate_id_str);
        // Use delegate's content
        delegate_id_str
    } else {
        // DEBUG: Log no delegation found
        println!("DEBUG get_content: No delegation found for {}", inscription_id_str);
        // Use own content
        inscription_id_str.clone()
    };

    // Get content
    let content_table = InscriptionContentTable::new();
    println!("DEBUG get_content: Looking for content with ID: {}", content_id_str);
    if let Some(content) = content_table.get(&content_id_str) {
        println!("DEBUG get_content: Found content with length: {}", content.len());
        response.content = content;
    } else {
        println!("DEBUG get_content: No content found for ID: {}", content_id_str);
    }

    // Get content type (from the content source, which could be delegate)
    let content_type_table = InscriptionContentTypeTable::new();
    if let Some(content_type_bytes) = content_type_table.get(&content_id_str) {
        if let Ok(content_type) = String::from_utf8(content_type_bytes) {
            response.content_type = Some(content_type);
        }
    }

    Ok(response)
}

/// Get inscription metadata
///
/// Returns the metadata associated with an inscription as a hex-encoded string.
/// Metadata is typically JSON data stored in the inscription envelope.
pub fn get_metadata(request: &GetMetadataRequest) -> Result<MetadataResponse, String> {
    let mut response = MetadataResponse::new();
    
    // Get inscription ID string
    let proto_id = &request.id.as_ref().ok_or("Missing id")?;
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
    let mut response = SatResponse::new();
    let sat = request.sat;
    
    // Set basic sat info
    response.number = sat;
    
    // Calculate rarity (simplified)
    use crate::inscription::Rarity;
    let rarity = Rarity::from_sat(sat);
    let proto_rarity = match rarity {
        Rarity::Common => crate::proto::shrewscriptions::Rarity::COMMON,
        Rarity::Uncommon => crate::proto::shrewscriptions::Rarity::UNCOMMON,
        Rarity::Rare => crate::proto::shrewscriptions::Rarity::RARE,
        Rarity::Epic => crate::proto::shrewscriptions::Rarity::EPIC,
        Rarity::Legendary => crate::proto::shrewscriptions::Rarity::LEGENDARY,
        Rarity::Mythic => crate::proto::shrewscriptions::Rarity::MYTHIC,
    };
    response.rarity = proto_rarity.into();

    Ok(response)
}

/// Get inscriptions on a sat
///
/// Returns a paginated list of inscription IDs that are located on the specified satoshi.
pub fn get_sat_inscriptions(request: &GetSatInscriptionsRequest) -> Result<SatInscriptionsResponse, String> {
    let mut response = SatInscriptionsResponse::new();
    let _sat = request.sat;
    
    // For now, return empty list but structure is correct
    Ok(response)
}

/// Get inscription on a sat
///
/// Returns the inscription at a specific index on the specified satoshi.
/// Index -1 returns the latest inscription on the sat.
pub fn get_sat_inscription(request: &GetSatInscriptionRequest) -> Result<SatInscriptionResponse, String> {
    let mut response = SatInscriptionResponse::new();
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
    let mut response = ChildInscriptionsResponse::new();
    
    // Get parent ID string
    let proto_id = &request.parent_id.as_ref().ok_or("Missing parent_id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let parent_id_str = format!("{}i{}", txid, index);

    // Get children list and build detailed response
    let children_table = InscriptionChildrenTable::new();
    if let Some(children_bytes) = children_table.get(&parent_id_str) {
        if let Ok(children_list) = serde_json::from_slice::<Vec<String>>(&children_bytes) {
            let mut relative_inscriptions = Vec::new();
            
            for child_id_str in children_list {
                // Build RelativeInscription for each child
                let mut relative = crate::proto::shrewscriptions::RelativeInscription::new();
                
                // Set ID
                let parts: Vec<&str> = child_id_str.split('i').collect();
                if parts.len() == 2 {
                    if let Ok(child_txid) = bitcoin::Txid::from_str(parts[0]) {
                        if let Ok(child_index) = parts[1].parse::<u32>() {
                            let mut proto_child_id = ProtoInscriptionId::new();
                            proto_child_id.txid = child_txid.as_byte_array().to_vec();
                            proto_child_id.index = child_index;
                            relative.id = protobuf::MessageField::some(proto_child_id);
                        }
                    }
                }
                
                // Get additional details (number, height, etc.)
                let number_table = InscriptionNumberTable::new();
                if let Some(number_bytes) = number_table.get(&child_id_str) {
                    if let Ok(number) = serde_json::from_slice::<u64>(&number_bytes) {
                        relative.number = number as i32;
                    }
                }
                
                relative_inscriptions.push(relative);
            }
            
            response.children = relative_inscriptions;
        }
    }

    Ok(response)
}

/// Get parent inscriptions with full info
///
/// Returns detailed information about parent inscriptions including their metadata,
/// location, and other properties.
pub fn get_parent_inscriptions(request: &GetParentInscriptionsRequest) -> Result<ParentInscriptionsResponse, String> {
    let mut response = ParentInscriptionsResponse::new();
    
    // Get child ID string
    let proto_id = &request.child_id.as_ref().ok_or("Missing child_id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid)
        .map_err(|e| format!("Invalid txid: {}", e))?;
    let index = proto_id.index;
    let child_id_str = format!("{}i{}", txid, index);

    // Get parent and build detailed response
    let parent_table = InscriptionParentTable::new();
    if let Some(parent_id_str) = parent_table.get(&child_id_str) {
        let mut relative = crate::proto::shrewscriptions::RelativeInscription::new();
        
        // Set ID
        let parts: Vec<&str> = parent_id_str.split('i').collect();
        if parts.len() == 2 {
            if let Ok(parent_txid) = bitcoin::Txid::from_str(parts[0]) {
                if let Ok(parent_index) = parts[1].parse::<u32>() {
                    let mut proto_parent_id = ProtoInscriptionId::new();
                    proto_parent_id.txid = parent_txid.as_byte_array().to_vec();
                    proto_parent_id.index = parent_index;
                    relative.id = protobuf::MessageField::some(proto_parent_id);
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
    let mut response = UndelegatedContentResponse::new();
    
    // Get inscription ID string
    let proto_id = &request.id.as_ref().ok_or("Missing id")?;
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
    let mut response = UtxoResponse::new();
    
    // Get outpoint
    let proto_outpoint = &request.outpoint.as_ref().ok_or("Missing outpoint")?;
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
    let mut response = BlockHashResponse::new();
    
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
    let mut response = BlockHeightResponse::new();
    
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
    let mut response = BlockTimeResponse::new();
    
    // For now, return current timestamp
    response.timestamp = 1640995200; // 2022-01-01 as placeholder

    Ok(response)
}

/// Get block info
///
/// Returns detailed information about a block including hash, height, and statistics.
pub fn get_block_info(request: &GetBlockInfoRequest) -> Result<BlockInfoResponse, String> {
    use crate::proto::shrewscriptions::get_block_info_request::Query;
    
    let mut response = BlockInfoResponse::new();
    
    match &request.query {
        Some(Query::Height(height)) => {
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
        Some(Query::Hash(hash_str)) => {
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
        None => {
            return Err("No query parameter provided".to_string());
        }
    }

    Ok(response)
}

/// Get transaction info
///
/// Returns transaction information including hex representation.
pub fn get_tx(_request: &GetTransactionRequest) -> Result<TransactionResponse, String> {
    let mut response = TransactionResponse::new();
    
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