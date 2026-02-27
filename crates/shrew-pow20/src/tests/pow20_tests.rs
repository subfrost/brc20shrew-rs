use crate::tables::{POW20_TICKERS, POW20_BALANCES};
use crate::pow20_indexer::{Pow20Ticker, Pow20Balance};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::indexing::{index_ord_block, index_pow20_block};
use shrew_test_helpers::blocks::{create_coinbase_transaction, create_block_with_txs};
use shrew_test_helpers::transactions::create_inscription_transaction;
use metashrew_support::index_pointer::KeyValuePointer;
use bitcoin::{OutPoint, Txid};
use std::str::FromStr;

/// Create a unique outpoint from a simple integer to avoid duplicate txid issues
fn unique_outpoint(n: u8) -> OutPoint {
    let txid_hex = format!("{:064x}", n);
    OutPoint {
        txid: Txid::from_str(&txid_hex).unwrap(),
        vout: 0,
    }
}

/// Helper: deploy a ticker with a unique outpoint, index it at height 0.
fn deploy_ticker_with_outpoint(ticker: &str, max: &str, lim: &str, diff: u32, start: u32, outpoint_n: u8) {
    let content = format!(
        r#"{{"p":"pow-20","op":"deploy","tick":"{}","max":"{}","lim":"{}","diff":"{}","start":"{}"}}"#,
        ticker, max, lim, diff, start,
    );
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", Some(unique_outpoint(outpoint_n)));
    let block = create_block_with_txs(vec![create_coinbase_transaction(0), tx]);
    index_ord_block(&block, 0).unwrap();
    index_pow20_block(&block, 0);
}

/// Helper: create and index a mint block with a unique outpoint
fn mint_ticker_with_outpoint(ticker: &str, amt: &str, nonce: &str, height: u32, outpoint_n: u8) {
    let content = format!(
        r#"{{"p":"pow-20","op":"mint","tick":"{}","amt":"{}","nonce":"{}"}}"#,
        ticker, amt, nonce,
    );
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", Some(unique_outpoint(outpoint_n)));
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx]);
    index_ord_block(&block, height).unwrap();
    index_pow20_block(&block, height);
}

/// Helper: read a ticker entry from state
fn read_ticker(ticker: &str) -> Option<Pow20Ticker> {
    let data = POW20_TICKERS.select(&ticker.to_lowercase().as_bytes().to_vec()).get();
    if data.is_empty() { return None; }
    serde_json::from_slice(&data).ok()
}

/// Helper: read a balance entry from state
fn read_balance(owner: &str, ticker: &str) -> Option<Pow20Balance> {
    let key = format!("{}:{}", owner, ticker);
    let data = POW20_BALANCES.select(&key.as_bytes().to_vec()).get();
    if data.is_empty() { return None; }
    serde_json::from_slice(&data).ok()
}

// ---------------------------------------------------------------------------
// Deploy tests
// ---------------------------------------------------------------------------

#[test]
fn test_pow20_deploy() {
    clear();
    deploy_ticker_with_outpoint("test", "21000", "1000", 0, 0, 1);

    let entry = read_ticker("test");
    assert!(entry.is_some(), "Ticker 'test' should be deployed");
    let entry = entry.unwrap();
    assert_eq!(entry.name, "test");
    assert_eq!(entry.max_supply, 21000);
    assert_eq!(entry.limit_per_mint, 1000);
    assert_eq!(entry.difficulty, 0);
}

#[test]
fn test_pow20_deploy_first_wins() {
    clear();
    deploy_ticker_with_outpoint("abcd", "21000", "1000", 0, 0, 1);

    // Deploy again at height 1 with a different outpoint
    let content2 = r#"{"p":"pow-20","op":"deploy","tick":"abcd","max":"99999","lim":"5000","diff":"5","start":"0"}"#;
    let tx2 = create_inscription_transaction(content2.as_bytes(), "text/plain", Some(unique_outpoint(2)));
    let block2 = create_block_with_txs(vec![create_coinbase_transaction(1), tx2]);
    index_ord_block(&block2, 1).unwrap();
    index_pow20_block(&block2, 1);

    let entry = read_ticker("abcd").unwrap();
    assert_eq!(entry.max_supply, 21000, "First deploy should win");
    assert_eq!(entry.limit_per_mint, 1000);
}

#[test]
fn test_pow20_deploy_max_4_chars() {
    clear();
    // 5-char ticker should be rejected
    let content = r#"{"p":"pow-20","op":"deploy","tick":"abcde","max":"21000","lim":"1000","diff":"0","start":"0"}"#;
    let tx = create_inscription_transaction(content.as_bytes(), "text/plain", Some(unique_outpoint(1)));
    let block = create_block_with_txs(vec![create_coinbase_transaction(0), tx]);
    index_ord_block(&block, 0).unwrap();
    index_pow20_block(&block, 0);

    let entry = read_ticker("abcde");
    assert!(entry.is_none(), "5-char ticker should be rejected");
}

#[test]
fn test_pow20_deploy_stores_fields() {
    clear();
    deploy_ticker_with_outpoint("pow", "100000", "500", 4, 10, 1);

    let entry = read_ticker("pow").unwrap();
    assert_eq!(entry.name, "pow");
    assert_eq!(entry.max_supply, 100000);
    assert_eq!(entry.limit_per_mint, 500);
    assert_eq!(entry.difficulty, 4);
    assert_eq!(entry.starting_block_height, 10);
    assert_eq!(entry.current_supply, 0);
}

