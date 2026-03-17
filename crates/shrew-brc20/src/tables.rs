use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref BRC20_TICKERS: IndexPointer = IndexPointer::from_keyword("/brc20/tickers/");
    pub static ref BRC20_BALANCES: IndexPointer = IndexPointer::from_keyword("/brc20/balances/");
    pub static ref BRC20_EVENTS: IndexPointer = IndexPointer::from_keyword("/brc20/events/");
    pub static ref BRC20_TRANSFERABLE_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/brc20/transferable/");
    /// Pending BRC20-PROG deposit events. Written by BRC-20 indexer, consumed by prog indexer.
    /// Key: height (u32 LE), Value: JSON array of DepositEvent
    pub static ref BRC20_PROG_PENDING_DEPOSITS: IndexPointer = IndexPointer::from_keyword("/brc20/prog_deposits/");
}

/// A pending BRC20-PROG deposit event. Recorded when tokens are sent to the
/// BRC20-PROG OP_RETURN address, consumed by the prog indexer to call controller_mint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepositEvent {
    pub ticker: String,
    pub amount: u128,
    pub sender: String,
}

pub struct Brc20ProgDeposits;

impl Brc20ProgDeposits {
    pub fn new() -> Self { Self }

    /// Append a deposit event for the given block height.
    pub fn push(&self, height: u32, event: &DepositEvent) {
        let key = height.to_le_bytes().to_vec();
        let pointer = BRC20_PROG_PENDING_DEPOSITS.select(&key);
        let existing = pointer.get();
        let mut events: Vec<DepositEvent> = if existing.is_empty() {
            Vec::new()
        } else {
            serde_json::from_slice(&existing).unwrap_or_default()
        };
        events.push(event.clone());
        let mut pointer = BRC20_PROG_PENDING_DEPOSITS.select(&key);
        pointer.set(Arc::new(serde_json::to_vec(&events).unwrap_or_default()));
    }

    /// Get all pending deposit events for a block height.
    pub fn get(&self, height: u32) -> Vec<DepositEvent> {
        let key = height.to_le_bytes().to_vec();
        let pointer = BRC20_PROG_PENDING_DEPOSITS.select(&key);
        let result = pointer.get();
        if result.is_empty() {
            Vec::new()
        } else {
            serde_json::from_slice(&result).unwrap_or_default()
        }
    }

    /// Clear deposit events for a block height (after processing).
    pub fn clear(&self, height: u32) {
        let key = height.to_le_bytes().to_vec();
        let mut pointer = BRC20_PROG_PENDING_DEPOSITS.select(&key);
        pointer.set(Arc::new(vec![]));
    }
}

pub struct Brc20Tickers;
pub struct Brc20Balances;
pub struct Brc20EventsTable;
pub struct Brc20TransferableInscriptions;

impl Brc20Tickers {
    pub fn new() -> Self { Self }
    pub fn get(&self, ticker: &str) -> Option<Vec<u8>> {
        let pointer = BRC20_TICKERS.select(&ticker.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
    pub fn set(&self, ticker: &str, data: &[u8]) {
        let mut pointer = BRC20_TICKERS.select(&ticker.as_bytes().to_vec());
        pointer.set(Arc::new(data.to_vec()));
    }
}

impl Brc20Balances {
    pub fn new() -> Self { Self }
    pub fn get(&self, address: &str, ticker: &str) -> Option<Vec<u8>> {
        let key = format!("{}:{}", address, ticker);
        let pointer = BRC20_BALANCES.select(&key.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
    pub fn set(&self, address: &str, ticker: &str, data: &[u8]) {
        let key = format!("{}:{}", address, ticker);
        let mut pointer = BRC20_BALANCES.select(&key.as_bytes().to_vec());
        pointer.set(Arc::new(data.to_vec()));
    }
}

impl Brc20TransferableInscriptions {
    pub fn new() -> Self { Self }
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = BRC20_TRANSFERABLE_INSCRIPTIONS.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
    pub fn set(&self, inscription_id: &str, data: &[u8]) {
        let mut pointer = BRC20_TRANSFERABLE_INSCRIPTIONS.select(&inscription_id.as_bytes().to_vec());
        pointer.set(Arc::new(data.to_vec()));
    }
    pub fn delete(&self, inscription_id: &str) {
        let mut pointer = BRC20_TRANSFERABLE_INSCRIPTIONS.select(&inscription_id.as_bytes().to_vec());
        pointer.set(Arc::new(vec![]));
    }
}

impl Brc20EventsTable {
    pub fn new() -> Self { Self }
    pub fn get(&self, tx_id: &str) -> Option<Vec<u8>> {
        let pointer = BRC20_EVENTS.select(&tx_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
    pub fn set(&self, tx_id: &str, data: &[u8]) {
        let mut pointer = BRC20_EVENTS.select(&tx_id.as_bytes().to_vec());
        pointer.set(Arc::new(data.to_vec()));
    }
}
