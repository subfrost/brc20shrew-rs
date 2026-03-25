///! Transfer Destination Tests
///!
///! Tests for all transfer-transfer (claim) destination types per OPI spec:
///! - Wallet: normal transfer to recipient address
///! - Burn: OP_RETURN output, tokens permanently destroyed
///! - Brc20ProgDeposit: BRC20-PROG OP_RETURN, phase-gated deposit
///! - SentAsFee: inscription spent as tx fee, tokens returned to sender
///!
///! Also tests classify_destination() and burned_supply tracking.

use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::brc20::{
    Brc20Indexer, Brc20Operation, Ticker, Balance, TransferInfo,
    TransferDestination, MAX_AMOUNT,
    OP_RETURN_PKSCRIPT, BRC20_PROG_OP_RETURN_PKSCRIPT, BRC20_PROG_ALL_TICKERS_HEIGHT,
};
use crate::tables::{Brc20Tickers, Brc20Balances, Brc20TransferableInscriptions};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::assertions::{assert_brc20_balance, assert_brc20_supply};

const SCALE: u128 = 1_000_000_000_000_000_000u128; // 10^18

/// Helper: deploy + mint to set up a sender with balance
fn setup_sender(indexer: &Brc20Indexer, ticker: &str, mint_amount: u128, sender: &str) {
    let deploy = Brc20Operation::Deploy {
        ticker: ticker.to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: mint_amount,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();
    let mint = Brc20Operation::Mint { ticker: ticker.to_string(), amount: mint_amount };
    indexer.process_operation(&mint, "mint_0i0", sender).unwrap();
}

/// Helper: inscribe a transfer (freeze available balance)
fn inscribe_transfer(indexer: &Brc20Indexer, ticker: &str, amount: u128, sender: &str, inscription_id: &str) {
    let transfer = Brc20Operation::Transfer { ticker: ticker.to_string(), amount };
    indexer.process_operation(&transfer, inscription_id, sender).unwrap();
}

/// Helper: build TransferInfo
fn transfer_info(ticker: &str, amount: u128, sender: &str) -> TransferInfo {
    TransferInfo { ticker: ticker.to_string(), amount, sender: sender.to_string() }
}

// ============================================================================
// classify_destination() unit tests
// ============================================================================

#[test]
fn test_classify_normal_address() {
    let dest = Brc20Indexer::classify_destination("76a91489abcdefab88ac", false);
    assert_eq!(dest, TransferDestination::Wallet("76a91489abcdefab88ac".to_string()));
}

#[test]
fn test_classify_op_return() {
    let dest = Brc20Indexer::classify_destination("6a", false);
    assert_eq!(dest, TransferDestination::Burn);
}

#[test]
fn test_classify_op_return_with_data() {
    // OP_RETURN with arbitrary data (starts with "6a")
    let dest = Brc20Indexer::classify_destination("6a146f7264", false);
    assert_eq!(dest, TransferDestination::Burn);
}

#[test]
fn test_classify_brc20_prog_op_return() {
    let dest = Brc20Indexer::classify_destination(BRC20_PROG_OP_RETURN_PKSCRIPT, false);
    assert_eq!(dest, TransferDestination::Brc20ProgDeposit);
}

#[test]
fn test_classify_sent_as_fee() {
    // When sent_as_fee=true, destination is always SentAsFee regardless of pkscript
    let dest = Brc20Indexer::classify_destination("76a91489abcdefab88ac", true);
    assert_eq!(dest, TransferDestination::SentAsFee);
}

#[test]
fn test_classify_sent_as_fee_overrides_op_return() {
    let dest = Brc20Indexer::classify_destination("6a", true);
    assert_eq!(dest, TransferDestination::SentAsFee);
}

#[test]
fn test_classify_empty_pkscript_as_fee() {
    let dest = Brc20Indexer::classify_destination("", false);
    assert_eq!(dest, TransferDestination::SentAsFee);
}

// ============================================================================
// TransferDestination::Wallet — normal transfer
// ============================================================================

#[test]
fn test_wallet_transfer_basic() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Wallet("bc1qrecipient".to_string()), &info, 840000).unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 600 * SCALE);
    assert_brc20_balance("bc1qrecipient", "ordi", 400 * SCALE, 400 * SCALE);
}

#[test]
fn test_wallet_transfer_preserves_supply() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Wallet("bc1qrecipient".to_string()), &info, 840000).unwrap();

    // Supply unchanged
    assert_brc20_supply("ordi", 1000 * SCALE);
    // No burned supply
    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 0);
}

