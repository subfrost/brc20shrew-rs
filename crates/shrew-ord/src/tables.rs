use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;

lazy_static::lazy_static! {
    // Core mappings
    pub static ref INSCRIPTION_ID_TO_SEQUENCE: IndexPointer = IndexPointer::from_keyword("/inscriptions/id_to_seq/");
    pub static ref SEQUENCE_TO_INSCRIPTION_ENTRY: IndexPointer = IndexPointer::from_keyword("/inscriptions/seq_to_entry/");
    pub static ref INSCRIPTION_NUMBER_TO_SEQUENCE: IndexPointer = IndexPointer::from_keyword("/inscriptions/num_to_seq/");

    // Location tracking
    pub static ref SEQUENCE_TO_SATPOINT: IndexPointer = IndexPointer::from_keyword("/inscriptions/seq_to_satpoint/");
    pub static ref SAT_TO_SEQUENCE: IndexPointer = IndexPointer::from_keyword("/inscriptions/sat_to_seq/");
    pub static ref OUTPOINT_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/outpoint_to_list/");

    // Hierarchical relationships
    pub static ref SEQUENCE_TO_CHILDREN: IndexPointer = IndexPointer::from_keyword("/inscriptions/seq_to_children/");
    pub static ref SEQUENCE_TO_PARENTS: IndexPointer = IndexPointer::from_keyword("/inscriptions/seq_to_parents/");

    // Block and height indexing
    pub static ref HEIGHT_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/height_to_list/");
    pub static ref HEIGHT_TO_BLOCK_HASH: IndexPointer = IndexPointer::from_keyword("/inscriptions/height_to_hash/");
    pub static ref BLOCK_HASH_TO_HEIGHT: IndexPointer = IndexPointer::from_keyword("/inscriptions/hash_to_height/");

    // Content and metadata indexing
    pub static ref CONTENT_TYPE_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/content_type/");
    pub static ref METAPROTOCOL_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/metaprotocol/");

    // Statistics and counters
    pub static ref GLOBAL_SEQUENCE_COUNTER: IndexPointer = IndexPointer::from_keyword("/inscriptions/counters/sequence");
    pub static ref BLESSED_INSCRIPTION_COUNTER: IndexPointer = IndexPointer::from_keyword("/inscriptions/counters/blessed");
    pub static ref CURSED_INSCRIPTION_COUNTER: IndexPointer = IndexPointer::from_keyword("/inscriptions/counters/cursed");

    // Special collections
    pub static ref HOME_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/home/");
    pub static ref COLLECTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/collections/");

    // Sat tracking
    pub static ref SAT_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/sat_to_inscriptions/");
    pub static ref INSCRIPTION_TO_SAT: IndexPointer = IndexPointer::from_keyword("/inscriptions/inscription_to_sat/");

    // Transaction tracking
    pub static ref TXID_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/txid_to_inscriptions/");
    pub static ref INSCRIPTION_TO_TXID: IndexPointer = IndexPointer::from_keyword("/inscriptions/inscription_to_txid/");

    // Address tracking
    pub static ref ADDRESS_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/address_to_inscriptions/");
    pub static ref INSCRIPTION_TO_ADDRESS: IndexPointer = IndexPointer::from_keyword("/inscriptions/inscription_to_address/");

    // Rune tracking
    pub static ref RUNE_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/rune_to_inscriptions/");
    pub static ref INSCRIPTION_TO_RUNE: IndexPointer = IndexPointer::from_keyword("/inscriptions/inscription_to_rune/");

    // Content storage
    pub static ref INSCRIPTION_CONTENT: IndexPointer = IndexPointer::from_keyword("/inscriptions/content/");
    pub static ref INSCRIPTION_METADATA: IndexPointer = IndexPointer::from_keyword("/inscriptions/metadata/");

    // Delegation tracking
    pub static ref DELEGATE_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/delegate_to_inscriptions/");
    pub static ref INSCRIPTION_TO_DELEGATE: IndexPointer = IndexPointer::from_keyword("/inscriptions/inscription_to_delegate/");
}

/// Table wrapper structs for easier access
pub struct InscriptionContentTable;
pub struct InscriptionContentTypeTable;
pub struct InscriptionMetadataTable;
pub struct InscriptionParentTable;
pub struct InscriptionNumberTable;

impl InscriptionContentTable {
    pub fn new() -> Self { Self }
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = INSCRIPTION_CONTENT.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
    pub fn set(&self, inscription_id: &str, content: &[u8]) {
        let mut pointer = INSCRIPTION_CONTENT.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(content.to_vec()));
    }
}

impl InscriptionContentTypeTable {
    pub fn new() -> Self { Self }
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let content_type_table = IndexPointer::from_keyword("/inscriptions/content_types/");
        let pointer = content_type_table.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
}

impl InscriptionMetadataTable {
    pub fn new() -> Self { Self }
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = INSCRIPTION_METADATA.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
}

impl InscriptionParentTable {
    pub fn new() -> Self { Self }
    pub fn get(&self, inscription_id: &str) -> Option<String> {
        let pointer = SEQUENCE_TO_PARENTS.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { String::from_utf8((*result).clone()).ok() }
    }
}

impl InscriptionNumberTable {
    pub fn new() -> Self { Self }
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let number_table = IndexPointer::from_keyword("/inscriptions/numbers/");
        let pointer = number_table.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() { None } else { Some((*result).clone()) }
    }
}
