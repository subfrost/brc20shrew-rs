use crate::{
    inscription::{InscriptionEntry, InscriptionId, SatPoint},
    proto::shrewscriptions::{
        BlockHashAtHeightRequest, BlockHashAtHeightResponse, BlockHashRequest, BlockHashResponse,
        BlockHeightRequest, BlockHeightResponse, BlockInfoRequest, BlockInfoResponse,
        BlockTimeRequest, BlockTimeResponse, ChildInscriptionsRequest, ChildInscriptionsResponse,
        ChildrenRequest, ChildrenResponse, ContentRequest, ContentResponse, InscriptionRequest,
        InscriptionResponse, InscriptionsRequest, InscriptionsResponse, MetadataRequest,
        MetadataResponse, ParentInscriptionsRequest, ParentInscriptionsResponse, ParentsRequest,
        ParentsResponse, SatInscriptionContentRequest, SatInscriptionContentResponse,
        SatInscriptionRequest, SatInscriptionResponse, SatInscriptionsRequest,
        SatInscriptionsResponse, SatRequest, SatResponse, TxRequest, TxResponse,
        UndelegatedContentRequest, UndelegatedContentResponse, UtxoRequest, UtxoResponse,
        InscriptionInfo, SatInfo, BlockInfo, UtxoInfo, TxInfo,
    },
    tables::TABLES,
};
use bitcoin::{BlockHash, Txid};
use protobuf::RepeatedField;

/// Get inscription by ID or number
pub fn get_inscription(request: &InscriptionRequest) -> Result<InscriptionResponse, String> {
    let mut response = InscriptionResponse::new();

    let sequence_bytes = if request.has_id() {
        // Look up by inscription ID
        let id_str = request.get_id();
        let inscription_id = parse_inscription_id(id_str)?;
        let id_bytes = inscription_id.to_bytes();
        
        TABLES.INSCRIPTION_ID_TO_SEQUENCE
            .select(&id_bytes)
            .get()
            .ok_or_else(|| format!("Inscription not found: {}", id_str))?
    } else if request.has_number() {
        // Look up by inscription number
        let number = request.get_number();
        let number_bytes = number.to_le_bytes();
        
        TABLES.INSCRIPTION_NUMBER_TO_SEQUENCE
            .select(&number_bytes)
            .get()
            .ok_or_else(|| format!("Inscription not found: {}", number))?
    } else {
        return Err("Either id or number must be specified".to_string());
    };

    // Get inscription entry
    let entry_bytes = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY
        .select(&sequence_bytes)
        .get()
        .ok_or("Inscription entry not found")?;
    
    let entry = InscriptionEntry::from_bytes(&entry_bytes)
        .map_err(|e| format!("Failed to parse inscription entry: {}", e))?;

    // Convert to protobuf
    let mut info = InscriptionInfo::new();
    info.set_id(entry.id.to_string());
    info.set_number(entry.number);
    info.set_sequence(entry.sequence);
    info.set_height(entry.height);
    info.set_fee(entry.fee);
    info.set_timestamp(entry.timestamp);
    info.set_genesis_fee(entry.genesis_fee);
    info.set_genesis_height(entry.genesis_height);
    info.set_charms(entry.charms as u32);
    
    if let Some(sat) = entry.sat {
        info.set_sat(sat);
    }
    
    if let Some(content_type) = &entry.content_type {
        info.set_content_type(content_type.clone());
    }
    
    if let Some(content_length) = entry.content_length {
        info.set_content_length(content_length);
    }
    
    if let Some(metaprotocol) = &entry.metaprotocol {
        info.set_metaprotocol(metaprotocol.clone());
    }
    
    if let Some(parent) = &entry.parent {
        info.set_parent(parent.to_string());
    }
    
    if let Some(delegate) = &entry.delegate {
        info.set_delegate(delegate.to_string());
    }
    
    if let Some(pointer) = entry.pointer {
        info.set_pointer(pointer);
    }

    response.set_inscription(info);
    Ok(response)
}

