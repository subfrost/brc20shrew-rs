use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;

lazy_static::lazy_static! {
    /// Rune ID -> RuneEntry metadata (serialized)
    pub static ref RUNE_ID_TO_ENTRY: IndexPointer = IndexPointer::from_keyword("/runes/id_to_entry/");

    /// Etching txid -> RuneId
    pub static ref ETCHING_TO_RUNE_ID: IndexPointer = IndexPointer::from_keyword("/runes/etching_to_id/");

    /// RuneId -> Etching txid
    pub static ref RUNE_ID_TO_ETCHING: IndexPointer = IndexPointer::from_keyword("/runes/id_to_etching/");

    /// Rune name (uppercased, no spacers) -> RuneId
    pub static ref RUNE_NAME_TO_ID: IndexPointer = IndexPointer::from_keyword("/runes/name_to_id/");

    /// Outpoint -> BalanceSheet (list of rune balances)
    pub static ref RUNE_BALANCES_BY_OUTPOINT: IndexPointer = IndexPointer::from_keyword("/runes/byoutpoint/");

    /// RuneId -> remaining mints
    pub static ref RUNE_MINTS_REMAINING: IndexPointer = IndexPointer::from_keyword("/runes/mints_remaining/");

    /// RuneId -> total minted count
    pub static ref RUNE_MINTED_COUNT: IndexPointer = IndexPointer::from_keyword("/runes/minted_count/");

    /// RuneId -> cap
    pub static ref RUNE_CAP: IndexPointer = IndexPointer::from_keyword("/runes/cap/");

    /// RuneId -> divisibility
    pub static ref RUNE_DIVISIBILITY: IndexPointer = IndexPointer::from_keyword("/runes/divisibility/");

    /// RuneId -> symbol
    pub static ref RUNE_SYMBOL: IndexPointer = IndexPointer::from_keyword("/runes/symbol/");

    /// Height -> list of rune events
    pub static ref HEIGHT_TO_RUNE_EVENTS: IndexPointer = IndexPointer::from_keyword("/runes/height_to_events/");

    /// Global rune counter
    pub static ref RUNE_COUNTER: IndexPointer = IndexPointer::from_keyword("/runes/counter");
}
