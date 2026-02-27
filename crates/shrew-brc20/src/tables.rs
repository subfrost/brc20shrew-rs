use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref BRC20_TICKERS: IndexPointer = IndexPointer::from_keyword("/brc20/tickers/");
    pub static ref BRC20_BALANCES: IndexPointer = IndexPointer::from_keyword("/brc20/balances/");
    pub static ref BRC20_EVENTS: IndexPointer = IndexPointer::from_keyword("/brc20/events/");
    pub static ref BRC20_TRANSFERABLE_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/brc20/transferable/");
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
