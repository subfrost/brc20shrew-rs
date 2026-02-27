use shrew_support::inscription::{InscriptionId, InscriptionEntry};
use crate::tables::*;
use crate::proto::{
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
    get_inscription_request,
};
use bitcoin::Txid;
use bitcoin_hashes::Hash;
use metashrew_support::index_pointer::KeyValuePointer;
use std::str::FromStr;

pub fn get_inscription(request: &GetInscriptionRequest) -> Result<InscriptionResponse, String> {
    let query = request.query.as_ref().ok_or("Request must specify a query")?;
    let seq_bytes = match query {
        get_inscription_request::Query::Id(proto_id) => {
            let inscription_id = InscriptionId {
                txid: Txid::from_slice(&proto_id.txid).map_err(|e| e.to_string())?,
                index: proto_id.index,
            };
            INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.to_bytes()).get()
        }
        get_inscription_request::Query::Number(number) => {
            INSCRIPTION_NUMBER_TO_SEQUENCE.select(&number.to_le_bytes().to_vec()).get()
        }
        get_inscription_request::Query::Sat(_) => {
            return Err("Query by sat is not yet implemented".to_string());
        }
    };
    if seq_bytes.is_empty() { return Ok(InscriptionResponse::default()); }
    let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
    if entry_bytes.is_empty() { return Ok(InscriptionResponse::default()); }
    let entry = InscriptionEntry::from_bytes(&entry_bytes)
        .map_err(|e| format!("Failed to parse inscription entry: {}", e))?;
    Ok(InscriptionResponse {
        id: Some(ProtoInscriptionId { txid: entry.id.txid.as_byte_array().to_vec(), index: entry.id.index }),
        number: entry.number,
        content_type: Some(entry.content_type.unwrap_or_default()),
        content_length: entry.content_length,
        timestamp: entry.timestamp as i64,
        satpoint: Some(ProtoSatPoint {
            outpoint: Some(ProtoOutPoint {
                txid: entry.satpoint.outpoint.txid.as_byte_array().to_vec(),
                vout: entry.satpoint.outpoint.vout,
            }),
            offset: entry.satpoint.offset,
        }),
        ..Default::default()
    })
}

pub fn get_inscriptions(request: &GetInscriptionsRequest) -> Result<InscriptionsResponse, String> {
    let mut response = InscriptionsResponse::default();
    let limit = request.pagination.as_ref().map_or(100, |p| p.limit.max(1).min(100));
    let offset = request.pagination.as_ref().map_or(0, |p| p.page * limit);
    let sequence_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    let total = if !sequence_bytes.is_empty() && sequence_bytes.len() >= 4 {
        u32::from_le_bytes([sequence_bytes[0], sequence_bytes[1], sequence_bytes[2], sequence_bytes[3]]) as u64
    } else { 0 };
    let mut inscription_ids = Vec::new();
    let start_seq = offset + 1;
    let end_seq = (start_seq + limit).min((total + 1) as u32);
    for seq in start_seq..end_seq {
        let seq_bytes = (seq as u32).to_le_bytes().to_vec();
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
        if !entry_bytes.is_empty() {
            if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                inscription_ids.push(ProtoInscriptionId {
                    txid: entry.id.txid.as_byte_array().to_vec(),
                    index: entry.id.index,
                });
            }
        }
    }
    response.ids = inscription_ids;
    response.pagination = Some(crate::proto::PaginationResponse {
        limit, page: offset / limit, total, more: (offset + limit) < (total as u32),
    });
    Ok(response)
}

pub fn get_children(request: &GetChildrenRequest) -> Result<ChildrenResponse, String> {
    let mut response = ChildrenResponse::default();
    let parent_proto_id = request.parent_id.as_ref().ok_or("Missing parent_id")?;
    let parent_id = InscriptionId {
        txid: Txid::from_slice(&parent_proto_id.txid).map_err(|e| e.to_string())?,
        index: parent_proto_id.index,
    };
    let parent_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id.to_bytes()).get();
    if parent_seq_bytes.is_empty() { return Ok(response); }
    let children_seq_list = SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).get_list();
    let mut children_ids = Vec::new();
    for child_seq_bytes in children_seq_list {
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&child_seq_bytes).get();
        if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
            children_ids.push(ProtoInscriptionId { txid: entry.id.txid.as_byte_array().to_vec(), index: entry.id.index });
        }
    }
    response.ids = children_ids;
    Ok(response)
}

