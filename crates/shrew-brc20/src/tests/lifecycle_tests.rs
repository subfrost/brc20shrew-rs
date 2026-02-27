use crate::brc20::{Brc20Indexer, Brc20Operation, TransferInfo};
use crate::tables::{Brc20Tickers, Brc20Balances};
use shrew_test_helpers::state::clear;
use shrew_test_helpers::assertions::{assert_brc20_balance, assert_brc20_supply};

/// Helper: deploy a ticker via process_operation
fn deploy(indexer: &Brc20Indexer, ticker: &str, max_supply: u64, lim: u64, inscription_id: &str) {
    let op = Brc20Operation::Deploy {
        ticker: ticker.to_string(),
        max_supply,
        limit_per_mint: lim,
        decimals: 18,
    };
    indexer.process_operation(&op, inscription_id, "bc1qdeployer").unwrap();
}

/// Helper: mint tokens via process_operation
fn mint(indexer: &Brc20Indexer, ticker: &str, amount: u64, owner: &str, inscription_id: &str) {
    let op = Brc20Operation::Mint {
        ticker: ticker.to_string(),
        amount,
    };
    indexer.process_operation(&op, inscription_id, owner).unwrap();
}

/// Helper: inscribe a transfer via process_operation
fn transfer_inscribe(
    indexer: &Brc20Indexer,
    ticker: &str,
    amount: u64,
    owner: &str,
    inscription_id: &str,
) {
    let op = Brc20Operation::Transfer {
        ticker: ticker.to_string(),
        amount,
    };
    indexer.process_operation(&op, inscription_id, owner).unwrap();
}

/// Helper: claim a transfer
fn transfer_claim(indexer: &Brc20Indexer, ticker: &str, amount: u64, sender: &str, recipient: &str) {
    let info = TransferInfo {
        ticker: ticker.to_string(),
        amount,
        sender: sender.to_string(),
    };
    indexer.claim_transfer(recipient, &info).unwrap();
}

// ---------------------------------------------------------------------------
// Full lifecycle tests
// ---------------------------------------------------------------------------

#[test]
fn test_deploy_mint_transfer_lifecycle() {
    clear();
    let indexer = Brc20Indexer::new();

    // 1. Deploy "ordi" with max=21M, lim=1000
    deploy(&indexer, "ordi", 21_000_000, 1000, "deploy_0i0");

    // Verify ticker exists
    let ticker_data = Brc20Tickers::new().get("ordi");
    assert!(ticker_data.is_some(), "Ticker should exist after deploy");

    // 2. Mint 1000 to alice
    mint(&indexer, "ordi", 1000, "bc1qalice", "mint_0i0");
    assert_brc20_supply("ordi", 1000);
    assert_brc20_balance("bc1qalice", "ordi", 1000, 1000);

    // 3. Transfer inscribe 400 from alice
    transfer_inscribe(&indexer, "ordi", 400, "bc1qalice", "xfer_inscribe_0i0");
    // Alice: available=600, total=1000
    assert_brc20_balance("bc1qalice", "ordi", 600, 1000);

    // 4. Claim transfer to bob
    transfer_claim(&indexer, "ordi", 400, "bc1qalice", "bc1qbob");
    // Alice: available=600, total=600
    assert_brc20_balance("bc1qalice", "ordi", 600, 600);
    // Bob: available=400, total=400
    assert_brc20_balance("bc1qbob", "ordi", 400, 400);

    // Supply should be unchanged
    assert_brc20_supply("ordi", 1000);
}

#[test]
fn test_multiple_mints_same_ticker() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "pepe", 1_000_000, 500, "deploy_0i0");

    // 3 mints of 500 each
    mint(&indexer, "pepe", 500, "bc1qminer1", "mint1_0i0");
    mint(&indexer, "pepe", 500, "bc1qminer2", "mint2_0i0");
    mint(&indexer, "pepe", 500, "bc1qminer3", "mint3_0i0");

    assert_brc20_supply("pepe", 1500);
    assert_brc20_balance("bc1qminer1", "pepe", 500, 500);
    assert_brc20_balance("bc1qminer2", "pepe", 500, 500);
    assert_brc20_balance("bc1qminer3", "pepe", 500, 500);
}

