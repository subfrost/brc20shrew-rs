//! BRC-20 Programmable Module
//!
//! This module implements the logic for handling BRC-20 smart contracts
//! within the `metashrew` environment. It includes the `ProgrammableBrc20Indexer`,
//! which wraps the standard BRC-20 indexer and adds EVM execution capabilities.

use crate::indexer::InscriptionIndexer;
use crate::envelope::Inscription;
use crate::inscription::InscriptionEntry;
use crate::tables::{EVM_ACCOUNTS, EVM_STORAGE, CODE_HASH_TO_BYTECODE, HEIGHT_TO_BLOCK_HASH, CONTRACT_ADDRESS_TO_INSCRIPTION_ID, INSCRIPTION_ID_TO_CONTRACT_ADDRESS};
use revm::primitives::{Account, AccountInfo, Bytecode, B256, U256, TransactTo, ExecutionResult, Output, Address, HashMap as RevmHashMap, CreateScheme};
use revm::{Database, DatabaseCommit, EVM};
use std::sync::Arc;
use metashrew_support::index_pointer::KeyValuePointer;
use serde::Deserialize;
use std::fmt;
use std::error::Error as StdError;

#[derive(Debug, Deserialize)]
struct ProgrammableBrc20Operation {
    p: String,
    op: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DeployOperation {
    d: String, // bytecode
}

#[derive(Debug, Deserialize)]
struct CallOperation {
    i: String, // inscription id
    d: String, // calldata
}

/// A custom database for `revm` that interacts with the `metashrew` key-value store.
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

use revm_database_interface::DBErrorMarker;

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

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        let height_bytes = (number.as_limbs()[0] as u32).to_le_bytes().to_vec();
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
   fn commit(&mut self, changes: RevmHashMap<Address, Account>) {
       for (address, account) in changes {
           if account.is_selfdestructed() {
               // Clear storage
               let mut pointer = EVM_ACCOUNTS.select(&address.to_vec());
               pointer.set(Arc::new(vec![]));
               // In a real implementation, you would need a way to clear all storage slots
               // associated with this address. This is a simplification.
           } else {
               // Store account info
               let account_info_bytes = bincode::serialize(&account.info).unwrap();
               EVM_ACCOUNTS.select(&address.to_vec()).set(Arc::new(account_info_bytes));

               // Store bytecode if it exists
               if let Some(bytecode) = &account.info.code {
                   if !bytecode.is_empty() {
                       CODE_HASH_TO_BYTECODE.select(&account.info.code_hash.to_vec()).set(Arc::new(bytecode.bytes().to_vec()));
                   }
               }

               // Store storage changes
               for (index, value) in account.storage {
                   let mut key = address.to_vec();
                   key.extend_from_slice(&index.to_be_bytes::<32>());
                   EVM_STORAGE.select(&key).set(Arc::new(value.present_value().to_be_bytes::<32>().to_vec()));
               }
           }
       }
   }
}

/// The main indexer for the BRC-20 programmable module.
pub struct ProgrammableBrc20Indexer {
   /// The underlying BRC-20 and inscription indexer.
   pub indexer: InscriptionIndexer,
   /// The EVM instance for executing smart contracts.
   pub evm: EVM<MetashrewDB>,
}

impl ProgrammableBrc20Indexer {
   /// Creates a new `ProgrammableBrc20Indexer`.
   pub fn new() -> Self {
        let mut evm = EVM::<MetashrewDB>::new();
        evm.env.tx.gas_limit = u64::MAX;
        Self {
            indexer: InscriptionIndexer::new(),
            evm,
        }
   }

   /// Indexes a single inscription, checking for programmable BRC-20 operations.
   pub fn index_programmable_inscription(&mut self, entry: &InscriptionEntry, inscription: &Inscription) {
       if let Some(content) = &inscription.body {
           if let Ok(op) = serde_json::from_slice::<ProgrammableBrc20Operation>(&content) {
               if op.p == "brc20-prog" {
                   match op.op.as_str() {
                       "deploy" => {
                           if let Ok(deploy_op) = serde_json::from_value::<DeployOperation>(op.data) {
                               self.execute_deploy(entry, deploy_op);
                           }
                       },
                       "call" => {
                           if let Ok(call_op) = serde_json::from_value::<CallOperation>(op.data) {
                               self.execute_call(call_op);
                           }
                       },
                       _ => {}
                   }
               }
           }
       }
   }

   fn execute_deploy(&mut self, entry: &InscriptionEntry, op: DeployOperation) {
        self.evm.env.tx.transact_to = TransactTo::Create(CreateScheme::Create);
        self.evm.env.tx.data = hex::decode(op.d).unwrap_or_default().into();
       
        let result = self.evm.transact_commit();

        if let Ok(exec_result) = result {
            if let ExecutionResult::Success { output, .. } = exec_result {
                 if let Output::Create(_, Some(address)) = output {
                    // Store contract address -> inscription id mapping
                    let inscription_id_bytes = entry.id.to_bytes();
                    CONTRACT_ADDRESS_TO_INSCRIPTION_ID.select(&address.to_vec()).set(Arc::new(inscription_id_bytes.clone()));
                    INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id_bytes).set(Arc::new(address.to_vec()));
                 }
            }
        }
   }

   fn execute_call(&mut self, op: CallOperation) {
       let inscription_id_bytes = op.i.as_bytes();
       let pointer = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id_bytes.to_vec());
       let result = pointer.get();
       if !result.is_empty() {
           let address = Address::from_slice(&result);
           self.evm.env.tx.transact_to = TransactTo::Call(address);
           self.evm.env.tx.data = hex::decode(op.d).unwrap_or_default().into();
           let _ = self.evm.transact_commit();
       }
   }
}