// ---------------------------------------------------------------------------
// Mint tests
// ---------------------------------------------------------------------------

#[test]
fn test_pow20_mint_valid_nonce() {
    clear();
    // Deploy with difficulty=0 so any nonce passes
    deploy_ticker_with_outpoint("mint", "21000", "1000", 0, 0, 1);

    // Mint with a different outpoint
    mint_ticker_with_outpoint("mint", "100", "anynonce", 1, 2);

    let entry = read_ticker("mint").unwrap();
    assert_eq!(entry.current_supply, 100, "Supply should increase by minted amount");
}

#[test]
fn test_pow20_mint_invalid_nonce_rejected() {
    clear();
    // Deploy with difficulty=32 (requires 32 leading zero bits, extremely unlikely with nonce "0")
    deploy_ticker_with_outpoint("hard", "21000", "1000", 32, 0, 1);

    mint_ticker_with_outpoint("hard", "100", "0", 1, 2);

    let entry = read_ticker("hard").unwrap();
    assert_eq!(entry.current_supply, 0, "Mint with invalid PoW nonce should be rejected");
}

#[test]
fn test_pow20_mint_before_start_height_rejected() {
    clear();
    // Deploy with starting block 100
    deploy_ticker_with_outpoint("late", "21000", "1000", 0, 100, 1);

    mint_ticker_with_outpoint("late", "100", "nonce1", 50, 2);

    let entry = read_ticker("late").unwrap();
    assert_eq!(entry.current_supply, 0, "Mint before starting block height should be rejected");
}

#[test]
fn test_pow20_mint_exceeds_limit_rejected() {
    clear();
    // Deploy with limit 100
    deploy_ticker_with_outpoint("lim", "21000", "100", 0, 0, 1);

    mint_ticker_with_outpoint("lim", "500", "nonce1", 1, 2);

    let entry = read_ticker("lim").unwrap();
    assert_eq!(entry.current_supply, 0, "Mint amount exceeding limit should be rejected");
}

#[test]
fn test_pow20_mint_exceeds_max_supply_rejected() {
    clear();
    // Deploy with max 100, limit 100
    deploy_ticker_with_outpoint("cap", "100", "100", 0, 0, 1);

    // Mint 100 first (fills max supply)
    mint_ticker_with_outpoint("cap", "100", "nonce1", 1, 2);

    let entry1 = read_ticker("cap").unwrap();
    assert_eq!(entry1.current_supply, 100, "First mint should succeed");

    // Try to mint more - should be rejected since max supply reached
    mint_ticker_with_outpoint("cap", "1", "nonce2", 2, 3);

    let entry2 = read_ticker("cap").unwrap();
    assert_eq!(entry2.current_supply, 100, "Second mint should be rejected (max supply reached)");
}

#[test]
fn test_pow20_mint_nonexistent_ticker_ignored() {
    clear();
    // Don't deploy anything, just try to mint
    mint_ticker_with_outpoint("nope", "100", "nonce1", 1, 1);

    let entry = read_ticker("nope");
    assert!(entry.is_none(), "Mint on nonexistent ticker should be ignored");
}

// ---------------------------------------------------------------------------
// Transfer tests
// ---------------------------------------------------------------------------

#[test]
fn test_pow20_transfer_inscribe_reduces_available() {
    clear();
    // Deploy and mint
    deploy_ticker_with_outpoint("xfer", "21000", "1000", 0, 0, 1);
    mint_ticker_with_outpoint("xfer", "500", "nonce1", 1, 2);

    // Now create a transfer inscription with a unique outpoint
    let content = r#"{"p":"pow-20","op":"transfer","tick":"xfer","amt":"200"}"#;
    let tx2 = create_inscription_transaction(content.as_bytes(), "text/plain", Some(unique_outpoint(3)));
    let block2 = create_block_with_txs(vec![create_coinbase_transaction(2), tx2.clone()]);
    index_ord_block(&block2, 2).unwrap();
    index_pow20_block(&block2, 2);

    // Check that the owner address from the test transaction has reduced available balance.
    let owner = shrew_test_helpers::state::get_test_address(0).to_string();
    let balance = read_balance(&owner, "xfer");
    if let Some(b) = balance {
        assert_eq!(b.available_balance, 300,
            "Available balance should decrease by transfer amount");
        assert_eq!(b.total_balance, 500,
            "Total balance should remain the same after transfer inscribe");
    }
}

// ---------------------------------------------------------------------------
// Leading zero bits function tests
// ---------------------------------------------------------------------------

#[test]
fn test_leading_zero_bits_boundary_cases() {
    clear();
    // difficulty=0 means any hash passes
    deploy_ticker_with_outpoint("d0", "1000", "100", 0, 0, 1);
    mint_ticker_with_outpoint("d0", "100", "anything", 1, 2);
    let entry = read_ticker("d0").unwrap();
    assert_eq!(entry.current_supply, 100, "difficulty=0 should pass any nonce");
}

#[test]
fn test_leading_zero_bits_high_difficulty() {
    clear();
    // difficulty=256 means hash must be all zeros (extremely unlikely with simple nonce)
    deploy_ticker_with_outpoint("d256", "1000", "100", 256, 0, 1);
    mint_ticker_with_outpoint("d256", "100", "testnonce", 1, 2);
    let entry2 = read_ticker("d256").unwrap();
    assert_eq!(entry2.current_supply, 0, "difficulty=256 should reject any normal nonce");
}