#[test]
fn test_wallet_transfer_backwards_compat_claim_transfer() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    // Old API still works
    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.claim_transfer("bc1qrecipient", &info).unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 600 * SCALE);
    assert_brc20_balance("bc1qrecipient", "ordi", 400 * SCALE, 400 * SCALE);
}

// ============================================================================
// TransferDestination::Burn — OP_RETURN
// ============================================================================

#[test]
fn test_burn_reduces_sender_total() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    // After inscribe: available=600, total=1000
    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 1000 * SCALE);

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info, 840000).unwrap();

    // After burn: available=600 (unchanged), total=600 (reduced by burned amount)
    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 600 * SCALE);
}

#[test]
fn test_burn_increments_burned_supply() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info, 840000).unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 400 * SCALE, "burned_supply should increase by burned amount");
    assert_eq!(ticker.current_supply, 1000 * SCALE, "current_supply unchanged (tokens were minted)");
}

#[test]
fn test_burn_does_not_add_to_any_recipient() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info, 840000).unwrap();

    // No one receives the burned tokens
    let op_return_balance = Brc20Balances::new().get("6a", "ordi");
    assert!(op_return_balance.is_none(), "No balance should exist at OP_RETURN address");
}

#[test]
fn test_burn_multiple_times_accumulates() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");

    // Burn 200 twice
    inscribe_transfer(&indexer, "ordi", 200 * SCALE, "bc1qsender", "xfer1_0i0");
    let info1 = transfer_info("ordi", 200 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info1, 840000).unwrap();

    inscribe_transfer(&indexer, "ordi", 200 * SCALE, "bc1qsender", "xfer2_0i0");
    let info2 = transfer_info("ordi", 200 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info2, 840000).unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 400 * SCALE, "burned_supply should accumulate");
    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 600 * SCALE);
}

#[test]
fn test_burn_all_tokens() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 1000 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 1000 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info, 840000).unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 0, 0);

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 1000 * SCALE);
}

// ============================================================================
// TransferDestination::SentAsFee — return to sender
// ============================================================================

#[test]
fn test_sent_as_fee_returns_to_sender() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    // After inscribe: available=600, total=1000
    assert_brc20_balance("bc1qsender", "ordi", 600 * SCALE, 1000 * SCALE);

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::SentAsFee, &info, 840000).unwrap();

    // After fee return: available=1000 (restored), total=1000 (unchanged)
    assert_brc20_balance("bc1qsender", "ordi", 1000 * SCALE, 1000 * SCALE);
}

#[test]
fn test_sent_as_fee_no_burn() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::SentAsFee, &info, 840000).unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 0, "No tokens should be burned on fee return");
}

#[test]
fn test_sent_as_fee_preserves_total_supply() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::SentAsFee, &info, 840000).unwrap();

    assert_brc20_supply("ordi", 1000 * SCALE);
}

#[test]
fn test_sent_as_fee_no_recipient_created() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::SentAsFee, &info, 840000).unwrap();

    // No external recipient should have tokens
    let any_balance = Brc20Balances::new().get("bc1qrecipient", "ordi");
    assert!(any_balance.is_none(), "No recipient should receive tokens on fee return");
}

// ============================================================================
// TransferDestination::Brc20ProgDeposit — phase-gated
// ============================================================================

#[test]
fn test_brc20_prog_deposit_before_phase1_burns() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    // Before phase 1 (912690): should burn, not deposit
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info, 912689).unwrap();

    // Tokens burned
    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 400 * SCALE, "Should burn before phase 1");

    // No deposit balance
    let prog_balance = Brc20Balances::new().get(BRC20_PROG_OP_RETURN_PKSCRIPT, "ordi");
    assert!(prog_balance.is_none(), "No prog deposit before phase 1");
}

#[test]
fn test_brc20_prog_deposit_4byte_ticker_after_phase1_burns() {
    clear();
    let indexer = Brc20Indexer::new();
    // 4-byte ticker
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    // After phase 1 but 4-byte ticker (< 6 bytes) and before phase 2: should burn
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info, 912690).unwrap();

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 400 * SCALE,
        "4-byte ticker should burn even after phase 1 (before phase 2)");
}

