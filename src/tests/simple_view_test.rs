//! Simple View Function Test
//!
//! This test verifies that the view functions compile and can be called correctly
//! with the proper protobuf message patterns.

use wasm_bindgen_test::*;
use crate::view::*;
use crate::proto::shrewscriptions::*;

#[wasm_bindgen_test]
fn test_view_functions_compile() {
    // Test get_inscription with proper protobuf message
    let mut get_inscription_req = GetInscriptionRequest::default();
    let mut proto_id = InscriptionId::default();
    proto_id.txid = vec![0u8; 32]; // Dummy txid
    proto_id.index = 0;
    get_inscription_req.query = Some(get_inscription_request::Query::Id(proto_id));
    
    let inscription_response = get_inscription(&get_inscription_req);
    assert!(inscription_response.is_ok());
    
    // Test get_content
    let mut get_content_req = GetContentRequest::default();
    let mut proto_id2 = InscriptionId::default();
    proto_id2.txid = vec![0u8; 32];
    proto_id2.index = 0;
    get_content_req.id = Some(proto_id2);
    
    let content_response = get_content(&get_content_req);
    assert!(content_response.is_ok());
    
    // Test get_inscriptions
    let mut get_inscriptions_req = GetInscriptionsRequest::default();
    let mut pagination = PaginationRequest::default();
    pagination.limit = 10;
    pagination.page = 0;
    get_inscriptions_req.pagination = Some(pagination);
    
    let inscriptions_response = get_inscriptions(&get_inscriptions_req);
    assert!(inscriptions_response.is_ok());
    
    // Test get_children
    let mut get_children_req = GetChildrenRequest::default();
    let mut proto_id3 = InscriptionId::default();
    proto_id3.txid = vec![0u8; 32];
    proto_id3.index = 0;
    get_children_req.parent_id = Some(proto_id3);
    
    let children_response = get_children(&get_children_req);
    assert!(children_response.is_ok());
    
    // Test get_parents
    let mut get_parents_req = GetParentsRequest::default();
    let mut proto_id4 = InscriptionId::default();
    proto_id4.txid = vec![0u8; 32];
    proto_id4.index = 0;
    get_parents_req.child_id = Some(proto_id4);
    
    let parents_response = get_parents(&get_parents_req);
    assert!(parents_response.is_ok());
    
    // Test get_metadata
    let mut get_metadata_req = GetMetadataRequest::default();
    let mut proto_id5 = InscriptionId::default();
    proto_id5.txid = vec![0u8; 32];
    proto_id5.index = 0;
    get_metadata_req.id = Some(proto_id5);
    
    let metadata_response = get_metadata(&get_metadata_req);
    assert!(metadata_response.is_ok());
    
    // Test get_sat
    let mut get_sat_req = GetSatRequest::default();
    get_sat_req.sat = 5000000000;
    
    let sat_response = get_sat(&get_sat_req);
    assert!(sat_response.is_ok());
    
    // Test get_block_info with height
    let mut get_block_info_req = GetBlockInfoRequest::default();
    get_block_info_req.query = Some(get_block_info_request::Query::Height(840000));
    
    let block_info_response = get_block_info(&get_block_info_req);
    assert!(block_info_response.is_ok());
    
    // Test get_block_info with hash
    let mut get_block_info_req2 = GetBlockInfoRequest::default();
    get_block_info_req2.query = Some(get_block_info_request::Query::Hash("0000000000000000000000000000000000000000000000000000000000000000".to_string()));
    
    let block_info_response2 = get_block_info(&get_block_info_req2);
    assert!(block_info_response2.is_ok());
    
    // Test get_block_hash_at_height
    let mut get_block_hash_req = GetBlockHashRequest::default();
    get_block_hash_req.height = Some(840000);
    
    let block_hash_response = get_block_hash_at_height(&get_block_hash_req);
    assert!(block_hash_response.is_ok());
}

#[wasm_bindgen_test]
fn test_view_function_responses() {
    crate::tests::helpers::clear();
    // Test that responses have the expected structure
    let mut get_inscription_req = GetInscriptionRequest::default();
    let mut proto_id = InscriptionId::default();
    proto_id.txid = vec![0u8; 32];
    proto_id.index = 0;
    get_inscription_req.query = Some(get_inscription_request::Query::Id(proto_id));
    
    let inscription_response = get_inscription(&get_inscription_req).unwrap();
    
    // Response should be valid (empty but structured correctly)
    assert!(inscription_response.id.is_none()); // No data in empty database
    assert_eq!(inscription_response.number, 0);
    assert!(inscription_response.content_type.is_none());
    assert!(inscription_response.content_length.is_none());
    assert!(inscription_response.satpoint.is_none());
    
    // Test get_inscriptions response structure
    let mut get_inscriptions_req = GetInscriptionsRequest::default();
    let mut pagination = PaginationRequest::default();
    pagination.limit = 10;
    pagination.page = 0;
    get_inscriptions_req.pagination = Some(pagination);
    
    let inscriptions_response = get_inscriptions(&get_inscriptions_req).unwrap();
    
    // Should have pagination info even with empty database
    assert!(inscriptions_response.pagination.is_some());
    let pagination_resp = inscriptions_response.pagination.as_ref().unwrap();
    assert_eq!(pagination_resp.limit, 10);
    assert_eq!(pagination_resp.page, 0);
    assert_eq!(pagination_resp.total, 0); // Empty database
    assert!(!pagination_resp.more);
    
    // Test get_sat response structure
    let mut get_sat_req = GetSatRequest::default();
    get_sat_req.sat = 5000000000;
    
    let sat_response = get_sat(&get_sat_req).unwrap();
    assert_eq!(sat_response.number, 5000000000);
}

#[wasm_bindgen_test]
fn test_error_handling() {
    // Test get_inscription with missing ID
    let get_inscription_req = GetInscriptionRequest::default(); // No ID set
    let result = get_inscription(&get_inscription_req);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Request must specify a query"));
    
    // Test get_content with missing ID
    let get_content_req = GetContentRequest::default(); // No ID set
    let result = get_content(&get_content_req);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing id"));
    
    // Test get_children with missing parent_id
    let get_children_req = GetChildrenRequest::default(); // No parent_id set
    let result = get_children(&get_children_req);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing parent_id"));
    
    // Test get_parents with missing child_id
    let get_parents_req = GetParentsRequest::default(); // No child_id set
    let result = get_parents(&get_parents_req);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing child_id"));
    
    // Test get_metadata with missing ID
    let get_metadata_req = GetMetadataRequest::default(); // No ID set
    let result = get_metadata(&get_metadata_req);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing id"));
    
    // Test get_block_info with no query
    let get_block_info_req = GetBlockInfoRequest::default(); // No query set
    let result = get_block_info(&get_block_info_req);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No query parameter provided"));
}