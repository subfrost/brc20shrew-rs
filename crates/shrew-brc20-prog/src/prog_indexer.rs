use shrew_support::inscription::InscriptionEntry;
use shrew_support::constants::{
    BRC20_PROG_GAS_PER_BYTE, BRC20_PROG_MAX_CALL_GAS, BRC20_PROG_PRAGUE_HARDFORK,
};
use shrew_ord::tables::{
    SEQUENCE_TO_INSCRIPTION_ENTRY, INSCRIPTION_CONTENT, GLOBAL_SEQUENCE_COUNTER,
};
use shrew_evm::database::MetashrewDB;
use shrew_evm::tables::{
    CONTRACT_ADDRESS_TO_INSCRIPTION_ID, INSCRIPTION_ID_TO_CONTRACT_ADDRESS,
    EVM_ACCOUNTS, CODE_HASH_TO_BYTECODE,
};
use shrew_brc20::tables::Brc20ProgDeposits;
use shrew_evm::ShrewPrecompiles;
use crate::controller::{CONTROLLER_ADDRESS, controller_bytecode};
use revm::primitives::{Address, Bytes, U256, B256, TxKind};
use revm::primitives::hardfork::SpecId;
use revm::state::{AccountInfo, Bytecode};
use revm::context::result::{ExecutionResult, Output};
use revm::context::{Context, TxEnv, BlockEnv, CfgEnv, Journal, FrameStack, Evm};
use revm::handler::instructions::EthInstructions;
use revm::handler::EthFrame;
use revm::interpreter::interpreter::EthInterpreter;
use revm::ExecuteCommitEvm;
use bitcoin::Block;
use metashrew_support::index_pointer::KeyValuePointer;
use serde::Deserialize;
use std::sync::Arc;

type Ctx = Context<BlockEnv, TxEnv, CfgEnv, MetashrewDB, Journal<MetashrewDB>, ()>;
type ShrewEvm = Evm<Ctx, (), EthInstructions<EthInterpreter, Ctx>, ShrewPrecompiles, EthFrame<EthInterpreter>>;

/// BRC20-prog chain ID: 0x4252433230 ("BRC20" in ASCII)
const BRC20_PROG_CHAIN_ID: u64 = 0x4252433230;

/// Derive an EVM address from a Bitcoin script pubkey.
/// Matches the canonical brc20-prog: keccak256(pkscript)[12:]
fn pkscript_to_evm_address(script_pubkey: &[u8]) -> Address {
    use revm::primitives::keccak256;
    let hash = keccak256(script_pubkey);
    Address::from_slice(&hash[12..32])
}

/// Derive the sender EVM address from an inscription entry and its block.
/// Looks up the inscription's output script_pubkey and computes keccak256(pkscript)[12:].
/// Falls back to keccak256(inscription_id)[12:] if the output can't be found.
fn derive_sender_address(entry: &InscriptionEntry, block: &Block) -> Address {
    // Find the transaction containing this inscription
    let txid = entry.id.txid;
    for tx in &block.txdata {
        if tx.compute_txid() == txid {
            // The inscription output is at the satpoint vout
            let vout = entry.satpoint.outpoint.vout as usize;
            if vout < tx.output.len() {
                let pkscript = tx.output[vout].script_pubkey.as_bytes();
                return pkscript_to_evm_address(pkscript);
            }
        }
    }
    // Fallback: use inscription ID hash
    use revm::primitives::keccak256;
    let mut input = Vec::new();
    input.extend_from_slice(&entry.id.txid[..]);
    input.extend_from_slice(&entry.id.index.to_le_bytes());
    let hash = keccak256(&input);
    Address::from_slice(&hash[12..32])
}

/// Table key for tracking whether controller has been deployed
const CONTROLLER_DEPLOYED_KEY: &str = "/prog/controller_deployed";

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
    #[serde(default)]
    i: Option<String>, // inscription id of contract (legacy format)
    #[serde(default)]
    c: Option<String>, // contract address (0x-prefixed hex, new format)
    d: String,         // hex calldata
}

#[derive(Debug, Deserialize)]
struct TransactOp {
    #[serde(default)]
    to: Option<String>, // hex address
    #[serde(default)]
    c: Option<String>,  // contract address (alternative to 'to')
    d: String,          // hex calldata
}

fn get_evm_spec(height: u32) -> SpecId {
    if height >= BRC20_PROG_PRAGUE_HARDFORK {
        SpecId::PRAGUE
    } else {
        SpecId::CANCUN
    }
}