#[test]
fn test_mint_to_max_supply() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "tiny", 1000, 1000, "deploy_0i0");

    // Mint exactly to max supply
    mint(&indexer, "tiny", 1000, "bc1qminter", "mint1_0i0");
    assert_brc20_supply("tiny", 1000);
    assert_brc20_balance("bc1qminter", "tiny", 1000, 1000);

    // Next mint should be rejected (would exceed max_supply)
    mint(&indexer, "tiny", 1, "bc1qminter2", "mint2_0i0");

    // Supply still 1000
    assert_brc20_supply("tiny", 1000);
    // minter2 should have no balance
    let balance = Brc20Balances::new().get("bc1qminter2", "tiny");
    assert!(balance.is_none(), "Mint beyond max supply should be rejected");
}

#[test]
fn test_multiple_tickers() {
    clear();
    let indexer = Brc20Indexer::new();

    // Deploy two different tickers
    deploy(&indexer, "ordi", 21_000_000, 1000, "deploy_ordi_0i0");
    deploy(&indexer, "pepe", 420_000_000, 5000, "deploy_pepe_0i0");

    // Mint ordi to alice
    mint(&indexer, "ordi", 1000, "bc1qalice", "mint_ordi_0i0");
    // Mint pepe to alice
    mint(&indexer, "pepe", 5000, "bc1qalice", "mint_pepe_0i0");
    // Mint ordi to bob
    mint(&indexer, "ordi", 800, "bc1qbob", "mint_ordi_1i0");

    assert_brc20_supply("ordi", 1800);
    assert_brc20_supply("pepe", 5000);
    assert_brc20_balance("bc1qalice", "ordi", 1000, 1000);
    assert_brc20_balance("bc1qalice", "pepe", 5000, 5000);
    assert_brc20_balance("bc1qbob", "ordi", 800, 800);

    // Bob should have no pepe balance
    let bob_pepe = Brc20Balances::new().get("bc1qbob", "pepe");
    assert!(bob_pepe.is_none(), "Bob should have no pepe tokens");
}

#[test]
fn test_brc20_across_blocks() {
    // Simulate operations spread across 3 "blocks" (process_operation calls).
    // Since process_operation is stateless with respect to block boundaries,
    // we just call clear once and interleave operations to prove state persists.
    clear();
    let indexer = Brc20Indexer::new();

    // Block 1: Deploy
    deploy(&indexer, "ordi", 21_000_000, 1000, "deploy_0i0");
    assert_brc20_supply("ordi", 0);

    // Block 2: Mint
    mint(&indexer, "ordi", 1000, "bc1qalice", "mint_0i0");
    assert_brc20_supply("ordi", 1000);
    assert_brc20_balance("bc1qalice", "ordi", 1000, 1000);

    // Block 3: Another mint + transfer inscribe
    mint(&indexer, "ordi", 500, "bc1qbob", "mint2_0i0");
    transfer_inscribe(&indexer, "ordi", 300, "bc1qalice", "xfer_0i0");

    assert_brc20_supply("ordi", 1500);
    assert_brc20_balance("bc1qalice", "ordi", 700, 1000);
    assert_brc20_balance("bc1qbob", "ordi", 500, 500);

    // Claim transfer from alice to bob
    transfer_claim(&indexer, "ordi", 300, "bc1qalice", "bc1qbob");
    assert_brc20_balance("bc1qalice", "ordi", 700, 700);
    assert_brc20_balance("bc1qbob", "ordi", 800, 800);
}

#[test]
fn test_transfer_to_self() {
    clear();
    let indexer = Brc20Indexer::new();

    deploy(&indexer, "ordi", 21_000_000, 1000, "deploy_0i0");
    mint(&indexer, "ordi", 1000, "bc1qsame", "mint_0i0");

    // Transfer inscribe 400 from self
    transfer_inscribe(&indexer, "ordi", 400, "bc1qsame", "xfer_0i0");
    // Available=600, Total=1000
    assert_brc20_balance("bc1qsame", "ordi", 600, 1000);

    // Claim to self (sender == recipient)
    transfer_claim(&indexer, "ordi", 400, "bc1qsame", "bc1qsame");

    // claim_transfer adds 400 to recipient (bc1qsame) and subtracts 400 from sender (bc1qsame) total.
    // After claim:
    //   new_owner_balance gets +400 available, +400 total
    //   sender_balance gets -400 total
    // Since sender == recipient, the operations are:
    //   balance starts at available=600, total=1000
    //   claim adds: available=600+400=1000, total=1000+400=1400
    //   then sender deduct: total=1400-400=1000
    // Final: available=1000, total=1000
    assert_brc20_balance("bc1qsame", "ordi", 1000, 1000);

    // Supply unchanged
    assert_brc20_supply("ordi", 1000);
}
