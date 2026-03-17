use wasm_bindgen_test::wasm_bindgen_test as test;
use shrew_test_helpers::state::clear;
use shrew_test_helpers::blocks::*;
use shrew_test_helpers::transactions::*;
use shrew_test_helpers::indexing::*;
use shrew_test_helpers::assertions::*;
use shrew_test_helpers::brc20::*;
use shrew_test_helpers::bitmap::*;
use shrew_test_helpers::sns::*;
use shrew_test_helpers::runes::*;
use shrew_support::inscription::InscriptionId;
use shrew_bitmap::tables::BITMAP_NUMBER_TO_ID;
use metashrew_support::index_pointer::KeyValuePointer;

/// Test: inscription with BRC20 deploy JSON is indexed by both ord and brc20 indexers
#[test]
fn test_e2e_inscription_to_brc20() {
    clear();
    let height = 0u32;
    let content = create_brc20_json("deploy", "test", &[("max", "21000"), ("lim", "1000")]);
    let tx = create_inscription_transaction(&content, "text/plain", None);
    let block = create_block_with_txs(vec![create_coinbase_transaction(height), tx.clone()]);

    index_ord_block(&block, height).unwrap();
    index_brc20_block(&block, height);

    // Ord should have indexed the inscription
    assert_inscription_exists(tx.txid(), 0);
    assert_inscription_count(1);
}

/// Test: bitmap inscription is indexed by both ord and bitmap indexers
#[test]
fn test_e2e_inscription_to_bitmap() {
    clear();
    let height = 840000u32;
    let (block, tx) = create_bitmap_inscription_block(0, height);

    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);

    // Ord should have the inscription
    assert_inscription_exists(tx.txid(), 0);
    assert_inscription_count(1);

    // Bitmap indexer should have registered bitmap 0
    let inscription_id = InscriptionId::new(tx.txid(), 0);
    assert_bitmap_registered(0, &inscription_id);
}

/// Test: SNS registration is indexed by both ord and sns indexers
#[test]
fn test_e2e_inscription_to_sns() {
    clear();
    let height = 840000u32;
    let (block, tx) = create_sns_reg_block("alice.btc", height);

    index_ord_block(&block, height).unwrap();
    index_sns_block(&block, height);

    // Ord should have the inscription
    assert_inscription_exists(tx.txid(), 0);
    assert_inscription_count(1);

    // SNS indexer should have registered the name
    let inscription_id = InscriptionId::new(tx.txid(), 0);
    assert_sns_registered("alice.btc", &inscription_id);
}

/// Test: rune etching in a block without inscriptions works independently
#[test]
fn test_e2e_runes_independent_of_inscriptions() {
    clear();
    let height = 840000u32;
    let (block, rune_id) = create_etching_block("TESTCOIN", 0, Some('T'), 1000, None, height);

    index_ord_block(&block, height).unwrap();
    index_runes_block(&block, height);

    // Rune entry should exist
    assert_rune_entry(rune_id, "TESTCOIN", 1000);
}

/// Test: block with both an inscription and a bitmap processed by all indexers
#[test]
fn test_e2e_all_indexers_same_block() {
    clear();
    let height = 840000u32;

    // Create a bitmap inscription (text/plain "0.bitmap")
    let bitmap_content = b"0.bitmap";
    let bitmap_tx = create_inscription_transaction(bitmap_content, "text/plain", None);
    let block = create_block_with_txs(vec![
        create_coinbase_transaction(height),
        bitmap_tx.clone(),
    ]);

    index_all(&block, height).unwrap();

    // Ord should have the inscription
    assert_inscription_exists(bitmap_tx.txid(), 0);
    assert_inscription_count(1);

    // Bitmap should be registered
    let bitmap_id = InscriptionId::new(bitmap_tx.txid(), 0);
    assert_bitmap_registered(0, &bitmap_id);
}

/// Test: multiple blocks processed through all indexers in sequence
#[test]
fn test_e2e_multi_block_all_indexers() {
    clear();

    // Block 1: a bitmap inscription
    let height1 = 840000u32;
    let bitmap_content = b"0.bitmap";
    let bitmap_tx = create_inscription_transaction(bitmap_content, "text/plain", None);
    let block1 = create_block_with_txs(vec![
        create_coinbase_transaction(height1),
        bitmap_tx.clone(),
    ]);
    index_all(&block1, height1).unwrap();

    // Block 2: an SNS registration with unique outpoint
    let height2 = 840001u32;
    let sns_content = br#"{"p":"sns","op":"reg","name":"bob.btc"}"#;
    let sns_outpoint = create_mock_outpoint(1);
    let sns_tx = create_inscription_transaction(sns_content, "application/json", Some(sns_outpoint));
    let block2 = create_block_with_txs(vec![
        create_coinbase_transaction(height2),
        sns_tx.clone(),
    ]);
    index_all(&block2, height2).unwrap();

    // Block 3: empty block (coinbase only)
    let height3 = 840002u32;
    let block3 = create_block_with_coinbase_tx(height3);
    index_all(&block3, height3).unwrap();

    // Verify all inscriptions
    assert_inscription_count(2);
    assert_inscription_exists(bitmap_tx.txid(), 0);
    assert_inscription_exists(sns_tx.txid(), 0);

    // Verify bitmap
    let bitmap_id = InscriptionId::new(bitmap_tx.txid(), 0);
    assert_bitmap_registered(0, &bitmap_id);

    // Verify SNS
    let sns_id = InscriptionId::new(sns_tx.txid(), 0);
    assert_sns_registered("bob.btc", &sns_id);
}