/// Get list of inscriptions with pagination
pub fn get_inscriptions(request: &InscriptionsRequest) -> Result<InscriptionsResponse, String> {
    let mut response = InscriptionsResponse::new();
    let mut inscriptions = Vec::new();

    let limit = if request.has_limit() {
        request.get_limit() as usize
    } else {
        100
    };

    let offset = if request.has_offset() {
        request.get_offset() as usize
    } else {
        0
    };

    // Get inscriptions by different criteria
    let sequence_list = if request.has_height() {
        // Get inscriptions at specific height
        let height = request.get_height();
        let height_bytes = height.to_le_bytes();
        
        TABLES.HEIGHT_TO_INSCRIPTIONS
            .select(&height_bytes)
            .get_list()
            .unwrap_or_default()
    } else if request.has_content_type() {
        // Get inscriptions by content type
        let content_type = request.get_content_type();
        
        TABLES.CONTENT_TYPE_TO_INSCRIPTIONS
            .select(content_type.as_bytes())
            .get_list()
            .unwrap_or_default()
    } else if request.has_metaprotocol() {
        // Get inscriptions by metaprotocol
        let metaprotocol = request.get_metaprotocol();
        
        TABLES.METAPROTOCOL_TO_INSCRIPTIONS
            .select(metaprotocol.as_bytes())
            .get_list()
            .unwrap_or_default()
    } else {
        // Get all inscriptions (by sequence)
        let mut sequences = Vec::new();
        let counter_bytes = TABLES.GLOBAL_SEQUENCE_COUNTER.get().unwrap_or_default();
        
        if counter_bytes.len() >= 4 {
            let max_sequence = u32::from_le_bytes([
                counter_bytes[0], counter_bytes[1], counter_bytes[2], counter_bytes[3]
            ]);
            
            for seq in 1..=max_sequence {
                sequences.extend_from_slice(&seq.to_le_bytes());
            }
        }
        
        sequences
    };

    // Apply pagination
    let total_count = sequence_list.len() / 4; // Each sequence is 4 bytes
    let start_idx = offset * 4;
    let end_idx = std::cmp::min(start_idx + (limit * 4), sequence_list.len());

    if start_idx < sequence_list.len() {
        for chunk in sequence_list[start_idx..end_idx].chunks(4) {
            if chunk.len() == 4 {
                let sequence_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                
                if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
                    if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                        let mut info = InscriptionInfo::new();
                        info.set_id(entry.id.to_string());
                        info.set_number(entry.number);
                        info.set_sequence(entry.sequence);
                        info.set_height(entry.height);
                        info.set_fee(entry.fee);
                        info.set_timestamp(entry.timestamp);
                        info.set_genesis_fee(entry.genesis_fee);
                        info.set_genesis_height(entry.genesis_height);
                        info.set_charms(entry.charms as u32);
                        
                        if let Some(sat) = entry.sat {
                            info.set_sat(sat);
                        }
                        
                        if let Some(content_type) = &entry.content_type {
                            info.set_content_type(content_type.clone());
                        }
                        
                        if let Some(content_length) = entry.content_length {
                            info.set_content_length(content_length);
                        }
                        
                        if let Some(metaprotocol) = &entry.metaprotocol {
                            info.set_metaprotocol(metaprotocol.clone());
                        }
                        
                        if let Some(parent) = &entry.parent {
                            info.set_parent(parent.to_string());
                        }
                        
                        if let Some(delegate) = &entry.delegate {
                            info.set_delegate(delegate.to_string());
                        }
                        
                        if let Some(pointer) = entry.pointer {
                            info.set_pointer(pointer);
                        }

                        inscriptions.push(info);
                    }
                }
            }
        }
    }

    response.set_inscriptions(RepeatedField::from_vec(inscriptions));
    response.set_total(total_count as u32);
    response.set_offset(offset as u32);
    response.set_limit(limit as u32);

    Ok(response)
}

/// Get children of an inscription
pub fn get_children(request: &ChildrenRequest) -> Result<ChildrenResponse, String> {
    let mut response = ChildrenResponse::new();

    let inscription_id = parse_inscription_id(request.get_id())?;
    let id_bytes = inscription_id.to_bytes();

    // Get sequence for this inscription
    let sequence_bytes = TABLES.INSCRIPTION_ID_TO_SEQUENCE
        .select(&id_bytes)
        .get()
        .ok_or("Inscription not found")?;

    // Get children sequences
    let children_list = TABLES.SEQUENCE_TO_CHILDREN
        .select(&sequence_bytes)
        .get_list()
        .unwrap_or_default();

    let mut children_ids = Vec::new();
    for chunk in children_list.chunks(4) {
        if chunk.len() == 4 {
            let child_sequence_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            
            if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&child_sequence_bytes).get() {
                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    children_ids.push(entry.id.to_string());
                }
            }
        }
    }

    response.set_children(RepeatedField::from_vec(children_ids));
    Ok(response)
}