#[test]
fn test_brc20_prog_deposit_6byte_ticker_after_phase1_deposits() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy a 6-byte ticker (needs height >= 912690 for parse)
    let deploy = Brc20Operation::Deploy {
        ticker: "abcdef".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();
    let mint = Brc20Operation::Mint { ticker: "abcdef".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();
    inscribe_transfer(&indexer, "abcdef", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("abcdef", 400 * SCALE, "bc1qsender");
    // 6-byte ticker after phase 1: should deposit, not burn
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info, 912690).unwrap();

    // Tokens deposited, not burned
    let data = Brc20Tickers::new().get("abcdef").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 0, "6-byte ticker should deposit, not burn");

    // Prog address should have balance
    let prog_data = Brc20Balances::new().get(BRC20_PROG_OP_RETURN_PKSCRIPT, "abcdef")
        .expect("Prog deposit balance should exist");
    let prog_balance: Balance = serde_json::from_slice(&prog_data).unwrap();
    assert_eq!(prog_balance.total_balance, 400 * SCALE);
    assert_eq!(prog_balance.available_balance, 400 * SCALE);

    // Sender balance reduced
    assert_brc20_balance("bc1qsender", "abcdef", 600 * SCALE, 600 * SCALE);
}

#[test]
fn test_brc20_prog_deposit_accumulates() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "abcdef".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();
    let mint = Brc20Operation::Mint { ticker: "abcdef".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    // Two deposits
    inscribe_transfer(&indexer, "abcdef", 200 * SCALE, "bc1qsender", "xfer1_0i0");
    let info1 = transfer_info("abcdef", 200 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info1, 912690).unwrap();

    inscribe_transfer(&indexer, "abcdef", 300 * SCALE, "bc1qsender", "xfer2_0i0");
    let info2 = transfer_info("abcdef", 300 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info2, 912690).unwrap();

    let prog_data = Brc20Balances::new().get(BRC20_PROG_OP_RETURN_PKSCRIPT, "abcdef").unwrap();
    let prog_balance: Balance = serde_json::from_slice(&prog_data).unwrap();
    assert_eq!(prog_balance.total_balance, 500 * SCALE, "Deposits should accumulate");

    assert_brc20_balance("bc1qsender", "abcdef", 500 * SCALE, 500 * SCALE);
}

// ============================================================================
// Mixed destination scenarios
// ============================================================================

#[test]
fn test_mix_wallet_and_burn() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");

    // Transfer 300 to wallet
    inscribe_transfer(&indexer, "ordi", 300 * SCALE, "bc1qsender", "xfer1_0i0");
    let info1 = transfer_info("ordi", 300 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Wallet("bc1qbob".to_string()), &info1, 840000).unwrap();

    // Burn 200
    inscribe_transfer(&indexer, "ordi", 200 * SCALE, "bc1qsender", "xfer2_0i0");
    let info2 = transfer_info("ordi", 200 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info2, 840000).unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 500 * SCALE, 500 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 300 * SCALE, 300 * SCALE);

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 200 * SCALE);
    assert_eq!(ticker.current_supply, 1000 * SCALE); // minted supply unchanged
}

#[test]
fn test_mix_wallet_burn_fee() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");

    // Transfer 200 to wallet
    inscribe_transfer(&indexer, "ordi", 200 * SCALE, "bc1qsender", "xfer1_0i0");
    let info1 = transfer_info("ordi", 200 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Wallet("bc1qbob".to_string()), &info1, 840000).unwrap();

    // Burn 300
    inscribe_transfer(&indexer, "ordi", 300 * SCALE, "bc1qsender", "xfer2_0i0");
    let info2 = transfer_info("ordi", 300 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info2, 840000).unwrap();

    // Fee return 100 (inscribe then resolve as fee)
    inscribe_transfer(&indexer, "ordi", 100 * SCALE, "bc1qsender", "xfer3_0i0");
    let info3 = transfer_info("ordi", 100 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::SentAsFee, &info3, 840000).unwrap();

    // Sender: started 1000, sent 200, burned 300, fee-returned 100
    // total = 1000 - 200 - 300 = 500, available = 500 (400 remaining after inscribes + 100 returned)
    assert_brc20_balance("bc1qsender", "ordi", 500 * SCALE, 500 * SCALE);
    assert_brc20_balance("bc1qbob", "ordi", 200 * SCALE, 200 * SCALE);

    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 300 * SCALE);
}

// ============================================================================
// burned_supply field: serde backwards compatibility
// ============================================================================

#[test]
fn test_burned_supply_default_zero_on_existing_ticker() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");

    // Ticker created without explicit burned_supply — serde default should be 0
    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 0, "burned_supply should default to 0");
}