/// Test: 5 empty blocks through all indexers without errors
#[test]
fn test_e2e_empty_blocks_no_errors() {
    clear();
    let start_height = 840000u32;
    let chain = create_test_chain(5, start_height);

    for (i, block) in chain.iter().enumerate() {
        let height = start_height + i as u32;
        index_all(block, height).unwrap();
    }

    // No inscriptions should have been created
    assert_inscription_count(0);
}

/// Test: BRC20 deploy + mint lifecycle across blocks, verify balances
#[test]
fn test_e2e_brc20_deploy_mint_lifecycle() {
    clear();
    let height0 = 0u32;

    // Deploy the BRC20 ticker
    let (deploy_block, deploy_tx) = create_brc20_deploy_block("life", "21000", "1000");
    index_ord_block(&deploy_block, height0).unwrap();
    index_brc20_block(&deploy_block, height0);

    // Verify the inscription was created
    assert_inscription_exists(deploy_tx.txid(), 0);
    assert_inscription_count(1);

    // Mint some tokens
    let test_address = shrew_test_helpers::state::get_test_address(1);
    let height1 = 1u32;
    let (mint_block, mint_tx) = create_brc20_mint_block("life", "500", &test_address, &deploy_tx.txid());
    index_ord_block(&mint_block, height1).unwrap();
    index_brc20_block(&mint_block, height1);

    // Verify the mint inscription was created
    assert_inscription_exists(mint_tx.txid(), 0);
    assert_inscription_count(2);
}

/// Test: bitmap first-wins semantics across blocks
#[test]
fn test_e2e_bitmap_first_wins_across_blocks() {
    clear();

    // Block 1: register bitmap 5
    let height1 = 840000u32;
    let (block1, tx1) = create_bitmap_inscription_block(5, height1);
    index_ord_block(&block1, height1).unwrap();
    index_bitmap_block(&block1, height1);

    let first_id = InscriptionId::new(tx1.txid(), 0);
    assert_bitmap_registered(5, &first_id);

    // Block 2: attempt to register bitmap 5 again with a unique outpoint
    let height2 = 840001u32;
    let dup_content = b"5.bitmap";
    let dup_outpoint = create_mock_outpoint(2);
    let dup_tx = create_inscription_transaction(dup_content, "text/plain", Some(dup_outpoint));
    let block2 = create_block_with_txs(vec![create_coinbase_transaction(height2), dup_tx.clone()]);
    index_ord_block(&block2, height2).unwrap();
    index_bitmap_block(&block2, height2);

    // The first inscription should still own bitmap 5
    let stored = BITMAP_NUMBER_TO_ID.select(&5u64.to_le_bytes().to_vec()).get();
    let stored_id = InscriptionId::from_bytes(&stored).unwrap();
    assert_eq!(stored_id, first_id, "First inscription should retain ownership of bitmap 5");
}

/// Test: SNS and bitmap protocols both work in the same block
#[test]
fn test_e2e_sns_and_bitmap_same_block() {
    clear();
    let height = 840000u32;

    // Create a bitmap inscription with default outpoint
    let bitmap_content = b"0.bitmap";
    let bitmap_tx = create_inscription_transaction(bitmap_content, "text/plain", None);

    // Create an SNS registration inscription with a unique outpoint
    let sns_content = br#"{"p":"sns","op":"reg","name":"charlie.btc"}"#;
    let sns_outpoint = create_mock_outpoint(3);
    let sns_tx = create_inscription_transaction(sns_content, "application/json", Some(sns_outpoint));

    let block = create_block_with_txs(vec![
        create_coinbase_transaction(height),
        bitmap_tx.clone(),
        sns_tx.clone(),
    ]);

    index_ord_block(&block, height).unwrap();
    index_bitmap_block(&block, height);
    index_sns_block(&block, height);

    // Both inscriptions should exist
    assert_inscription_count(2);
    assert_inscription_exists(bitmap_tx.txid(), 0);
    assert_inscription_exists(sns_tx.txid(), 0);

    // Bitmap should be registered
    let bitmap_id = InscriptionId::new(bitmap_tx.txid(), 0);
    assert_bitmap_registered(0, &bitmap_id);

    // SNS should be registered
    let sns_id = InscriptionId::new(sns_tx.txid(), 0);
    assert_sns_registered("charlie.btc", &sns_id);
}
