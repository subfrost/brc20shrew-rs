use shrew_support::inscription::InscriptionEntry;
use shrew_support::constants::{
    BRC20_PROG_GAS_PER_BYTE, BRC20_PROG_MAX_CALL_GAS, BRC20_PROG_PRAGUE_HARDFORK,
};
use shrew_ord::tables::{
    SEQUENCE_TO_INSCRIPTION_ENTRY, INSCRIPTION_CONTENT, GLOBAL_SEQUENCE_COUNTER,
};
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::{CONTRACT_ADDRESS_TO_INSCRIPTION_ID, INSCRIPTION_ID_TO_CONTRACT_ADDRESS};
use revm::primitives::{Address, U256, TxKind};
use revm::primitives::hardfork::SpecId;
use revm::context::result::{ExecutionResult, Output};
use revm::context::{Context, TxEnv};
use revm::{MainBuilder, ExecuteCommitEvm};
use bitcoin::Block;
use metashrew_support::index_pointer::KeyValuePointer;
use serde::Deserialize;
use std::sync::Arc;

type Ctx = Context<
    revm::context::BlockEnv,
    TxEnv,
    revm::context::CfgEnv,
    MetashrewDB,
    revm::context::Journal<MetashrewDB>,
    (),
>;

/// BRC20-prog chain ID: 0x4252433230 ("BRC20" in ASCII)
const BRC20_PROG_CHAIN_ID: u64 = 0x4252433230;

#[derive(Debug, Deserialize)]
struct ProgOperation {
    p: String,
    op: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DeployOp {
    d: String, // hex bytecode
}

#[derive(Debug, Deserialize)]
struct CallOp {
    i: String, // inscription id of contract
    d: String, // hex calldata
}

#[derive(Debug, Deserialize)]
struct TransactOp {
    to: String,   // hex address
    d: String,    // hex calldata
}

fn get_evm_spec(height: u32) -> SpecId {
    if height >= BRC20_PROG_PRAGUE_HARDFORK {
        SpecId::PRAGUE
    } else {
        SpecId::CANCUN
    }
}

fn make_tx(kind: TxKind, data: revm::primitives::Bytes, gas_limit: u64) -> TxEnv {
    TxEnv {
        kind,
        data,
        gas_limit,
        gas_price: 0,
        value: U256::ZERO,
        chain_id: Some(BRC20_PROG_CHAIN_ID),
        ..Default::default()
    }
}

pub struct ProgrammableBrc20Indexer {
    current_height: u32,
}

impl ProgrammableBrc20Indexer {
    pub fn new() -> Self {
        Self { current_height: 0 }
    }

    fn build_ctx(&self) -> Ctx {
        let spec = get_evm_spec(self.current_height);
        let mut ctx: Ctx = Context::new(MetashrewDB, spec);

        ctx.cfg.chain_id = BRC20_PROG_CHAIN_ID;
        ctx.cfg.limit_contract_code_size = Some(usize::MAX);
        ctx.cfg.disable_nonce_check = true;
        ctx.cfg.disable_eip3607 = true;  // allow non-EOA callers
        ctx.cfg.disable_base_fee = true;
        ctx.cfg.disable_priority_fee_check = true;

        ctx.block.number = U256::from(self.current_height);
        ctx.block.gas_limit = u64::MAX;
        ctx.block.basefee = 0;
        ctx.block.difficulty = U256::ZERO;

        ctx
    }

    pub fn index_block(&mut self, _block: &Block, height: u32) {
        self.current_height = height;

        let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
        if seq_bytes.is_empty() { return; }
        let max_seq = u32::from_le_bytes(seq_bytes[..4].try_into().unwrap_or([0; 4]));

        for seq in 1..=max_seq {
            let entry_bytes = SEQUENCE_TO_INSCRIPTION_ENTRY.select(&seq.to_le_bytes().to_vec()).get();
            if entry_bytes.is_empty() { continue; }
            let entry = match InscriptionEntry::from_bytes(&entry_bytes) {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.height != height { continue; }

            let inscription_id_str = entry.id.to_string();
            let content_bytes = INSCRIPTION_CONTENT.select(&inscription_id_str.as_bytes().to_vec()).get();
            if content_bytes.is_empty() { continue; }

            if let Ok(op) = serde_json::from_slice::<ProgOperation>(&content_bytes) {
                if op.p == "brc20-prog" {
                    match op.op.as_str() {
                        "deploy" => {
                            if let Ok(deploy) = serde_json::from_value::<DeployOp>(op.data) {
                                self.execute_deploy(&entry, deploy);
                            }
                        }
                        "call" => {
                            if let Ok(call) = serde_json::from_value::<CallOp>(op.data) {
                                self.execute_call(call);
                            }
                        }
                        "transact" => {
                            if let Ok(transact) = serde_json::from_value::<TransactOp>(op.data) {
                                self.execute_transact(transact);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn execute_deploy(&mut self, entry: &InscriptionEntry, op: DeployOp) {
        let data: revm::primitives::Bytes = hex::decode(op.d).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);

        let mut evm = self.build_ctx().build_mainnet();
        let tx = make_tx(TxKind::Create, data, gas_limit);

        let result = evm.transact_commit(tx);
        if let Ok(exec_result) = result {
            if let ExecutionResult::Success { output, .. } = exec_result {
                if let Output::Create(_, Some(address)) = output {
                    let inscription_id_bytes = entry.id.to_bytes();
                    CONTRACT_ADDRESS_TO_INSCRIPTION_ID.select(&address.to_vec())
                        .set(Arc::new(inscription_id_bytes.clone()));
                    INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id_bytes)
                        .set(Arc::new(address.to_vec()));
                }
            }
        }
    }

    fn execute_call(&mut self, op: CallOp) {
        let inscription_id_bytes = op.i.as_bytes();
        let pointer = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id_bytes.to_vec());
        let result = pointer.get();
        if result.is_empty() { return; }

        let address = Address::from_slice(&result);
        let data: revm::primitives::Bytes = hex::decode(op.d).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);

        let mut evm = self.build_ctx().build_mainnet();
        let tx = make_tx(TxKind::Call(address), data, gas_limit);

        let _ = evm.transact_commit(tx);
    }

    fn execute_transact(&mut self, op: TransactOp) {
        let to_bytes = hex::decode(&op.to).unwrap_or_default();
        if to_bytes.len() != 20 { return; }

        let address = Address::from_slice(&to_bytes);
        let data: revm::primitives::Bytes = hex::decode(op.d).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);

        let mut evm = self.build_ctx().build_mainnet();
        let tx = make_tx(TxKind::Call(address), data, gas_limit);

        let _ = evm.transact_commit(tx);
    }
}
