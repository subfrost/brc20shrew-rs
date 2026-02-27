use metashrew_core::index_pointer::IndexPointer;
use metashrew_support::index_pointer::KeyValuePointer;

lazy_static::lazy_static! {
    pub static ref EVM_ACCOUNTS: IndexPointer = IndexPointer::from_keyword("/prog/accounts/");
    pub static ref EVM_STORAGE: IndexPointer = IndexPointer::from_keyword("/prog/storage/");
    pub static ref CODE_HASH_TO_BYTECODE: IndexPointer = IndexPointer::from_keyword("/prog/code_hash_to_bytecode/");
    pub static ref CONTRACT_ADDRESS_TO_INSCRIPTION_ID: IndexPointer = IndexPointer::from_keyword("/prog/contract_to_id/");
    pub static ref INSCRIPTION_ID_TO_CONTRACT_ADDRESS: IndexPointer = IndexPointer::from_keyword("/prog/id_to_contract/");
}