#[test]
fn test_burned_supply_serialization_roundtrip() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");

    // Burn some tokens
    inscribe_transfer(&indexer, "ordi", 100 * SCALE, "bc1qsender", "xfer_0i0");
    let info = transfer_info("ordi", 100 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info, 840000).unwrap();

    // Re-read and verify
    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 100 * SCALE);

    // Burn more and verify accumulation persists through serialization
    inscribe_transfer(&indexer, "ordi", 50 * SCALE, "bc1qsender", "xfer2_0i0");
    let info2 = transfer_info("ordi", 50 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Burn, &info2, 840000).unwrap();

    let data2 = Brc20Tickers::new().get("ordi").unwrap();
    let ticker2: Ticker = serde_json::from_slice(&data2).unwrap();
    assert_eq!(ticker2.burned_supply, 150 * SCALE);
}

// ============================================================================
// Transfer-to-self with different destinations
// ============================================================================

#[test]
fn test_wallet_transfer_to_self() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Wallet("bc1qsender".to_string()), &info, 840000).unwrap();

    // Self-transfer: balance should be fully restored
    assert_brc20_balance("bc1qsender", "ordi", 1000 * SCALE, 1000 * SCALE);
}

// ============================================================================
// Edge cases: sender balance missing or corrupt
// ============================================================================

#[test]
fn test_burn_with_missing_sender_balance_no_panic() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy ticker but don't mint (no balance for sender)
    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    // Attempt burn with no sender balance — should not panic
    let info = transfer_info("ordi", 400 * SCALE, "bc1qnosender");
    let result = indexer.resolve_transfer(TransferDestination::Burn, &info, 840000);
    assert!(result.is_ok(), "Burn with missing sender should not panic");
}

#[test]
fn test_fee_return_with_missing_sender_balance_no_panic() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "ordi".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();

    let info = transfer_info("ordi", 400 * SCALE, "bc1qnosender");
    let result = indexer.resolve_transfer(TransferDestination::SentAsFee, &info, 840000);
    assert!(result.is_ok(), "Fee return with missing sender should not panic");
}

// ============================================================================
// Full lifecycle: inscribe → classify → resolve
// ============================================================================

#[test]
fn test_full_lifecycle_inscribe_classify_resolve_wallet() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 500 * SCALE, "bc1qsender", "xfer_0i0");

    // Simulate: output pkscript is a normal wallet
    let dest = Brc20Indexer::classify_destination("76a914abc88ac", false);
    let info = transfer_info("ordi", 500 * SCALE, "bc1qsender");
    indexer.resolve_transfer(dest, &info, 840000).unwrap();

    // Sender lost 500
    assert_brc20_balance("bc1qsender", "ordi", 500 * SCALE, 500 * SCALE);
    // Recipient (the pkscript hex) got 500
    assert_brc20_balance("76a914abc88ac", "ordi", 500 * SCALE, 500 * SCALE);
}

#[test]
fn test_full_lifecycle_inscribe_classify_resolve_burn() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 500 * SCALE, "bc1qsender", "xfer_0i0");

    let dest = Brc20Indexer::classify_destination("6a", false);
    assert_eq!(dest, TransferDestination::Burn);

    let info = transfer_info("ordi", 500 * SCALE, "bc1qsender");
    indexer.resolve_transfer(dest, &info, 840000).unwrap();

    assert_brc20_balance("bc1qsender", "ordi", 500 * SCALE, 500 * SCALE);
    let data = Brc20Tickers::new().get("ordi").unwrap();
    let ticker: Ticker = serde_json::from_slice(&data).unwrap();
    assert_eq!(ticker.burned_supply, 500 * SCALE);
}

#[test]
fn test_full_lifecycle_inscribe_classify_resolve_fee() {
    clear();
    let indexer = Brc20Indexer::new();
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 500 * SCALE, "bc1qsender", "xfer_0i0");

    let dest = Brc20Indexer::classify_destination("anything", true);
    assert_eq!(dest, TransferDestination::SentAsFee);

    let info = transfer_info("ordi", 500 * SCALE, "bc1qsender");
    indexer.resolve_transfer(dest, &info, 840000).unwrap();

    // Tokens returned to sender
    assert_brc20_balance("bc1qsender", "ordi", 1000 * SCALE, 1000 * SCALE);
}