/// Get parents of an inscription
pub fn get_parents(request: &ParentsRequest) -> Result<ParentsResponse, String> {
    let mut response = ParentsResponse::new();

    let inscription_id = parse_inscription_id(request.get_id())?;
    let id_bytes = inscription_id.to_bytes();

    // Get sequence for this inscription
    let sequence_bytes = TABLES.INSCRIPTION_ID_TO_SEQUENCE
        .select(&id_bytes)
        .get()
        .ok_or("Inscription not found")?;

    // Get parents sequences
    let parents_list = TABLES.SEQUENCE_TO_PARENTS
        .select(&sequence_bytes)
        .get_list()
        .unwrap_or_default();

    let mut parent_ids = Vec::new();
    for chunk in parents_list.chunks(4) {
        if chunk.len() == 4 {
            let parent_sequence_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            
            if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&parent_sequence_bytes).get() {
                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    parent_ids.push(entry.id.to_string());
                }
            }
        }
    }

    response.set_parents(RepeatedField::from_vec(parent_ids));
    Ok(response)
}

/// Get inscription content
pub fn get_content(request: &ContentRequest) -> Result<ContentResponse, String> {
    let mut response = ContentResponse::new();

    let inscription_id = parse_inscription_id(request.get_id())?;
    let id_bytes = inscription_id.to_bytes();

    // Get sequence for this inscription
    let sequence_bytes = TABLES.INSCRIPTION_ID_TO_SEQUENCE
        .select(&id_bytes)
        .get()
        .ok_or("Inscription not found")?;

    // Get content
    if let Some(content) = TABLES.INSCRIPTION_CONTENT.select(&sequence_bytes).get() {
        response.set_content(content);
    }

    // Get content type from inscription entry
    if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
        if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
            if let Some(content_type) = entry.content_type {
                response.set_content_type(content_type);
            }
        }
    }

    Ok(response)
}

/// Get inscription metadata
pub fn get_metadata(request: &MetadataRequest) -> Result<MetadataResponse, String> {
    let mut response = MetadataResponse::new();

    let inscription_id = parse_inscription_id(request.get_id())?;
    let id_bytes = inscription_id.to_bytes();

    // Get sequence for this inscription
    let sequence_bytes = TABLES.INSCRIPTION_ID_TO_SEQUENCE
        .select(&id_bytes)
        .get()
        .ok_or("Inscription not found")?;

    // Get metadata
    if let Some(metadata) = TABLES.INSCRIPTION_METADATA.select(&sequence_bytes).get() {
        response.set_metadata(metadata);
    }

    Ok(response)
}

/// Get sat information
pub fn get_sat(request: &SatRequest) -> Result<SatResponse, String> {
    let mut response = SatResponse::new();
    let sat = request.get_sat();

    let mut info = SatInfo::new();
    info.set_sat(sat);
    
    // Calculate rarity
    let rarity = crate::inscription::Rarity::from_sat(sat);
    info.set_rarity(rarity.name().to_string());

    // Check if sat has inscriptions
    let sat_bytes = sat.to_le_bytes();
    if let Some(sequence_bytes) = TABLES.SAT_TO_SEQUENCE.select(&sat_bytes).get() {
        if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
            if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                info.set_inscription_id(entry.id.to_string());
            }
        }
    }

    response.set_sat_info(info);
    Ok(response)
}

/// Get inscriptions on a sat
pub fn get_sat_inscriptions(request: &SatInscriptionsRequest) -> Result<SatInscriptionsResponse, String> {
    let mut response = SatInscriptionsResponse::new();
    let sat = request.get_sat();

    let sat_bytes = sat.to_le_bytes();
    let inscriptions_list = TABLES.SAT_TO_INSCRIPTIONS
        .select(&sat_bytes)
        .get_list()
        .unwrap_or_default();

    let mut inscription_ids = Vec::new();
    for chunk in inscriptions_list.chunks(4) {
        if chunk.len() == 4 {
            let sequence_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            
            if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    inscription_ids.push(entry.id.to_string());
                }
            }
        }
    }

    response.set_inscription_ids(RepeatedField::from_vec(inscription_ids));
    Ok(response)
}

/// Get inscription on a sat
pub fn get_sat_inscription(request: &SatInscriptionRequest) -> Result<SatInscriptionResponse, String> {
    let mut response = SatInscriptionResponse::new();
    let sat = request.get_sat();

    let sat_bytes = sat.to_le_bytes();
    if let Some(sequence_bytes) = TABLES.SAT_TO_SEQUENCE.select(&sat_bytes).get() {
        if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
            if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                response.set_inscription_id(entry.id.to_string());
            }
        }
    }

    Ok(response)
}