fn make_tx(kind: TxKind, data: Bytes, gas_limit: u64, caller: Address) -> TxEnv {
    TxEnv {
        caller,
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
    controller_deployed: bool,
}

impl ProgrammableBrc20Indexer {
    pub fn new() -> Self {
        Self { current_height: 0, controller_deployed: false }
    }

    /// Build an EVM instance with custom BRC20-prog precompiles.
    fn build_evm(&self, op_return_tx_id: B256) -> ShrewEvm {
        let spec = get_evm_spec(self.current_height);
        let mut ctx: Ctx = Context::new(MetashrewDB, spec);

        ctx.cfg.chain_id = BRC20_PROG_CHAIN_ID;
        ctx.cfg.limit_contract_code_size = Some(usize::MAX);
        ctx.cfg.disable_nonce_check = true;
        ctx.cfg.disable_eip3607 = true;
        ctx.cfg.disable_base_fee = true;
        ctx.cfg.disable_priority_fee_check = true;

        ctx.block.number = U256::from(self.current_height);
        ctx.block.gas_limit = u64::MAX;
        ctx.block.basefee = 0;
        ctx.block.difficulty = U256::ZERO;

        let precompiles = ShrewPrecompiles::new(
            spec.into(),
            op_return_tx_id,
            self.current_height,
        );

        Evm {
            ctx,
            inspector: (),
            instruction: EthInstructions::new_mainnet_with_spec(spec.into()),
            precompiles,
            frame_stack: FrameStack::new_prealloc(8),
        }
    }

    /// Deploy the BRC20 controller contract at its fixed address.
    /// Called once on first block, sets account code directly in the database.
    fn ensure_controller_deployed(&mut self) {
        if self.controller_deployed { return; }

        // Check persistent state
        let mut deployed_marker = metashrew_core::index_pointer::IndexPointer::from_keyword(CONTROLLER_DEPLOYED_KEY);
        if !deployed_marker.get().is_empty() {
            self.controller_deployed = true;
            return;
        }

        // Deploy the controller contract bytecode at the fixed address
        let bytecode = controller_bytecode();
        let code_hash = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&bytecode);
            B256::from_slice(&hasher.finalize())
        };

        // Store bytecode
        CODE_HASH_TO_BYTECODE.select(&code_hash.to_vec())
            .set(Arc::new(bytecode.clone()));

        // Store account info at controller address
        let account_info = AccountInfo {
            balance: U256::ZERO,
            nonce: 1,
            code_hash,
            account_id: None,
            code: Some(Bytecode::new_raw(Bytes::from(bytecode))),
        };
        let account_bytes = bincode::serialize(&account_info).expect("serialize account info");
        EVM_ACCOUNTS.select(&CONTROLLER_ADDRESS.to_vec())
            .set(Arc::new(account_bytes));

        // Mark as deployed
        deployed_marker.set(Arc::new(vec![1]));
        self.controller_deployed = true;
    }

    pub fn index_block(&mut self, block: &Block, height: u32) {
        self.current_height = height;

        // Ensure controller contract is deployed
        self.ensure_controller_deployed();

        let seq_bytes = GLOBAL_SEQUENCE_COUNTER.get();
        if seq_bytes.is_empty() {
            // Still process pending deposits even if no inscriptions
            self.process_pending_deposits(height);
            return;
        }
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
                        "deploy" | "d" => {
                            if let Ok(deploy) = serde_json::from_value::<DeployOp>(op.data) {
                                self.execute_deploy(&entry, deploy, block);
                            }
                        }
                        "call" | "c" => {
                            if let Ok(call) = serde_json::from_value::<CallOp>(op.data) {
                                self.execute_call(&entry, call, block);
                            }
                        }
                        "transact" | "t" => {
                            if let Ok(transact) = serde_json::from_value::<TransactOp>(op.data) {
                                self.execute_transact(&entry, transact, block);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Process any BRC20 deposit events from the BRC-20 indexer
        self.process_pending_deposits(height);
    }

    /// Process pending BRC20-PROG deposit events from the BRC-20 indexer.
    /// For each deposit, call controller_mint to create EVM-side token representation.
    fn process_pending_deposits(&mut self, height: u32) {
        let deposits_table = Brc20ProgDeposits::new();
        let events = deposits_table.get(height);
        if events.is_empty() { return; }

        for event in &events {
            // The sender address is a Bitcoin address string.
            // For EVM representation, we hash it to get a deterministic 20-byte address.
            let recipient = {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(event.sender.as_bytes());
                let hash = hasher.finalize();
                Address::from_slice(&hash[..20])
            };

            self.controller_mint(&event.ticker, recipient, U256::from(event.amount));
        }

        // Clear processed deposits
        deposits_table.clear(height);
    }

    fn execute_deploy(&mut self, entry: &InscriptionEntry, op: DeployOp, block: &Block) {
        let hex_str = op.d.strip_prefix("0x").unwrap_or(&op.d);
        let data: Bytes = hex::decode(hex_str).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);
        let sender = derive_sender_address(entry, block);

        let mut evm = self.build_evm(B256::ZERO);
        let tx = make_tx(TxKind::Create, data, gas_limit, sender);

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

    fn execute_call(&mut self, entry: &InscriptionEntry, op: CallOp, block: &Block) {
        // Resolve contract address — either from "c" (hex address) or "i" (inscription ID)
        let address = if let Some(ref addr_hex) = op.c {
            let hex = addr_hex.strip_prefix("0x").unwrap_or(addr_hex);
            let bytes = hex::decode(hex).unwrap_or_default();
            if bytes.len() != 20 { return; }
            Address::from_slice(&bytes)
        } else if let Some(ref inscription_id) = op.i {
            let pointer = INSCRIPTION_ID_TO_CONTRACT_ADDRESS.select(&inscription_id.as_bytes().to_vec());
            let result = pointer.get();
            if result.is_empty() { return; }
            Address::from_slice(&result)
        } else {
            return; // No address source
        };

        let data_hex = op.d.strip_prefix("0x").unwrap_or(&op.d);
        let data: Bytes = hex::decode(data_hex).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);
        let sender = derive_sender_address(entry, block);

        let mut evm = self.build_evm(B256::ZERO);
        let tx = make_tx(TxKind::Call(address), data, gas_limit, sender);

        let _ = evm.transact_commit(tx);
    }

    fn execute_transact(&mut self, entry: &InscriptionEntry, op: TransactOp, block: &Block) {
        // Resolve address from "to" or "c"
        let addr_hex = op.to.as_deref().or(op.c.as_deref()).unwrap_or("");
        let hex = addr_hex.strip_prefix("0x").unwrap_or(addr_hex);
        let to_bytes = hex::decode(hex).unwrap_or_default();
        if to_bytes.len() != 20 { return; }

        let address = Address::from_slice(&to_bytes);
        let data_hex = op.d.strip_prefix("0x").unwrap_or(&op.d);
        let data: Bytes = hex::decode(data_hex).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);
        let sender = derive_sender_address(entry, block);

        let mut evm = self.build_evm(B256::ZERO);
        let tx = make_tx(TxKind::Call(address), data, gas_limit, sender);

        let _ = evm.transact_commit(tx);
    }

    /// Call the controller's mint function to create EVM-side token representation.
    /// Called when BRC20 tokens are deposited via BRC20-PROG OP_RETURN.
    pub fn controller_mint(&mut self, ticker: &str, recipient: Address, amount: U256) {
        use crate::controller::selectors;

        // ABI-encode: mint(bytes ticker, address recipient, uint256 amount)
        let ticker_bytes = ticker.as_bytes();
        let mut calldata = Vec::new();
        calldata.extend_from_slice(&selectors::MINT);
        // offset to ticker bytes (0x60 = 96)
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);
        // recipient address (padded to 32 bytes)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(recipient.as_slice());
        // amount
        calldata.extend_from_slice(&amount.to_be_bytes::<32>());
        // ticker length
        let mut len_bytes = [0u8; 32];
        len_bytes[31] = ticker_bytes.len() as u8;
        calldata.extend_from_slice(&len_bytes);
        // ticker data (padded)
        let padded_len = (ticker_bytes.len() + 31) / 32 * 32;
        let mut padded = vec![0u8; padded_len];
        padded[..ticker_bytes.len()].copy_from_slice(ticker_bytes);
        calldata.extend_from_slice(&padded);

        let data: Bytes = calldata.into();
        let gas_limit = BRC20_PROG_MAX_CALL_GAS;

        let mut evm = self.build_evm(B256::ZERO);
        let tx = make_tx(TxKind::Call(CONTROLLER_ADDRESS), data, gas_limit, CONTROLLER_ADDRESS);
        let _ = evm.transact_commit(tx);
    }

    /// Call the controller's burn function to destroy EVM-side token representation.
    pub fn controller_burn(&mut self, ticker: &str, sender: Address, amount: U256) {
        use crate::controller::selectors;

        let ticker_bytes = ticker.as_bytes();
        let mut calldata = Vec::new();
        calldata.extend_from_slice(&selectors::BURN);
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(sender.as_slice());
        calldata.extend_from_slice(&amount.to_be_bytes::<32>());
        let mut len_bytes = [0u8; 32];
        len_bytes[31] = ticker_bytes.len() as u8;
        calldata.extend_from_slice(&len_bytes);
        let padded_len = (ticker_bytes.len() + 31) / 32 * 32;
        let mut padded = vec![0u8; padded_len];
        padded[..ticker_bytes.len()].copy_from_slice(ticker_bytes);
        calldata.extend_from_slice(&padded);

        let data: Bytes = calldata.into();
        let gas_limit = BRC20_PROG_MAX_CALL_GAS;

        let mut evm = self.build_evm(B256::ZERO);
        let tx = make_tx(TxKind::Call(CONTROLLER_ADDRESS), data, gas_limit, CONTROLLER_ADDRESS);
        let _ = evm.transact_commit(tx);
    }
}