#[test]
fn test_full_lifecycle_inscribe_classify_resolve_prog_deposit() {
    clear();
    let indexer = Brc20Indexer::new();

    // 6-byte ticker for prog deposit
    let deploy = Brc20Operation::Deploy {
        ticker: "abcdef".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();
    let mint = Brc20Operation::Mint { ticker: "abcdef".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();
    inscribe_transfer(&indexer, "abcdef", 500 * SCALE, "bc1qsender", "xfer_0i0");

    let dest = Brc20Indexer::classify_destination(BRC20_PROG_OP_RETURN_PKSCRIPT, false);
    assert_eq!(dest, TransferDestination::Brc20ProgDeposit);

    let info = transfer_info("abcdef", 500 * SCALE, "bc1qsender");
    indexer.resolve_transfer(dest, &info, 912690).unwrap();

    assert_brc20_balance("bc1qsender", "abcdef", 500 * SCALE, 500 * SCALE);
    let prog_data = Brc20Balances::new().get(BRC20_PROG_OP_RETURN_PKSCRIPT, "abcdef").unwrap();
    let prog_balance: Balance = serde_json::from_slice(&prog_data).unwrap();
    assert_eq!(prog_balance.total_balance, 500 * SCALE);
}

// ============================================================================
// Deposit event recording tests
// ============================================================================

use crate::tables::{Brc20ProgDeposits, DepositEvent};

#[test]
fn test_brc20_prog_deposit_records_event() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy 6-byte ticker
    let deploy = Brc20Operation::Deploy {
        ticker: "abcdef".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();
    let mint = Brc20Operation::Mint { ticker: "abcdef".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();
    inscribe_transfer(&indexer, "abcdef", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("abcdef", 400 * SCALE, "bc1qsender");
    // Deposit at phase 1 height — should record event
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info, 912690).unwrap();

    // Check deposit event was recorded
    let events = Brc20ProgDeposits::new().get(912690);
    assert_eq!(events.len(), 1, "Should have 1 deposit event");
    assert_eq!(events[0].ticker, "abcdef");
    assert_eq!(events[0].amount, 400 * SCALE);
    assert_eq!(events[0].sender, "bc1qsender");
}

#[test]
fn test_brc20_prog_deposit_burn_does_not_record_event() {
    clear();
    let indexer = Brc20Indexer::new();

    // 4-byte ticker — deposit should burn (before phase 2), NOT record event
    setup_sender(&indexer, "ordi", 1000 * SCALE, "bc1qsender");
    inscribe_transfer(&indexer, "ordi", 400 * SCALE, "bc1qsender", "xfer_0i0");

    let info = transfer_info("ordi", 400 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info, 912690).unwrap();

    // No deposit event (burned instead)
    let events = Brc20ProgDeposits::new().get(912690);
    assert_eq!(events.len(), 0, "Burn should NOT record deposit event");
}

#[test]
fn test_brc20_prog_deposit_events_accumulate() {
    clear();
    let indexer = Brc20Indexer::new();

    let deploy = Brc20Operation::Deploy {
        ticker: "abcdef".to_string(),
        max_supply: 21_000_000 * SCALE,
        limit_per_mint: 1000 * SCALE,
        decimals: 18,
        self_mint: false,
        salt: None,
    };
    indexer.process_operation(&deploy, "deploy_0i0", "bc1qdeployer").unwrap();
    let mint = Brc20Operation::Mint { ticker: "abcdef".to_string(), amount: 1000 * SCALE };
    indexer.process_operation(&mint, "mint_0i0", "bc1qsender").unwrap();

    // Two deposits at same height
    inscribe_transfer(&indexer, "abcdef", 200 * SCALE, "bc1qsender", "xfer1_0i0");
    let info1 = transfer_info("abcdef", 200 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info1, 912690).unwrap();

    inscribe_transfer(&indexer, "abcdef", 300 * SCALE, "bc1qsender", "xfer2_0i0");
    let info2 = transfer_info("abcdef", 300 * SCALE, "bc1qsender");
    indexer.resolve_transfer(TransferDestination::Brc20ProgDeposit, &info2, 912690).unwrap();

    let events = Brc20ProgDeposits::new().get(912690);
    assert_eq!(events.len(), 2, "Should have 2 deposit events");
    assert_eq!(events[0].amount, 200 * SCALE);
    assert_eq!(events[1].amount, 300 * SCALE);
}

#[test]
fn test_brc20_prog_deposit_events_clear() {
    clear();
    let deposits = Brc20ProgDeposits::new();
    deposits.push(100, &DepositEvent { ticker: "test".to_string(), amount: 1000, sender: "bc1q".to_string() });
    assert_eq!(deposits.get(100).len(), 1);

    deposits.clear(100);
    assert_eq!(deposits.get(100).len(), 0, "Clear should remove all events for height");
}
