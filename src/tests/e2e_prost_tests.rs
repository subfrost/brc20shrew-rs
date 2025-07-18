//! End-to-end tests for view functions after the prost/serde_json refactor.
//!
//! ## Purpose
//! This test suite validates that all view functions are working correctly
//! with the new `prost`-based architecture and `serde_json` for serialization.
//!
//! ## Test Strategy
//! - Each test focuses on a specific view function.
//! - Tests set up the necessary state in the database using test helpers.
//! - A JSON request is constructed and serialized.
//! - The top-level view function from `src/lib.rs` is called with the serialized request.
//! - The JSON response is deserialized into the expected `prost`-generated struct.
//! - Assertions are made on the response fields to verify correctness.

use super::helpers::{create_inscription_envelope, index_block_with_inscriptions};
use crate::{
    get_balance,
    proto::{GetBalanceRequest, BalanceResponse},
};
use metashrew_core::test_utils::TestContext;

#[test]
fn test_get_balance_e2e() {
    let context = TestContext::new();
    context.set();

    // 1. Setup: Index a block with a BRC20 mint to create a balance.
    // (This part needs to be implemented based on how BRC20 indexing works)

    // 2. Construct Request
    let request = GetBalanceRequest {
        ticker: "shrew".to_string(),
        address: "bc1paf2gh9zu7xjw3jnuxv292y92daqqc9f5j2f2y2j2g".to_string(),
    };
    let request_bytes = serde_json::to_vec(&request).unwrap();

    // 3. Call View Function
    let response_bytes = get_balance(&request_bytes).unwrap();

    // 4. Deserialize and Assert
    let response: BalanceResponse = serde_json::from_slice(&response_bytes).unwrap();
    
    // TODO: Update this assertion with the expected balance
    assert_eq!(response.balance, "0"); 
}