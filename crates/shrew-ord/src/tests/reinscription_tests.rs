///! Reinscription Curse Rule Tests
///!
///! These tests verify the ord protocol's reinscription curse rule:
///! - When a satoshi already has an inscription and a new inscription lands on
///!   the same sat, the new inscription is a "reinscription"
///! - Before the Jubilee height, reinscriptions are cursed (negative number)
///! - Reinscriptions always get the Charm::Reinscription flag
///! - After Jubilee, reinscriptions are vindicated (blessed but still flagged)
///!
///! The reinscription detection depends on sat tracking (SAT_TO_INSCRIPTIONS),
///! which requires proper sat calculation from UTXO ranges.

use crate::tables::*;
use metashrew_support::index_pointer::KeyValuePointer;
use shrew_support::inscription::{Charm, InscriptionEntry, InscriptionId};
use shrew_test_helpers::blocks::*;
use shrew_test_helpers::indexing::index_ord_block;
use shrew_test_helpers::inscriptions::*;
use shrew_test_helpers::state;
use shrew_test_helpers::transactions::*;
use std::str::FromStr;
use wasm_bindgen_test::wasm_bindgen_test;

/// Helper: create a transaction spending a specific outpoint with an inscription.
/// The inscription will land on the sat carried by the spent outpoint.
fn create_inscription_tx_spending(
    content: &[u8],
    content_type: &str,
    prev_txid: &str,
    prev_vout: u32,
) -> bitcoin::Transaction {
    let witness = create_inscription_envelope(content_type.as_bytes(), content);
    bitcoin::Transaction {
        version: bitcoin::transaction::Version(1),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint {
                txid: bitcoin::Txid::from_str(prev_txid).unwrap(),
                vout: prev_vout,
            },
            script_sig: bitcoin::ScriptBuf::new(),
            sequence: bitcoin::Sequence::MAX,
            witness,
        }],
        output: vec![bitcoin::TxOut {
            value: bitcoin::Amount::from_sat(100_000_000),
            script_pubkey: bitcoin::ScriptBuf::new(),
        }],
    }
}

// ============================================================================
// TEST 1: Two inscriptions on the same sat — second should be cursed pre-jubilee
//
// Scenario: First inscription created at height 100, second inscription
// created at height 200 spending the same output (same sat).
// The second inscription should be cursed because it's a reinscription.
// ============================================================================

#[wasm_bindgen_test]
fn test_reinscription_on_same_sat_is_cursed_before_jubilee() {
    state::clear();

    // Block 1: First inscription on a sat (non-coinbase, so it's blessed)
    let tx1 = create_inscription_transaction(b"First inscription on sat", "text/plain", None);
    let mut block1 = create_block_with_coinbase_tx(100);
    block1.txdata.push(tx1.clone());
    index_ord_block(&block1, 100).unwrap();

    // Verify first inscription is blessed
    let id1 = InscriptionId::new(tx1.compute_txid(), 0);
    let seq1 = INSCRIPTION_ID_TO_SEQUENCE.select(&id1.to_bytes()).get();
    assert!(!seq1.is_empty(), "First inscription should be indexed");
    let entry1 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq1).get()).unwrap();
    assert!(entry1.number > 0, "First inscription should be blessed");

    // Block 2: Second inscription spending first inscription's output (same sat)
    // This reinscribes on the same sat, so it should be cursed before jubilee
    let tx2 = create_inscription_tx_spending(
        b"Second inscription on same sat",
        "text/plain",
        &tx1.compute_txid().to_string(),
        0,
    );
    let mut block2 = create_block_with_coinbase_tx(200);
    block2.txdata.push(tx2.clone());
    index_ord_block(&block2, 200).unwrap();

    let id2 = InscriptionId::new(tx2.compute_txid(), 0);
    let seq2 = INSCRIPTION_ID_TO_SEQUENCE.select(&id2.to_bytes()).get();
    assert!(!seq2.is_empty(), "Second inscription should be indexed");
    let entry2 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq2).get()).unwrap();

    // The reinscription should be cursed (negative number) before jubilee
    assert!(
        entry2.number < 0,
        "Reinscription before jubilee should have negative number, got {}",
        entry2.number
    );
    assert!(
        entry2.has_charm(Charm::Cursed),
        "Reinscription before jubilee should have Cursed charm"
    );
}

