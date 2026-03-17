use shrew_ord::tables::{
    INSCRIPTION_ID_TO_SEQUENCE, GLOBAL_SEQUENCE_COUNTER,
};
use shrew_brc20::tables::{Brc20Tickers, Brc20Balances};
use shrew_brc20::brc20::{Ticker, Balance};
use shrew_runes::balance_sheet::{BalanceSheet, RuneId};
use shrew_runes::tables::{RUNE_ID_TO_ENTRY, RUNE_BALANCES_BY_OUTPOINT};
use shrew_bitmap::tables::BITMAP_NUMBER_TO_ID;
use shrew_sns::tables::SNS_NAME_TO_ID;
use shrew_pow20::tables::POW20_BALANCES;
use shrew_support::inscription::InscriptionId;
use metashrew_support::index_pointer::KeyValuePointer;
use bitcoin::{OutPoint, Txid};
use bitcoin_hashes::Hash;

/// Assert that an inscription exists for the given txid and index
pub fn assert_inscription_exists(txid: Txid, index: u32) {
    let inscription_id = InscriptionId::new(txid, index);
    let seq_bytes = INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.to_bytes()).get();
    assert!(!seq_bytes.is_empty(), "Inscription {}i{} does not exist", txid, index);
}

/// Assert the total number of inscriptions indexed
pub fn assert_inscription_count(expected: u32) {
    let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
    if expected == 0 {
        assert!(seq_bytes.is_empty() || seq_bytes.iter().all(|&b| b == 0),
            "Expected 0 inscriptions but counter is non-zero");
        return;
    }
    assert!(!seq_bytes.is_empty(), "Expected {} inscriptions but counter is empty", expected);
    let count = u32::from_le_bytes(seq_bytes[..4].try_into().unwrap());
    assert_eq!(count, expected, "Expected {} inscriptions, got {}", expected, count);
}

/// Assert BRC20 balance for an owner+ticker
pub fn assert_brc20_balance(owner: &str, ticker: &str, expected_available: u128, expected_total: u128) {
    let table = Brc20Balances::new();
    let data = table.get(owner, ticker).expect(&format!("No BRC20 balance for {}:{}", owner, ticker));
    let balance: Balance = serde_json::from_slice(&data).expect("Failed to deserialize balance");
    assert_eq!(balance.available_balance, expected_available,
        "BRC20 available balance mismatch for {}:{}", owner, ticker);
    assert_eq!(balance.total_balance, expected_total,
        "BRC20 total balance mismatch for {}:{}", owner, ticker);
}

/// Assert BRC20 ticker supply
pub fn assert_brc20_supply(ticker: &str, expected_supply: u128) {
    let table = Brc20Tickers::new();
    let data = table.get(ticker).expect(&format!("BRC20 ticker {} not found", ticker));
    let entry: Ticker = serde_json::from_slice(&data).expect("Failed to deserialize ticker");
    assert_eq!(entry.current_supply, expected_supply,
        "BRC20 supply mismatch for {}: expected {}, got {}", ticker, expected_supply, entry.current_supply);
}

/// Assert rune balance at an outpoint
pub fn assert_rune_balance(outpoint: &OutPoint, rune_id: RuneId, expected_amount: u128) {
    let outpoint_bytes: Vec<u8> = outpoint.txid.as_byte_array().iter()
        .chain(outpoint.vout.to_le_bytes().iter()).copied().collect();
    let data = RUNE_BALANCES_BY_OUTPOINT.select(&outpoint_bytes).get();
    let sheet = if data.is_empty() {
        BalanceSheet::new()
    } else {
        BalanceSheet::from_bytes(&data).unwrap_or_default()
    };
    let actual = sheet.get(&rune_id);
    assert_eq!(actual, expected_amount,
        "Rune balance mismatch at {:?} for {:?}: expected {}, got {}", outpoint, rune_id, expected_amount, actual);
}

/// Assert rune entry exists with expected name and supply
pub fn assert_rune_entry(rune_id: RuneId, expected_name: &str, expected_supply: u128) {
    let data = RUNE_ID_TO_ENTRY.select(&rune_id.to_bytes()).get();
    assert!(!data.is_empty(), "RuneEntry for {:?} not found", rune_id);
    let entry: shrew_runes::rune_indexer::RuneEntry = bincode::deserialize(&data)
        .expect("Failed to deserialize RuneEntry");
    assert_eq!(entry.name, expected_name, "Rune name mismatch");
    assert_eq!(entry.supply, expected_supply, "Rune supply mismatch");
}

/// Assert bitmap registered for a number
pub fn assert_bitmap_registered(number: u64, inscription_id: &InscriptionId) {
    let data = BITMAP_NUMBER_TO_ID.select(&number.to_le_bytes().to_vec()).get();
    assert!(!data.is_empty(), "Bitmap {} not registered", number);
    let stored_id = InscriptionId::from_bytes(&data).expect("Invalid stored inscription id");
    assert_eq!(stored_id, *inscription_id, "Bitmap {} registered with wrong inscription", number);
}

/// Assert SNS name is registered
pub fn assert_sns_registered(name: &str, inscription_id: &InscriptionId) {
    let data = SNS_NAME_TO_ID.select(&name.as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "SNS name '{}' not registered", name);
    let stored_id = InscriptionId::from_bytes(&data).expect("Invalid stored inscription id");
    assert_eq!(stored_id, *inscription_id, "SNS name '{}' registered with wrong inscription", name);
}

/// Assert POW20 balance
pub fn assert_pow20_balance(owner: &str, ticker: &str, expected: u64) {
    let key = format!("{}:{}", owner, ticker);
    let data = POW20_BALANCES.select(&key.as_bytes().to_vec()).get();
    assert!(!data.is_empty(), "No POW20 balance for {}", key);
    let balance: shrew_pow20::pow20_indexer::Pow20Balance = serde_json::from_slice(&data)
        .expect("Failed to deserialize POW20 balance");
    assert_eq!(balance.available_balance, expected,
        "POW20 balance mismatch for {}: expected {}, got {}", key, expected, balance.available_balance);
}
