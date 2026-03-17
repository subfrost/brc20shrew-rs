use wasm_bindgen_test::wasm_bindgen_test as test;
use crate::balance_sheet::{BalanceSheet, RuneId};

#[test]
fn test_balance_sheet_new_empty() {
    let sheet = BalanceSheet::new();
    assert!(sheet.is_empty());
    assert_eq!(sheet.balances.len(), 0);
}

#[test]
fn test_balance_sheet_credit() {
    let mut sheet = BalanceSheet::new();
    let rune = RuneId::new(840000, 1);
    sheet.credit(rune, 500);
    assert_eq!(sheet.get(&rune), 500);
    assert!(!sheet.is_empty());

    // Credit again to accumulate
    sheet.credit(rune, 300);
    assert_eq!(sheet.get(&rune), 800);
}

#[test]
fn test_balance_sheet_debit() {
    let mut sheet = BalanceSheet::new();
    let rune = RuneId::new(840000, 1);
    sheet.credit(rune, 1000);

    let ok = sheet.debit(rune, 400);
    assert!(ok);
    assert_eq!(sheet.get(&rune), 600);

    let ok = sheet.debit(rune, 600);
    assert!(ok);
    assert_eq!(sheet.get(&rune), 0);
    // After debiting to zero, the entry is removed
    assert!(sheet.is_empty());
}

#[test]
fn test_balance_sheet_debit_more_than_available() {
    let mut sheet = BalanceSheet::new();
    let rune = RuneId::new(840000, 2);
    sheet.credit(rune, 100);

    let ok = sheet.debit(rune, 200);
    assert!(!ok);
    // Balance unchanged after failed debit
    assert_eq!(sheet.get(&rune), 100);
}

#[test]
fn test_balance_sheet_multiple_runes() {
    let mut sheet = BalanceSheet::new();
    let rune_a = RuneId::new(840000, 0);
    let rune_b = RuneId::new(840001, 1);
    let rune_c = RuneId::new(840002, 3);

    sheet.credit(rune_a, 100);
    sheet.credit(rune_b, 200);
    sheet.credit(rune_c, 300);

    assert_eq!(sheet.get(&rune_a), 100);
    assert_eq!(sheet.get(&rune_b), 200);
    assert_eq!(sheet.get(&rune_c), 300);
    assert_eq!(sheet.balances.len(), 3);
}

#[test]
fn test_balance_sheet_merge() {
    let mut sheet_a = BalanceSheet::new();
    let mut sheet_b = BalanceSheet::new();
    let rune_x = RuneId::new(840000, 0);
    let rune_y = RuneId::new(840001, 1);

    sheet_a.credit(rune_x, 50);
    sheet_a.credit(rune_y, 100);

    sheet_b.credit(rune_x, 30);
    sheet_b.credit(rune_y, 20);

    sheet_a.merge(&sheet_b);
    assert_eq!(sheet_a.get(&rune_x), 80);
    assert_eq!(sheet_a.get(&rune_y), 120);

    // sheet_b should be unchanged
    assert_eq!(sheet_b.get(&rune_x), 30);
}

#[test]
fn test_balance_sheet_serialization_roundtrip() {
    let mut sheet = BalanceSheet::new();
    let rune_a = RuneId::new(840000, 0);
    let rune_b = RuneId::new(840001, 5);
    sheet.credit(rune_a, 999);
    sheet.credit(rune_b, 12345678);

    let bytes = sheet.to_bytes();
    assert!(!bytes.is_empty());

    let restored = BalanceSheet::from_bytes(&bytes).expect("deserialization should succeed");
    assert_eq!(restored.get(&rune_a), 999);
    assert_eq!(restored.get(&rune_b), 12345678);
    assert_eq!(restored.balances.len(), 2);
}

#[test]
fn test_balance_sheet_is_empty_after_debit() {
    let mut sheet = BalanceSheet::new();
    let rune = RuneId::new(840000, 0);
    sheet.credit(rune, 42);
    assert!(!sheet.is_empty());

    sheet.debit(rune, 42);
    assert!(sheet.is_empty());
}

#[test]
fn test_balance_sheet_get_nonexistent_rune() {
    let sheet = BalanceSheet::new();
    let rune = RuneId::new(999999, 99);
    assert_eq!(sheet.get(&rune), 0);
}