/// Get content of inscription on a sat
pub fn get_sat_inscription_content(request: &SatInscriptionContentRequest) -> Result<SatInscriptionContentResponse, String> {
    let mut response = SatInscriptionContentResponse::new();
    let sat = request.get_sat();

    let sat_bytes = sat.to_le_bytes();
    if let Some(sequence_bytes) = TABLES.SAT_TO_SEQUENCE.select(&sat_bytes).get() {
        if let Some(content) = TABLES.INSCRIPTION_CONTENT.select(&sequence_bytes).get() {
            response.set_content(content);
        }

        if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
            if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                if let Some(content_type) = entry.content_type {
                    response.set_content_type(content_type);
                }
            }
        }
    }

    Ok(response)
}

/// Get child inscriptions with full info
pub fn get_child_inscriptions(request: &ChildInscriptionsRequest) -> Result<ChildInscriptionsResponse, String> {
    let children_request = ChildrenRequest::new();
    // Note: This would need proper field copying in a real implementation
    let children_response = get_children(&children_request)?;
    
    let mut response = ChildInscriptionsResponse::new();
    let mut inscriptions = Vec::new();

    for child_id in children_response.get_children() {
        let mut inscription_request = InscriptionRequest::new();
        inscription_request.set_id(child_id.clone());
        
        if let Ok(inscription_response) = get_inscription(&inscription_request) {
            inscriptions.push(inscription_response.get_inscription().clone());
        }
    }

    response.set_inscriptions(RepeatedField::from_vec(inscriptions));
    Ok(response)
}

/// Get parent inscriptions with full info
pub fn get_parent_inscriptions(request: &ParentInscriptionsRequest) -> Result<ParentInscriptionsResponse, String> {
    let parents_request = ParentsRequest::new();
    // Note: This would need proper field copying in a real implementation
    let parents_response = get_parents(&parents_request)?;
    
    let mut response = ParentInscriptionsResponse::new();
    let mut inscriptions = Vec::new();

    for parent_id in parents_response.get_parents() {
        let mut inscription_request = InscriptionRequest::new();
        inscription_request.set_id(parent_id.clone());
        
        if let Ok(inscription_response) = get_inscription(&inscription_request) {
            inscriptions.push(inscription_response.get_inscription().clone());
        }
    }

    response.set_inscriptions(RepeatedField::from_vec(inscriptions));
    Ok(response)
}

/// Get undelegated content
pub fn get_undelegated_content(request: &UndelegatedContentRequest) -> Result<UndelegatedContentResponse, String> {
    // This would implement delegation resolution logic
    // For now, just return the direct content
    let mut content_request = ContentRequest::new();
    content_request.set_id(request.get_id().to_string());
    
    let content_response = get_content(&content_request)?;
    
    let mut response = UndelegatedContentResponse::new();
    response.set_content(content_response.get_content().to_vec());
    response.set_content_type(content_response.get_content_type().to_string());
    
    Ok(response)
}

/// Get UTXO information
pub fn get_utxo(request: &UtxoRequest) -> Result<UtxoResponse, String> {
    let mut response = UtxoResponse::new();
    
    // Parse outpoint
    let outpoint_str = request.get_outpoint();
    let parts: Vec<&str> = outpoint_str.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid outpoint format".to_string());
    }
    
    let txid = Txid::from_hex(parts[0]).map_err(|e| format!("Invalid txid: {}", e))?;
    let vout: u32 = parts[1].parse().map_err(|e| format!("Invalid vout: {}", e))?;
    
    let outpoint_bytes = txid.to_byte_array()
        .iter()
        .chain(vout.to_le_bytes().iter())
        .copied()
        .collect::<Vec<u8>>();

    // Get inscriptions on this outpoint
    let inscriptions_list = TABLES.OUTPOINT_TO_INSCRIPTIONS
        .select(&outpoint_bytes)
        .get_list()
        .unwrap_or_default();

    let mut inscription_ids = Vec::new();
    for chunk in inscriptions_list.chunks(4) {
        if chunk.len() == 4 {
            let sequence_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            
            if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    inscription_ids.push(entry.id.to_string());
                }
            }
        }
    }

    let mut utxo_info = UtxoInfo::new();
    utxo_info.set_outpoint(outpoint_str.to_string());
    utxo_info.set_inscription_ids(RepeatedField::from_vec(inscription_ids));

    response.set_utxo_info(utxo_info);
    Ok(response)
}

