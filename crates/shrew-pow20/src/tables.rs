use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;

lazy_static::lazy_static! {
    pub static ref POW20_TICKERS: IndexPointer = IndexPointer::from_keyword("/pow20/tickers/");
    pub static ref POW20_BALANCES: IndexPointer = IndexPointer::from_keyword("/pow20/balances/");
    pub static ref POW20_EVENTS: IndexPointer = IndexPointer::from_keyword("/pow20/events/");
    pub static ref POW20_TRANSFERABLE: IndexPointer = IndexPointer::from_keyword("/pow20/transferable/");
}
