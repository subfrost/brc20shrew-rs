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
            let mut account_info: AccountInfo = match bincode::deserialize(&result) {
                Ok(info) => info,
                Err(_) => return Ok(None),
            };
            // Load contract code from storage (matching canonical brc20-prog behavior).
            // AccountInfo deserialized from bincode may not include the code field,
            // so we must look it up by code_hash.
            if account_info.code.is_none() || account_info.code.as_ref().map_or(false, |c| c.is_empty()) {
                account_info.code = Some(
                    self.code_by_hash(account_info.code_hash)
                        .unwrap_or(Bytecode::new())
                );
            }
            Ok(Some(account_info))
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
        // Debug: log commit summary to a well-known key for diagnosis
        {
            let mut summary = String::new();
            let mut total_accounts = 0u32;
            let mut touched_accounts = 0u32;
            let mut total_storage_changes = 0u32;
            for (address, account) in &changes {
                total_accounts += 1;
                if account.is_touched() {
                    touched_accounts += 1;
                    let changed = account.storage.iter().filter(|(_, v)| v.is_changed()).count() as u32;
                    total_storage_changes += changed;
                    if changed > 0 || account.is_created() {
                        summary.push_str(&format!(
                            "{}:t={},c={},s={};",
                            &hex::encode(address.as_slice())[..8],
                            account.is_touched() as u8,
                            account.is_created() as u8,
                            changed,
                        ));
                    }
                }
            }
            let commit_summary = format!(
                "accounts={},touched={},storage_changes={},details={}",
                total_accounts, touched_accounts, total_storage_changes, summary,
            );
            let mut ptr = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_commit");
            ptr.set(Arc::new(commit_summary.into_bytes()));
        }

        for (address, account) in changes {
            // Only process touched accounts (matching canonical brc20-prog behavior)
            if !account.is_touched() {
                continue;
            }

            if account.is_selfdestructed() {
                EVM_ACCOUNTS.select(&address.to_vec()).set(Arc::new(vec![]));
                continue;
            }

            let account_info_bytes = bincode::serialize(&account.info).unwrap();
            EVM_ACCOUNTS.select(&address.to_vec()).set(Arc::new(account_info_bytes));

            // Only store code for newly created accounts
            if account.is_created() {
                if let Some(bytecode) = &account.info.code {
                    if !bytecode.is_empty() {
                        CODE_HASH_TO_BYTECODE.select(&account.info.code_hash.to_vec())
                            .set(Arc::new(bytecode.bytes().to_vec()));
                    }
                }
            }

            // Only store changed storage slots
            for (index, value) in account.storage {
                if !value.is_changed() {
                    continue;
                }
                let mut key = address.to_vec();
                key.extend_from_slice(&index.to_be_bytes::<32>());
                EVM_STORAGE.select(&key).set(
                    Arc::new(value.present_value().to_be_bytes::<32>().to_vec())
                );
            }
        }
    }
}