// ============================================================================
// TEST 2: Reinscription should always have Charm::Reinscription set
// ============================================================================

#[wasm_bindgen_test]
fn test_reinscription_has_reinscription_charm() {
    state::clear();

    // First inscription
    let tx1 = create_inscription_transaction(b"Original", "text/plain", None);
    let mut block1 = create_block_with_coinbase_tx(100);
    block1.txdata.push(tx1.clone());
    index_ord_block(&block1, 100).unwrap();

    // Second inscription spending same output (reinscription)
    let tx2 = create_inscription_tx_spending(
        b"Reinscription",
        "text/plain",
        &tx1.compute_txid().to_string(),
        0,
    );
    let mut block2 = create_block_with_coinbase_tx(200);
    block2.txdata.push(tx2.clone());
    index_ord_block(&block2, 200).unwrap();

    let id2 = InscriptionId::new(tx2.compute_txid(), 0);
    let seq2 = INSCRIPTION_ID_TO_SEQUENCE.select(&id2.to_bytes()).get();
    let entry2 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq2).get()).unwrap();

    assert!(
        entry2.has_charm(Charm::Reinscription),
        "Reinscription should have Reinscription charm"
    );
}

// ============================================================================
// TEST 3: Reinscription after jubilee should be blessed (vindicated) but
// still have the Reinscription charm
// ============================================================================

#[wasm_bindgen_test]
fn test_reinscription_after_jubilee_is_vindicated() {
    state::clear();

    let jubilee = shrew_support::constants::JUBILEE_HEIGHT;

    // First inscription at jubilee height
    let tx1 = create_inscription_transaction(b"Pre-jubilee original", "text/plain", None);
    let mut block1 = create_block_with_coinbase_tx(jubilee);
    block1.txdata.push(tx1.clone());
    index_ord_block(&block1, jubilee).unwrap();

    // Second inscription at jubilee+1 spending same output
    let tx2 = create_inscription_tx_spending(
        b"Post-jubilee reinscription",
        "text/plain",
        &tx1.compute_txid().to_string(),
        0,
    );
    let mut block2 = create_block_with_coinbase_tx(jubilee + 1);
    block2.txdata.push(tx2.clone());
    index_ord_block(&block2, jubilee + 1).unwrap();

    let id2 = InscriptionId::new(tx2.compute_txid(), 0);
    let seq2 = INSCRIPTION_ID_TO_SEQUENCE.select(&id2.to_bytes()).get();
    let entry2 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq2).get()).unwrap();

    // After jubilee, reinscriptions are blessed (vindicated)
    assert!(
        entry2.number > 0,
        "Reinscription after jubilee should have positive number (vindicated), got {}",
        entry2.number
    );
    // But should still have the Reinscription charm
    assert!(
        entry2.has_charm(Charm::Reinscription),
        "Reinscription after jubilee should still have Reinscription charm"
    );
    // Should NOT have Cursed charm
    assert!(
        !entry2.has_charm(Charm::Cursed),
        "Reinscription after jubilee should NOT have Cursed charm"
    );
}

// ============================================================================
// TEST 4: First inscription on a sat should NOT have Reinscription charm
// ============================================================================

#[wasm_bindgen_test]
fn test_first_inscription_on_sat_not_reinscription() {
    state::clear();

    let tx = create_inscription_transaction(b"Only inscription on this sat", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(tx.compute_txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();

    assert!(
        !entry.has_charm(Charm::Reinscription),
        "First inscription on a sat should NOT have Reinscription charm"
    );
}

// ============================================================================
// TEST 5: SAT_TO_INSCRIPTIONS table should be populated
// ============================================================================

#[wasm_bindgen_test]
fn test_sat_to_inscriptions_populated() {
    state::clear();

    let tx = create_inscription_transaction(b"Track my sat", "text/plain", None);
    let mut block = create_block_with_coinbase_tx(100);
    block.txdata.push(tx.clone());
    index_ord_block(&block, 100).unwrap();

    let id = InscriptionId::new(tx.compute_txid(), 0);
    let seq = INSCRIPTION_ID_TO_SEQUENCE.select(&id.to_bytes()).get();
    let entry = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq).get()).unwrap();

    // If the inscription has a sat number, SAT_TO_INSCRIPTIONS should have it
    if let Some(sat) = entry.sat {
        let inscriptions_on_sat = SAT_TO_INSCRIPTIONS
            .select(&sat.to_le_bytes().to_vec())
            .get_list();
        assert!(
            !inscriptions_on_sat.is_empty(),
            "SAT_TO_INSCRIPTIONS should be populated when inscription has a sat number"
        );
    }
}