pub fn get_parents(request: &GetParentsRequest) -> Result<ParentsResponse, String> {
    let mut response = ParentsResponse::default();
    let child_proto_id = request.child_id.as_ref().ok_or("Missing child_id")?;
    let child_id = InscriptionId {
        txid: Txid::from_slice(&child_proto_id.txid).map_err(|e| e.to_string())?,
        index: child_proto_id.index,
    };
    let child_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&child_id.to_bytes()).get();
    if child_seq_bytes.is_empty() { return Ok(response); }
    let parents_seq_list = SEQUENCE_TO_PARENTS.select(&child_seq_bytes).get_list();
    let mut parent_ids = Vec::new();
    for parent_seq_bytes in parents_seq_list {
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&parent_seq_bytes).get();
        if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
            parent_ids.push(ProtoInscriptionId { txid: entry.id.txid.as_byte_array().to_vec(), index: entry.id.index });
        }
    }
    response.ids = parent_ids;
    Ok(response)
}

pub fn get_content(request: &GetContentRequest) -> Result<ContentResponse, String> {
    let mut response = ContentResponse::default();
    let proto_id = request.id.as_ref().ok_or("Missing id")?;
    let inscription_id = InscriptionId {
        txid: Txid::from_slice(&proto_id.txid).map_err(|e| e.to_string())?,
        index: proto_id.index,
    };
    let id_bytes = inscription_id.to_bytes();
    let seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&id_bytes).get();
    if seq_bytes.is_empty() { return Ok(response); }
    let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq_bytes).get();
    if entry_bytes.is_empty() { return Ok(response); }
    let entry = InscriptionEntry::from_bytes(&entry_bytes)
        .map_err(|e| format!("Failed to parse inscription entry: {}", e))?;
    if let Some(delegate_id) = entry.delegate {
        let delegate_proto_id = ProtoInscriptionId { txid: delegate_id.txid.as_byte_array().to_vec(), index: delegate_id.index };
        return get_content(&GetContentRequest { id: Some(delegate_proto_id) });
    }
    let inscription_id_str = inscription_id.to_string();
    let content_table = InscriptionContentTable::new();
    if let Some(content) = content_table.get(&inscription_id_str) { response.content = content; }
    response.content_type = Some(entry.content_type.unwrap_or_default());
    Ok(response)
}

pub fn get_metadata(request: &GetMetadataRequest) -> Result<MetadataResponse, String> {
    let mut response = MetadataResponse::default();
    let proto_id = request.id.as_ref().ok_or("Missing id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid).map_err(|e| format!("Invalid txid: {}", e))?;
    let inscription_id_str = format!("{}i{}", txid, proto_id.index);
    let metadata_table = InscriptionMetadataTable::new();
    if let Some(metadata) = metadata_table.get(&inscription_id_str) { response.metadata_hex = hex::encode(metadata); }
    Ok(response)
}

pub fn get_sat(_request: &GetSatRequest) -> Result<SatResponse, String> {
    let mut response = SatResponse::default();
    response.number = _request.sat;
    Ok(response)
}

pub fn get_sat_inscriptions(_request: &GetSatInscriptionsRequest) -> Result<SatInscriptionsResponse, String> {
    Ok(SatInscriptionsResponse::default())
}

pub fn get_sat_inscription(_request: &GetSatInscriptionRequest) -> Result<SatInscriptionResponse, String> {
    Ok(SatInscriptionResponse::default())
}

pub fn get_child_inscriptions(request: &GetChildInscriptionsRequest) -> Result<ChildInscriptionsResponse, String> {
    let mut response = ChildInscriptionsResponse::default();
    let parent_proto_id = request.parent_id.as_ref().ok_or("Missing parent_id")?;
    let parent_id = InscriptionId {
        txid: Txid::from_slice(&parent_proto_id.txid).map_err(|e| e.to_string())?,
        index: parent_proto_id.index,
    };
    let parent_seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&parent_id.to_bytes()).get();
    if parent_seq_bytes.is_empty() { return Ok(response); }
    let children_seq_list = SEQUENCE_TO_CHILDREN.select(&parent_seq_bytes).get_list();
    let mut children_info = Vec::new();
    for child_seq_bytes in children_seq_list {
        let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&child_seq_bytes).get();
        if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
            children_info.push(crate::proto::RelativeInscription {
                id: Some(ProtoInscriptionId { txid: entry.id.txid.as_byte_array().to_vec(), index: entry.id.index }),
                number: entry.number,
                ..Default::default()
            });
        }
    }
    response.children = children_info;
    Ok(response)
}

