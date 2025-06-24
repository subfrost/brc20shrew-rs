use crate::proto::shrewscriptions::{
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
};
use metashrew_core::message::MessageContext;
use protobuf::{Message, MessageDyn};
use std::collections::HashMap;

/// Message context for handling protobuf requests and responses
pub struct ShrewscriptionsMessageContext {
    handlers: HashMap<String, Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String>>>,
}

impl ShrewscriptionsMessageContext {
    pub fn new() -> Self {
        let mut context = Self {
            handlers: HashMap::new(),
        };
        context.register_handlers();
        context
    }

    fn register_handlers(&mut self) {
        // Register all view function handlers
        self.register_handler("inscription", handle_inscription);
        self.register_handler("inscriptions", handle_inscriptions);
        self.register_handler("children", handle_children);
        self.register_handler("parents", handle_parents);
        self.register_handler("content", handle_content);
        self.register_handler("metadata", handle_metadata);
        self.register_handler("sat", handle_sat);
        self.register_handler("satinscriptions", handle_sat_inscriptions);
        self.register_handler("satinscription", handle_sat_inscription);
        self.register_handler("satinscriptioncontent", handle_sat_inscription_content);
        self.register_handler("childinscriptions", handle_child_inscriptions);
        self.register_handler("parentinscriptions", handle_parent_inscriptions);
        self.register_handler("undelegatedcontent", handle_undelegated_content);
        self.register_handler("utxo", handle_utxo);
        self.register_handler("blockhash", handle_block_hash);
        self.register_handler("blockhashatheight", handle_block_hash_at_height);
        self.register_handler("blockheight", handle_block_height);
        self.register_handler("blocktime", handle_block_time);
        self.register_handler("blockinfo", handle_block_info);
        self.register_handler("tx", handle_tx);
    }

    fn register_handler<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[u8]) -> Result<Vec<u8>, String> + 'static,
    {
        self.handlers.insert(name.to_string(), Box::new(handler));
    }

    pub fn handle_message(&self, method: &str, input: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(handler) = self.handlers.get(method) {
            handler(input)
        } else {
            Err(format!("Unknown method: {}", method))
        }
    }
}

impl MessageContext for ShrewscriptionsMessageContext {
    fn handle(&self, method: &str, input: &[u8]) -> Result<Vec<u8>, String> {
        self.handle_message(method, input)
    }
}

// Handler functions for each view method

fn handle_inscription(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = InscriptionRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse InscriptionRequest: {}", e))?;
    
    let response = crate::view::get_inscription(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize InscriptionResponse: {}", e))
}

fn handle_inscriptions(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = InscriptionsRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse InscriptionsRequest: {}", e))?;
    
    let response = crate::view::get_inscriptions(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize InscriptionsResponse: {}", e))
}

fn handle_children(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = ChildrenRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse ChildrenRequest: {}", e))?;
    
    let response = crate::view::get_children(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize ChildrenResponse: {}", e))
}

fn handle_parents(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = ParentsRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse ParentsRequest: {}", e))?;
    
    let response = crate::view::get_parents(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize ParentsResponse: {}", e))
}

fn handle_content(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = ContentRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse ContentRequest: {}", e))?;
    
    let response = crate::view::get_content(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize ContentResponse: {}", e))
}

fn handle_metadata(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = MetadataRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse MetadataRequest: {}", e))?;
    
    let response = crate::view::get_metadata(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize MetadataResponse: {}", e))
}

fn handle_sat(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = SatRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse SatRequest: {}", e))?;
    
    let response = crate::view::get_sat(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize SatResponse: {}", e))
}

fn handle_sat_inscriptions(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = SatInscriptionsRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse SatInscriptionsRequest: {}", e))?;
    
    let response = crate::view::get_sat_inscriptions(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize SatInscriptionsResponse: {}", e))
}

fn handle_sat_inscription(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = SatInscriptionRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse SatInscriptionRequest: {}", e))?;
    
    let response = crate::view::get_sat_inscription(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize SatInscriptionResponse: {}", e))
}

fn handle_sat_inscription_content(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = SatInscriptionContentRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse SatInscriptionContentRequest: {}", e))?;
    
    let response = crate::view::get_sat_inscription_content(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize SatInscriptionContentResponse: {}", e))
}

fn handle_child_inscriptions(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = ChildInscriptionsRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse ChildInscriptionsRequest: {}", e))?;
    
    let response = crate::view::get_child_inscriptions(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize ChildInscriptionsResponse: {}", e))
}

fn handle_parent_inscriptions(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = ParentInscriptionsRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse ParentInscriptionsRequest: {}", e))?;
    
    let response = crate::view::get_parent_inscriptions(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize ParentInscriptionsResponse: {}", e))
}

fn handle_undelegated_content(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = UndelegatedContentRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse UndelegatedContentRequest: {}", e))?;
    
    let response = crate::view::get_undelegated_content(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize UndelegatedContentResponse: {}", e))
}

fn handle_utxo(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = UtxoRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse UtxoRequest: {}", e))?;
    
    let response = crate::view::get_utxo(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize UtxoResponse: {}", e))
}

fn handle_block_hash(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = BlockHashRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse BlockHashRequest: {}", e))?;
    
    let response = crate::view::get_block_hash(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize BlockHashResponse: {}", e))
}

fn handle_block_hash_at_height(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = BlockHashAtHeightRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse BlockHashAtHeightRequest: {}", e))?;
    
    let response = crate::view::get_block_hash_at_height(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize BlockHashAtHeightResponse: {}", e))
}

fn handle_block_height(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = BlockHeightRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse BlockHeightRequest: {}", e))?;
    
    let response = crate::view::get_block_height(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize BlockHeightResponse: {}", e))
}

fn handle_block_time(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = BlockTimeRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse BlockTimeRequest: {}", e))?;
    
    let response = crate::view::get_block_time(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize BlockTimeResponse: {}", e))
}

fn handle_block_info(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = BlockInfoRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse BlockInfoRequest: {}", e))?;
    
    let response = crate::view::get_block_info(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize BlockInfoResponse: {}", e))
}

fn handle_tx(input: &[u8]) -> Result<Vec<u8>, String> {
    let request = TxRequest::parse_from_bytes(input)
        .map_err(|e| format!("Failed to parse TxRequest: {}", e))?;
    
    let response = crate::view::get_tx(&request)?;
    
    response.write_to_bytes()
        .map_err(|e| format!("Failed to serialize TxResponse: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_context_creation() {
        let context = ShrewscriptionsMessageContext::new();
        assert_eq!(context.handlers.len(), 20); // Should have all 20 handlers
    }

    #[test]
    fn test_unknown_method() {
        let context = ShrewscriptionsMessageContext::new();
        let result = context.handle_message("unknown_method", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown method"));
    }
}