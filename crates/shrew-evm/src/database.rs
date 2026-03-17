use crate::tables::*;
use shrew_ord::tables::HEIGHT_TO_BLOCK_HASH;
use revm::primitives::{Address, B256, U256};
use revm::state::{Account, AccountInfo, Bytecode};
use revm::{Database, DatabaseCommit};
use revm::database_interface::DBErrorMarker;
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::Arc;
use std::fmt;
use std::error::Error as StdError;

#[derive(Debug)]
pub enum MetashrewError {
    DBError,
}

impl fmt::Display for MetashrewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Metashrew DB Error")
    }
}

impl StdError for MetashrewError {}
impl DBErrorMarker for MetashrewError {}

#[derive(Default, Debug)]
pub struct MetashrewDB;

impl Database for MetashrewDB {
    type Error = MetashrewError;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let pointer = EVM_ACCOUNTS.select(&address.to_vec());
        let result = pointer.get();
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(bincode::deserialize(&result).ok())
        }
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let pointer = CODE_HASH_TO_BYTECODE.select(&code_hash.to_vec());
        let result = pointer.get();
        if result.is_empty() {
            Ok(Bytecode::new())
        } else {
            Ok(Bytecode::new_raw((*result).clone().into()))
        }
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let mut key = address.to_vec();
        key.extend_from_slice(&index.to_be_bytes::<32>());
        let pointer = EVM_STORAGE.select(&key);
        let result = pointer.get();
        if result.is_empty() {
            Ok(U256::ZERO)
        } else {
            Ok(U256::from_be_slice(&result))
        }
    }

    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        let height_bytes = (number as u32).to_le_bytes().to_vec();
        let pointer = HEIGHT_TO_BLOCK_HASH.select(&height_bytes);
        let result = pointer.get();
        if result.is_empty() {
            Ok(B256::ZERO)
        } else {
            Ok(B256::from_slice(&result))
        }
    }
}

impl DatabaseCommit for MetashrewDB {
    fn commit(&mut self, changes: revm::primitives::map::HashMap<Address, Account>) {
        for (address, account) in changes {
            if account.is_selfdestructed() {
                EVM_ACCOUNTS.select(&address.to_vec()).set(Arc::new(vec![]));
            } else {
                let account_info_bytes = bincode::serialize(&account.info).unwrap();
                EVM_ACCOUNTS.select(&address.to_vec()).set(Arc::new(account_info_bytes));

                if let Some(bytecode) = &account.info.code {
                    if !bytecode.is_empty() {
                        CODE_HASH_TO_BYTECODE.select(&account.info.code_hash.to_vec())
                            .set(Arc::new(bytecode.bytes().to_vec()));
                    }
                }

                for (index, value) in account.storage {
                    let mut key = address.to_vec();
                    key.extend_from_slice(&index.to_be_bytes::<32>());
                    EVM_STORAGE.select(&key).set(
                        Arc::new(value.present_value().to_be_bytes::<32>().to_vec())
                    );
                }
            }
        }
    }
}
