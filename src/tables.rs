use metashrew_core::index_pointer::IndexPointer;
use once_cell::sync::Lazy;

#[derive(Default, Clone)]
pub struct InscriptionTables {
    // Core mappings
    pub INSCRIPTION_ID_TO_SEQUENCE: IndexPointer,
    pub SEQUENCE_TO_INSCRIPTION_ENTRY: IndexPointer,
    pub INSCRIPTION_NUMBER_TO_SEQUENCE: IndexPointer,
    
    // Location tracking
    pub SEQUENCE_TO_SATPOINT: IndexPointer,
    pub SAT_TO_SEQUENCE: IndexPointer,
    pub OUTPOINT_TO_INSCRIPTIONS: IndexPointer,
    
    // Hierarchical relationships
    pub SEQUENCE_TO_CHILDREN: IndexPointer,
    pub SEQUENCE_TO_PARENTS: IndexPointer,
    
    // Block and height indexing
    pub HEIGHT_TO_INSCRIPTIONS: IndexPointer,
    pub HEIGHT_TO_BLOCK_HASH: IndexPointer,
    pub BLOCK_HASH_TO_HEIGHT: IndexPointer,
    
    // Content and metadata indexing
    pub CONTENT_TYPE_TO_INSCRIPTIONS: IndexPointer,
    pub METAPROTOCOL_TO_INSCRIPTIONS: IndexPointer,
    
    // Statistics and counters
    pub GLOBAL_SEQUENCE_COUNTER: IndexPointer,
    pub BLESSED_INSCRIPTION_COUNTER: IndexPointer,
    pub CURSED_INSCRIPTION_COUNTER: IndexPointer,
    
    // Special collections
    pub HOME_INSCRIPTIONS: IndexPointer,
    pub COLLECTIONS: IndexPointer,
    
    // Sat tracking (for sat index)
    pub SAT_TO_INSCRIPTIONS: IndexPointer,
    pub INSCRIPTION_TO_SAT: IndexPointer,
    
    // Transaction tracking
    pub TXID_TO_INSCRIPTIONS: IndexPointer,
    pub INSCRIPTION_TO_TXID: IndexPointer,
    
    // Address tracking (for address index)
    pub ADDRESS_TO_INSCRIPTIONS: IndexPointer,
    pub INSCRIPTION_TO_ADDRESS: IndexPointer,
    
    // Rune tracking
    pub RUNE_TO_INSCRIPTIONS: IndexPointer,
    pub INSCRIPTION_TO_RUNE: IndexPointer,
    
    // Content storage
    pub INSCRIPTION_CONTENT: IndexPointer,
    pub INSCRIPTION_METADATA: IndexPointer,
    
    // Delegation tracking
    pub DELEGATE_TO_INSCRIPTIONS: IndexPointer,
    pub INSCRIPTION_TO_DELEGATE: IndexPointer,
}

impl InscriptionTables {
    pub fn new() -> Self {
        InscriptionTables {
            INSCRIPTION_ID_TO_SEQUENCE: IndexPointer::from_keyword("/inscriptions/id_to_seq/"),
            SEQUENCE_TO_INSCRIPTION_ENTRY: IndexPointer::from_keyword("/inscriptions/seq_to_entry/"),
            INSCRIPTION_NUMBER_TO_SEQUENCE: IndexPointer::from_keyword("/inscriptions/num_to_seq/"),
            
            SEQUENCE_TO_SATPOINT: IndexPointer::from_keyword("/inscriptions/seq_to_satpoint/"),
            SAT_TO_SEQUENCE: IndexPointer::from_keyword("/inscriptions/sat_to_seq/"),
            OUTPOINT_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/outpoint_to_list/"),
            
            SEQUENCE_TO_CHILDREN: IndexPointer::from_keyword("/inscriptions/seq_to_children/"),
            SEQUENCE_TO_PARENTS: IndexPointer::from_keyword("/inscriptions/seq_to_parents/"),
            
            HEIGHT_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/height_to_list/"),
            HEIGHT_TO_BLOCK_HASH: IndexPointer::from_keyword("/inscriptions/height_to_hash/"),
            BLOCK_HASH_TO_HEIGHT: IndexPointer::from_keyword("/inscriptions/hash_to_height/"),
            
            CONTENT_TYPE_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/content_type/"),
            METAPROTOCOL_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/metaprotocol/"),
            
            GLOBAL_SEQUENCE_COUNTER: IndexPointer::from_keyword("/inscriptions/counters/sequence"),
            BLESSED_INSCRIPTION_COUNTER: IndexPointer::from_keyword("/inscriptions/counters/blessed"),
            CURSED_INSCRIPTION_COUNTER: IndexPointer::from_keyword("/inscriptions/counters/cursed"),
            
            HOME_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/home/"),
            COLLECTIONS: IndexPointer::from_keyword("/inscriptions/collections/"),
            
            SAT_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/sat_to_inscriptions/"),
            INSCRIPTION_TO_SAT: IndexPointer::from_keyword("/inscriptions/inscription_to_sat/"),
            
            TXID_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/txid_to_inscriptions/"),
            INSCRIPTION_TO_TXID: IndexPointer::from_keyword("/inscriptions/inscription_to_txid/"),
            
            ADDRESS_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/address_to_inscriptions/"),
            INSCRIPTION_TO_ADDRESS: IndexPointer::from_keyword("/inscriptions/inscription_to_address/"),
            
            RUNE_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/rune_to_inscriptions/"),
            INSCRIPTION_TO_RUNE: IndexPointer::from_keyword("/inscriptions/inscription_to_rune/"),
            
            INSCRIPTION_CONTENT: IndexPointer::from_keyword("/inscriptions/content/"),
            INSCRIPTION_METADATA: IndexPointer::from_keyword("/inscriptions/metadata/"),
            
            DELEGATE_TO_INSCRIPTIONS: IndexPointer::from_keyword("/inscriptions/delegate_to_inscriptions/"),
            INSCRIPTION_TO_DELEGATE: IndexPointer::from_keyword("/inscriptions/inscription_to_delegate/"),
        }
    }
}

pub static TABLES: Lazy<InscriptionTables> = Lazy::new(|| InscriptionTables::new());