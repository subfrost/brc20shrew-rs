use metashrew::index_pointer::IndexPointer;
use crate::bst::BST;
use metashrew_support::index_pointer::KeyValuePointer;
use once_cell::sync::Lazy;
use std::sync::RwLock;

#[derive(Default, Clone)]
pub struct InscriptionTable {
    pub sat_to_outpoint: BST<IndexPointer>,
    pub outpoint_to_sat: IndexPointer,
    pub outpoint_to_value: IndexPointer,
    pub outpoint_to_sequence_numbers: IndexPointer,
    pub height_to_blockhash: IndexPointer,
    pub blockhash_to_height: IndexPointer,
    pub starting_sat: IndexPointer,
    pub inscription_id_to_inscription: IndexPointer,
    pub satpoint_to_inscription_id: IndexPointer,
    pub satpoint_to_sat: IndexPointer,
    pub inscription_id_to_satpoint: IndexPointer,
    pub inscription_id_to_blockheight: IndexPointer,
    pub height_to_inscription_ids: IndexPointer,
    pub next_sequence_number: IndexPointer,
    pub sequence_number_to_inscription_id: IndexPointer,
    pub inscription_id_to_sequence_number: IndexPointer,
}

impl InscriptionTable {
    pub fn new() -> Self {
        InscriptionTable {
            sat_to_outpoint: BST::at(IndexPointer::from_keyword("/outpoint/bysatrange/")),
            outpoint_to_sat: IndexPointer::from_keyword("/sat/by/outpoint/"),
            outpoint_to_value: IndexPointer::from_keyword("/value/byoutpoint/"),
            outpoint_to_sequence_numbers: IndexPointer::from_keyword("/sequencenumbers/byoutpoint"),
            height_to_blockhash: IndexPointer::from_keyword("/blockhash/byheight/"),
            blockhash_to_height: IndexPointer::from_keyword("/height/byblockhash/"),
            starting_sat: IndexPointer::from_keyword("/startingsat"),
            inscription_id_to_inscription: IndexPointer::from_keyword("/inscription/byid/"),
            satpoint_to_inscription_id: IndexPointer::from_keyword("/inscriptionid/bysatpoint"),
            satpoint_to_sat: IndexPointer::from_keyword("/sat/bysatpoint"),
            inscription_id_to_satpoint: IndexPointer::from_keyword("/satpoint/byinscriptionid/"),
            inscription_id_to_blockheight: IndexPointer::from_keyword("/height/byinscription/"),
            height_to_inscription_ids: IndexPointer::from_keyword("/inscriptionids/byheight/"),
            next_sequence_number: IndexPointer::from_keyword("/nextsequence"),
            sequence_number_to_inscription_id: IndexPointer::from_keyword("/inscriptionid/bysequence/"),
            inscription_id_to_sequence_number: IndexPointer::from_keyword("/sequence/byinscriptionid/"),
        }
    }
}

pub static INSCRIPTIONS: Lazy<RwLock<InscriptionTable>> = Lazy::new(|| RwLock::new(InscriptionTable::new()));
