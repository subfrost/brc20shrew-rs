use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;

// Create individual IndexPointer instances directly
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
    
    // Sat tracking (for sat index)
    pub static ref SAT_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/sat_to_inscriptions/");
    pub static ref INSCRIPTION_TO_SAT: IndexPointer = IndexPointer::from_keyword("/inscriptions/inscription_to_sat/");
    
    // Transaction tracking
    pub static ref TXID_TO_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/inscriptions/txid_to_inscriptions/");
    pub static ref INSCRIPTION_TO_TXID: IndexPointer = IndexPointer::from_keyword("/inscriptions/inscription_to_txid/");
    
    // Address tracking (for address index)
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

    // BRC20 Tables
    pub static ref BRC20_TICKERS: IndexPointer = IndexPointer::from_keyword("/brc20/tickers/");
    pub static ref BRC20_BALANCES: IndexPointer = IndexPointer::from_keyword("/brc20/balances/");
    pub static ref BRC20_EVENTS: IndexPointer = IndexPointer::from_keyword("/brc20/events/");
   pub static ref BRC20_TRANSFERABLE_INSCRIPTIONS: IndexPointer = IndexPointer::from_keyword("/brc20/transferable/");

   // BRC-20 Programmable Module Tables
   pub static ref EVM_ACCOUNTS: IndexPointer = IndexPointer::from_keyword("/prog/accounts/");
   pub static ref EVM_STORAGE: IndexPointer = IndexPointer::from_keyword("/prog/storage/");
   pub static ref CONTRACT_ADDRESS_TO_INSCRIPTION_ID: IndexPointer = IndexPointer::from_keyword("/prog/contract_to_id/");
   pub static ref CODE_HASH_TO_BYTECODE: IndexPointer = IndexPointer::from_keyword("/prog/code_hash_to_bytecode/");
   pub static ref INSCRIPTION_ID_TO_CONTRACT_ADDRESS: IndexPointer = IndexPointer::from_keyword("/prog/id_to_contract/");
}

/// Table wrapper structs for easier access in tests and indexing
pub struct InscriptionTable;
pub struct InscriptionContentTable;
pub struct InscriptionContentTypeTable;
pub struct InscriptionLocationTable;
pub struct InscriptionMetadataTable;
pub struct InscriptionParentTable;
pub struct InscriptionChildrenTable;
pub struct InscriptionDelegateTable;
pub struct InscriptionNumberTable;
pub struct InscriptionSatTable;
pub struct CursedInscriptionTable;

pub struct Brc20Tickers;
pub struct Brc20Balances;
pub struct Brc20EventsTable;
pub struct Brc20TransferableInscriptions;

impl InscriptionTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, data: &[u8]) {
        let mut pointer = INSCRIPTION_ID_TO_SEQUENCE.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(data.to_vec()));
    }
}

impl InscriptionContentTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = INSCRIPTION_CONTENT.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, content: &[u8]) {
        let mut pointer = INSCRIPTION_CONTENT.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(content.to_vec()));
    }
}

impl InscriptionContentTypeTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        // Use a dedicated content type table
        let content_type_table = IndexPointer::from_keyword("/inscriptions/content_types/");
        let pointer = content_type_table.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, content_type: &[u8]) {
        let content_type_table = IndexPointer::from_keyword("/inscriptions/content_types/");
        let mut pointer = content_type_table.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(content_type.to_vec()));
    }
}

impl InscriptionLocationTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<String> {
        let pointer = SEQUENCE_TO_SATPOINT.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            String::from_utf8((*result).clone()).ok()
        }
    }
    
    pub fn set(&self, inscription_id: &str, satpoint: &str) {
        let mut pointer = SEQUENCE_TO_SATPOINT.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(satpoint.as_bytes().to_vec()));
    }
}

impl InscriptionMetadataTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = INSCRIPTION_METADATA.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, metadata: &[u8]) {
        let mut pointer = INSCRIPTION_METADATA.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(metadata.to_vec()));
    }
}

impl InscriptionParentTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<String> {
        let pointer = SEQUENCE_TO_PARENTS.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            String::from_utf8((*result).clone()).ok()
        }
    }
    
    pub fn set(&self, inscription_id: &str, parent_id: &str) {
        let mut pointer = SEQUENCE_TO_PARENTS.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(parent_id.as_bytes().to_vec()));
    }
}