pub fn get_parent_inscriptions(request: &GetParentInscriptionsRequest) -> Result<ParentInscriptionsResponse, String> {
    let mut response = ParentInscriptionsResponse::default();
    let proto_id = request.child_id.as_ref().ok_or("Missing child_id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid).map_err(|e| format!("Invalid txid: {}", e))?;
    let child_id_str = format!("{}i{}", txid, proto_id.index);
    let parent_table = InscriptionParentTable::new();
    if let Some(parent_id_str) = parent_table.get(&child_id_str) {
        let mut relative = crate::proto::RelativeInscription::default();
        let parts: Vec<&str> = parent_id_str.split('i').collect();
        if parts.len() == 2 {
            if let Ok(parent_txid) = bitcoin::Txid::from_str(parts[0]) {
                if let Ok(parent_index) = parts[1].parse::<u32>() {
                    relative.id = Some(ProtoInscriptionId { txid: parent_txid.as_byte_array().to_vec(), index: parent_index });
                }
            }
        }
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

pub fn get_undelegated_content(request: &GetUndelegatedContentRequest) -> Result<UndelegatedContentResponse, String> {
    let mut response = UndelegatedContentResponse::default();
    let proto_id = request.id.as_ref().ok_or("Missing id")?;
    let txid = bitcoin::Txid::from_slice(&proto_id.txid).map_err(|e| format!("Invalid txid: {}", e))?;
    let inscription_id_str = format!("{}i{}", txid, proto_id.index);
    let content_table = InscriptionContentTable::new();
    if let Some(content) = content_table.get(&inscription_id_str) { response.content = content; }
    let content_type_table = InscriptionContentTypeTable::new();
    if let Some(content_type_bytes) = content_type_table.get(&inscription_id_str) {
        if let Ok(content_type) = String::from_utf8(content_type_bytes) {
            response.content_type = Some(content_type);
        }
    }
    Ok(response)
}

pub fn get_utxo(_request: &GetUtxoRequest) -> Result<UtxoResponse, String> {
    Ok(UtxoResponse::default())
}

pub fn get_block_hash(request: &GetBlockHashRequest) -> Result<BlockHashResponse, String> {
    let mut response = BlockHashResponse::default();
    if let Some(height) = request.height {
        let height_bytes = height.to_le_bytes().to_vec();
        let hash_bytes = HEIGHT_TO_BLOCK_HASH.select(&height_bytes).get();
        if !hash_bytes.is_empty() && hash_bytes.len() == 32 {
            let hash = bitcoin::BlockHash::from_byte_array(hash_bytes[..32].try_into().unwrap_or([0u8; 32]));
            response.hash = hash.to_string();
        }
    }
    Ok(response)
}

pub fn get_block_height(_request: &GetBlockHeightRequest) -> Result<BlockHeightResponse, String> {
    let mut response = BlockHeightResponse::default();
    let sequence_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    if !sequence_bytes.is_empty() && sequence_bytes.len() >= 4 {
        let height = u32::from_le_bytes([sequence_bytes[0], sequence_bytes[1], sequence_bytes[2], sequence_bytes[3]]);
        response.height = height;
    }
    Ok(response)
}

pub fn get_block_time(_request: &GetBlockTimeRequest) -> Result<BlockTimeResponse, String> {
    Ok(BlockTimeResponse { timestamp: 1640995200 })
}

pub fn get_block_info(request: &GetBlockInfoRequest) -> Result<BlockInfoResponse, String> {
    use crate::proto::get_block_info_request::Query;
    let mut response = BlockInfoResponse::default();
    if let Some(query) = &request.query {
        match query {
            Query::Height(height) => {
                response.height = *height;
                let height_bytes = height.to_le_bytes().to_vec();
                let hash_bytes = HEIGHT_TO_BLOCK_HASH.select(&height_bytes).get();
                if !hash_bytes.is_empty() && hash_bytes.len() == 32 {
                    let hash = bitcoin::BlockHash::from_byte_array(hash_bytes[..32].try_into().unwrap_or([0u8; 32]));
                    response.hash = hash.to_string();
                }
            }
            Query::Hash(hash_str) => {
                response.hash = hash_str.clone();
                if let Ok(hash) = bitcoin::BlockHash::from_str(hash_str) {
                    let hash_bytes = hash.as_byte_array().to_vec();
                    let height_bytes = BLOCK_HASH_TO_HEIGHT.select(&hash_bytes).get();
                    if !height_bytes.is_empty() && height_bytes.len() >= 4 {
                        response.height = u32::from_le_bytes([height_bytes[0], height_bytes[1], height_bytes[2], height_bytes[3]]);
                    }
                }
            }
        }
    } else {
        return Err("No query parameter provided".to_string());
    }
    Ok(response)
}

pub fn get_tx(_request: &GetTransactionRequest) -> Result<TransactionResponse, String> {
    Ok(TransactionResponse { hex: String::new() })
}

pub fn parse_inscription_id(id_str: &str) -> Result<InscriptionId, String> {
    id_str.parse()
}