/// Get block hash by height
pub fn get_block_hash_at_height(request: &BlockHashAtHeightRequest) -> Result<BlockHashAtHeightResponse, String> {
    let mut response = BlockHashAtHeightResponse::new();
    let height = request.get_height();

    let height_bytes = height.to_le_bytes();
    if let Some(hash_bytes) = TABLES.HEIGHT_TO_BLOCK_HASH.select(&height_bytes).get() {
        if hash_bytes.len() == 32 {
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&hash_bytes);
            let block_hash = BlockHash::from_byte_array(hash_array);
            response.set_block_hash(block_hash.to_string());
        }
    }

    Ok(response)
}

/// Get block hash (alias for block_hash_at_height)
pub fn get_block_hash(request: &BlockHashRequest) -> Result<BlockHashResponse, String> {
    let mut height_request = BlockHashAtHeightRequest::new();
    height_request.set_height(request.get_height());
    
    let height_response = get_block_hash_at_height(&height_request)?;
    
    let mut response = BlockHashResponse::new();
    response.set_block_hash(height_response.get_block_hash().to_string());
    
    Ok(response)
}

/// Get block height by hash
pub fn get_block_height(request: &BlockHeightRequest) -> Result<BlockHeightResponse, String> {
    let mut response = BlockHeightResponse::new();
    
    let block_hash = BlockHash::from_hex(request.get_block_hash())
        .map_err(|e| format!("Invalid block hash: {}", e))?;
    
    let hash_bytes = block_hash.to_byte_array();
    if let Some(height_bytes) = TABLES.BLOCK_HASH_TO_HEIGHT.select(&hash_bytes).get() {
        if height_bytes.len() >= 4 {
            let height = u32::from_le_bytes([
                height_bytes[0], height_bytes[1], height_bytes[2], height_bytes[3]
            ]);
            response.set_height(height);
        }
    }

    Ok(response)
}

/// Get block time (placeholder implementation)
pub fn get_block_time(request: &BlockTimeRequest) -> Result<BlockTimeResponse, String> {
    let mut response = BlockTimeResponse::new();
    // This would require storing block timestamps
    response.set_timestamp(0);
    Ok(response)
}

/// Get block info
pub fn get_block_info(request: &BlockInfoRequest) -> Result<BlockInfoResponse, String> {
    let mut response = BlockInfoResponse::new();
    let height = request.get_height();

    let mut info = BlockInfo::new();
    info.set_height(height);

    // Get block hash
    let height_bytes = height.to_le_bytes();
    if let Some(hash_bytes) = TABLES.HEIGHT_TO_BLOCK_HASH.select(&height_bytes).get() {
        if hash_bytes.len() == 32 {
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&hash_bytes);
            let block_hash = BlockHash::from_byte_array(hash_array);
            info.set_block_hash(block_hash.to_string());
        }
    }

    // Get inscriptions count
    let inscriptions_list = TABLES.HEIGHT_TO_INSCRIPTIONS
        .select(&height_bytes)
        .get_list()
        .unwrap_or_default();
    info.set_inscription_count((inscriptions_list.len() / 4) as u32);

    response.set_block_info(info);
    Ok(response)
}

/// Get transaction info
pub fn get_tx(request: &TxRequest) -> Result<TxResponse, String> {
    let mut response = TxResponse::new();
    
    let txid = Txid::from_hex(request.get_txid())
        .map_err(|e| format!("Invalid txid: {}", e))?;
    
    let txid_bytes = txid.to_byte_array();
    let inscriptions_list = TABLES.TXID_TO_INSCRIPTIONS
        .select(&txid_bytes)
        .get_list()
        .unwrap_or_default();

    let mut inscription_ids = Vec::new();
    for chunk in inscriptions_list.chunks(4) {
        if chunk.len() == 4 {
            let sequence_bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            
            if let Some(entry_bytes) = TABLES.SEQUENCE_TO_INSCRIPTION_ENTRY.select(&sequence_bytes).get() {
                if let Ok(entry) = InscriptionEntry::from_bytes(&entry_bytes) {
                    inscription_ids.push(entry.id.to_string());
                }
            }
        }
    }

    let mut tx_info = TxInfo::new();
    tx_info.set_txid(request.get_txid().to_string());
    tx_info.set_inscription_ids(RepeatedField::from_vec(inscription_ids));

    response.set_tx_info(tx_info);
    Ok(response)
}

/// Parse inscription ID from string format
fn parse_inscription_id(id_str: &str) -> Result<InscriptionId, String> {
    let parts: Vec<&str> = id_str.split('i').collect();
    if parts.len() != 2 {
        return Err("Invalid inscription ID format".to_string());
    }
    
    let txid = Txid::from_hex(parts[0]).map_err(|e| format!("Invalid txid: {}", e))?;
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