impl InscriptionChildrenTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = SEQUENCE_TO_CHILDREN.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, children: &[u8]) {
        let mut pointer = SEQUENCE_TO_CHILDREN.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(children.to_vec()));
    }
}

impl InscriptionDelegateTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<String> {
        let pointer = INSCRIPTION_TO_DELEGATE.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            String::from_utf8((*result).clone()).ok()
        }
    }
    
    pub fn set(&self, inscription_id: &str, delegate_id: &str) {
        let mut pointer = INSCRIPTION_TO_DELEGATE.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(delegate_id.as_bytes().to_vec()));
    }
}

impl InscriptionNumberTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let number_table = IndexPointer::from_keyword("/inscriptions/numbers/");
        let pointer = number_table.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, number: u64) {
        let number_table = IndexPointer::from_keyword("/inscriptions/numbers/");
        let number_bytes = serde_json::to_vec(&number).unwrap();
        let mut pointer = number_table.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(number_bytes));
    }
}

impl InscriptionSatTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let pointer = INSCRIPTION_TO_SAT.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, sat: u64) {
        let sat_bytes = serde_json::to_vec(&sat).unwrap();
        let mut pointer = INSCRIPTION_TO_SAT.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(sat_bytes));
    }
}


impl Brc20Tickers {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, ticker: &str) -> Option<Vec<u8>> {
        let pointer = BRC20_TICKERS.select(&ticker.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }

    pub fn set(&self, ticker: &str, data: &[u8]) {
        let mut pointer = BRC20_TICKERS.select(&ticker.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(data.to_vec()));
    }
}

impl Brc20Balances {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, address: &str, ticker: &str) -> Option<Vec<u8>> {
        let key = format!("{}:{}", address, ticker);
        let pointer = BRC20_BALANCES.select(&key.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }

    pub fn set(&self, address: &str, ticker: &str, data: &[u8]) {
        let key = format!("{}:{}", address, ticker);
        let mut pointer = BRC20_BALANCES.select(&key.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(data.to_vec()));
    }
}

impl Brc20TransferableInscriptions {
   pub fn new() -> Self {
       Self
   }

   pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
       let pointer = BRC20_TRANSFERABLE_INSCRIPTIONS.select(&inscription_id.as_bytes().to_vec());
       let result = pointer.get();
       if result.is_empty() {
           None
       } else {
           Some((*result).clone())
       }
   }

   pub fn set(&self, inscription_id: &str, data: &[u8]) {
       let mut pointer = BRC20_TRANSFERABLE_INSCRIPTIONS.select(&inscription_id.as_bytes().to_vec());
       pointer.set(std::sync::Arc::new(data.to_vec()));
   }

   pub fn delete(&self, inscription_id: &str) {
       let mut pointer = BRC20_TRANSFERABLE_INSCRIPTIONS.select(&inscription_id.as_bytes().to_vec());
       pointer.set(std::sync::Arc::new(vec![]));
   }
}

impl Brc20EventsTable {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, tx_id: &str) -> Option<Vec<u8>> {
        let pointer = BRC20_EVENTS.select(&tx_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }

    pub fn set(&self, tx_id: &str, data: &[u8]) {
        let mut pointer = BRC20_EVENTS.select(&tx_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(data.to_vec()));
    }
}

impl CursedInscriptionTable {
    pub fn new() -> Self {
        Self
    }
    
    pub fn get(&self, inscription_id: &str) -> Option<Vec<u8>> {
        let cursed_table = IndexPointer::from_keyword("/inscriptions/cursed/");
        let pointer = cursed_table.select(&inscription_id.as_bytes().to_vec());
        let result = pointer.get();
        if result.is_empty() {
            None
        } else {
            Some((*result).clone())
        }
    }
    
    pub fn set(&self, inscription_id: &str, is_cursed: bool) {
        let cursed_table = IndexPointer::from_keyword("/inscriptions/cursed/");
        let cursed_bytes = serde_json::to_vec(&is_cursed).unwrap();
        let mut pointer = cursed_table.select(&inscription_id.as_bytes().to_vec());
        pointer.set(std::sync::Arc::new(cursed_bytes));
    }
}