// ============================================================================
// TEST 6: Multiple reinscriptions — third inscription on same sat
// ============================================================================

#[wasm_bindgen_test]
fn test_multiple_reinscriptions_on_same_sat() {
    state::clear();

    // First inscription
    let tx1 = create_inscription_transaction(b"First", "text/plain", None);
    let mut block1 = create_block_with_coinbase_tx(100);
    block1.txdata.push(tx1.clone());
    index_ord_block(&block1, 100).unwrap();

    // Second inscription (first reinscription)
    let tx2 = create_inscription_tx_spending(
        b"Second",
        "text/plain",
        &tx1.compute_txid().to_string(),
        0,
    );
    let mut block2 = create_block_with_coinbase_tx(200);
    block2.txdata.push(tx2.clone());
    index_ord_block(&block2, 200).unwrap();

    // Third inscription (second reinscription) — spending tx2's output
    let tx3 = create_inscription_tx_spending(
        b"Third",
        "text/plain",
        &tx2.compute_txid().to_string(),
        0,
    );
    let mut block3 = create_block_with_coinbase_tx(300);
    block3.txdata.push(tx3.clone());
    index_ord_block(&block3, 300).unwrap();

    // Both second and third should be reinscriptions
    let id2 = InscriptionId::new(tx2.compute_txid(), 0);
    let seq2 = INSCRIPTION_ID_TO_SEQUENCE.select(&id2.to_bytes()).get();
    let entry2 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq2).get()).unwrap();

    let id3 = InscriptionId::new(tx3.compute_txid(), 0);
    let seq3 = INSCRIPTION_ID_TO_SEQUENCE.select(&id3.to_bytes()).get();
    let entry3 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq3).get()).unwrap();

    assert!(
        entry2.has_charm(Charm::Reinscription),
        "Second inscription should have Reinscription charm"
    );
    assert!(
        entry3.has_charm(Charm::Reinscription),
        "Third inscription should also have Reinscription charm"
    );

    // Both should be cursed (pre-jubilee)
    assert!(entry2.number < 0, "Second inscription should be cursed");
    assert!(entry3.number < 0, "Third inscription should be cursed");
}

// ============================================================================
// TEST 7: Reinscription numbering is correct — cursed numbers decrement
// ============================================================================

#[wasm_bindgen_test]
fn test_reinscription_cursed_numbering() {
    state::clear();

    // Blessed inscription first
    let tx1 = create_inscription_transaction(b"Blessed", "text/plain", None);
    let mut block1 = create_block_with_coinbase_tx(100);
    block1.txdata.push(tx1.clone());
    index_ord_block(&block1, 100).unwrap();

    let id1 = InscriptionId::new(tx1.compute_txid(), 0);
    let seq1 = INSCRIPTION_ID_TO_SEQUENCE.select(&id1.to_bytes()).get();
    let entry1 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq1).get()).unwrap();
    assert_eq!(entry1.number, 1, "First blessed inscription should be number 1");

    // Reinscription should be cursed with number -1
    let tx2 = create_inscription_tx_spending(
        b"Cursed reinscription",
        "text/plain",
        &tx1.compute_txid().to_string(),
        0,
    );
    let mut block2 = create_block_with_coinbase_tx(200);
    block2.txdata.push(tx2.clone());
    index_ord_block(&block2, 200).unwrap();

    let id2 = InscriptionId::new(tx2.compute_txid(), 0);
    let seq2 = INSCRIPTION_ID_TO_SEQUENCE.select(&id2.to_bytes()).get();
    let entry2 = InscriptionEntry::from_bytes(&SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq2).get()).unwrap();
    assert_eq!(
        entry2.number, -1,
        "First cursed reinscription should be number -1, got {}",
        entry2.number
    );
}
