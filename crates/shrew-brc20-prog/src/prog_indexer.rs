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
///
/// Strategy: Use the inscription transaction's first input's previous output
/// to trace back to the funding address. In the commit-reveal pattern:
///   - Reveal tx input[0] spends the commit output
///   - The commit output was funded by the wallet
///   - Find the commit tx in the same block or prior blocks
///   - Use the commit tx's SECOND output (change output back to wallet) pkscript
///
/// For simplicity and cross-block compatibility, we use the inscription
/// transaction's first input's previous_output txid to find the commit tx.
/// If the commit tx is in the SAME block (atomic broadcast), we can resolve it.
/// Otherwise, fall back to the inscription output pkscript.
fn derive_sender_address(entry: &InscriptionEntry, block: &Block) -> Address {
    let txid = entry.id.txid;

    // Find the inscription (reveal) transaction
    for tx in &block.txdata {
        if tx.compute_txid() == txid {
            if !tx.input.is_empty() {
                let prev_txid = tx.input[0].previous_output.txid;

                // Try to find the commit tx in the same block
                for prev_tx in &block.txdata {
                    if prev_tx.compute_txid() == prev_txid {
                        // The commit tx's change output (last non-OP_RETURN output)
                        // goes back to the wallet. Use its pkscript.
                        for output in prev_tx.output.iter().rev() {
                            if !output.script_pubkey.is_op_return() && output.value.to_sat() > 546 {
                                return pkscript_to_evm_address(output.script_pubkey.as_bytes());
                            }
                        }
                    }
                }
            }

            // Fallback: use inscription output pkscript
            let vout = entry.satpoint.outpoint.vout as usize;
            if vout < tx.output.len() {
                return pkscript_to_evm_address(tx.output[vout].script_pubkey.as_bytes());
            }
        }
    }

    // Last resort: derive from inscription ID
    use revm::primitives::keccak256;
    let mut input = Vec::new();
    input.extend_from_slice(&entry.id.txid[..]);
    input.extend_from_slice(&entry.id.index.to_le_bytes());
    let hash = keccak256(&input);
    Address::from_slice(&hash[12..32])
}

/// Table key for tracking whether controller has been deployed
const CONTROLLER_DEPLOYED_KEY: &str = "/prog/controller_deployed";

/// Debug: last processed inscription content (for diagnosing devnet issues)
const DEBUG_LAST_INSCRIPTION_KEY: &str = "/debug/last_inscription";
/// Debug: last processed inscription result
const DEBUG_LAST_RESULT_KEY: &str = "/debug/last_result";

#[derive(Debug, Deserialize)]
struct ProgOperation {
    p: String,
    op: String,
    #[serde(flatten)]
    data: serde_json::Value,
}

/// Serializable representation of a deferred inscription awaiting activation.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DeferredInscription {
    /// Raw inscription content bytes (the brc20-prog JSON)
    content: Vec<u8>,
    /// Inscription entry bytes (serialized)
    entry_bytes: Vec<u8>,
    /// The inscription's reveal txid (for looking up the activation mapping)
    reveal_txid: Vec<u8>,
    /// Height the inscription was first seen
    height: u32,
    /// Pre-computed sender EVM address (20 bytes)
    /// Stored because the sender derivation needs the reveal block, which
    /// won't be available when the deferred inscription is re-executed
    /// in the activation block context.
    sender: Vec<u8>,
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
    // The canonical brc20-prog uses PRAGUE for regtest/unknown networks.
    // On mainnet, PRAGUE activates at block 923,369.
    // Since brc20shrew doesn't know the network, we use PRAGUE by default
    // when below the mainnet activation height — this matches regtest behavior.
    // On mainnet, blocks below 923,369 would need CANCUN, but those blocks
    // were already indexed before brc20-prog existed.
    //
    // For production mainnet indexing, this should check the Bitcoin network.
    // For now, always use PRAGUE (matches devnet/regtest and mainnet post-activation).
    let _ = height; // TODO: network-aware spec selection
    SpecId::PRAGUE
}

fn make_tx(kind: TxKind, data: Bytes, _gas_limit: u64, caller: Address) -> TxEnv {
    // Match canonical brc20-prog: use u64::MAX for tx gas limit.
    TxEnv {
        caller,
        kind,
        data,
        gas_limit: u64::MAX,
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
        ctx.cfg.spec = spec;
        ctx.cfg.limit_contract_code_size = Some(usize::MAX);
        ctx.cfg.disable_nonce_check = true;
        ctx.cfg.disable_eip3607 = true;
        ctx.cfg.disable_balance_check = true;
        ctx.cfg.disable_base_fee = true;
        ctx.cfg.disable_fee_charge = true;
        ctx.cfg.disable_block_gas_limit = true;
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

        // Scan for BRC20PROG activation transactions FIRST.
        // The activation tx may appear in the same block as the inscription
        // or in a later block. We store the mapping so inscription processing
        // can resolve the correct op_return_tx_id. Also triggers deferred
        // execution of inscriptions that were waiting for their activation tx.
        self.scan_for_activation_txs(block, height);

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
                    // Check for activation tx (3-tx pattern).
                    // If found, use the activation txid for getTxId() precompile.
                    // If not found, execute anyway — calls that don't use getTxId()
                    // (like initialize(), setSigner, etc.) work fine without it.
                    // Calls that DO use getTxId() will get the reveal txid or B256::ZERO,
                    // and the contract will revert if it can't find the tx details.
                    //
                    // We still store deferred entries so that when the activation tx
                    // arrives, we can RE-execute the call with the correct context.
                    // But we don't skip execution — we execute optimistically now.
                    let is_call_or_transact = matches!(op.op.as_str(), "call" | "c" | "transact" | "t");
                    let has_activation = Self::has_activation_mapping(&entry);

                    if is_call_or_transact && !has_activation {
                        // Check if the reveal tx has OP_RETURN (2-tx pattern)
                        let reveal_has_op_return = block.txdata.iter()
                            .find(|tx| tx.compute_txid() == entry.id.txid)
                            .map(|tx| {
                                tx.output.iter().any(|o| {
                                    let s = o.script_pubkey.as_bytes();
                                    s.len() >= 11 && s[0] == 0x6a && s[1] == 0x09 && &s[2..11] == b"BRC20PROG"
                                })
                            })
                            .unwrap_or(false);

                        if !reveal_has_op_return {
                            // Store deferred entry for re-execution when activation arrives
                            let sender_addr = derive_sender_address(&entry, block);
                            let deferred = DeferredInscription {
                                content: content_bytes.to_vec(),
                                entry_bytes: entry_bytes.to_vec(),
                                reveal_txid: entry.id.txid[..].to_vec(),
                                height,
                                sender: sender_addr.to_vec(),
                            };
                            if let Ok(data) = serde_json::to_vec(&deferred) {
                                let mut ptr = metashrew_core::index_pointer::IndexPointer::from_keyword(
                                    Self::DEFERRED_INSCRIPTIONS_PREFIX,
                                ).select(&entry.id.txid[..].to_vec());
                                ptr.set(Arc::new(data));
                            }
                            // DON'T skip — execute optimistically. Pure EVM calls
                            // (initialize, setSigner, etc.) work without activation.
                            // Calls needing getTxId() will revert, and the deferred
                            // entry allows re-execution when activation arrives.
                        }
                    }

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

    /// Convert a Bitcoin Txid to B256 in display (big-endian) byte order.
    /// The getTxDetails precompile receives bytes32 from Solidity and reverses
    /// to get Bitcoin's internal LE order for the lookup, so we must store BE.
    fn txid_to_b256_be(txid: &bitcoin::Txid) -> B256 {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&txid[..]);
        bytes.reverse(); // LE (internal) → BE (display/EVM)
        B256::from(bytes)
    }

    /// Resolve the op_return_tx_id for a brc20-prog inscription.
    ///
    /// Table key prefix for storing activation tx mappings.
    /// Maps reveal_txid → activation_txid (both in internal LE byte order).
    const ACTIVATION_MAP_PREFIX: &'static str = "/prog/activation_map/";

    /// Table prefix for deferred inscription entries awaiting activation.
    /// Maps reveal_txid → serialized inscription content bytes.
    const DEFERRED_INSCRIPTIONS_PREFIX: &'static str = "/prog/deferred/";

    /// Scan a block for BRC20PROG activation transactions and store mappings.
    /// Also triggers deferred execution of inscriptions that were waiting for activation.
    fn scan_for_activation_txs(&mut self, block: &Block, height: u32) {
        for tx in &block.txdata {
            if tx.output.is_empty() || tx.input.is_empty() { continue; }

            let script = tx.output[0].script_pubkey.as_bytes();
            let is_brc20prog_op_return = if script.len() >= 11 {
                script[0] == 0x6a && script[1] == 0x09 && &script[2..11] == b"BRC20PROG"
            } else {
                // Fallback: check if script contains "BRC20PROG" anywhere
                script.windows(9).any(|w| w == b"BRC20PROG")
            };
            if is_brc20prog_op_return {
                // This is an activation tx. input[0] spends the reveal tx.
                let reveal_txid_bytes = tx.input[0].previous_output.txid[..].to_vec();
                let activation_txid_bytes = tx.compute_txid()[..].to_vec();

                // Store activation mapping
                let mut pointer = metashrew_core::index_pointer::IndexPointer::from_keyword(
                    Self::ACTIVATION_MAP_PREFIX,
                ).select(&reveal_txid_bytes);
                pointer.set(Arc::new(activation_txid_bytes));

                // Check if there's a deferred inscription waiting for this activation
                let mut deferred_ptr = metashrew_core::index_pointer::IndexPointer::from_keyword(
                    Self::DEFERRED_INSCRIPTIONS_PREFIX,
                ).select(&reveal_txid_bytes);
                let deferred_data = deferred_ptr.get();
                if !deferred_data.is_empty() {
                    // Re-execute the deferred inscription with the activation context
                    if let Ok(deferred) = serde_json::from_slice::<DeferredInscription>(&deferred_data) {
                        self.execute_deferred(&deferred, block);
                    }
                    // Clear the deferred entry
                    deferred_ptr.set(Arc::new(vec![]));
                }
            }
        }
    }

    /// Resolve the op_return_tx_id for a brc20-prog inscription.
    ///
    /// Looks up the activation map to find if this inscription has an
    /// associated activation tx (3-tx pattern). If found, returns the
    /// activation tx's id. Otherwise returns the reveal tx's id (2-tx pattern).
    fn resolve_op_return_tx_id(entry: &InscriptionEntry, block: &Block) -> B256 {
        let reveal_txid = entry.id.txid;
        let reveal_txid_bytes = reveal_txid[..].to_vec();

        // First check the persistent activation map (handles cross-block activation)
        let pointer = metashrew_core::index_pointer::IndexPointer::from_keyword(
            Self::ACTIVATION_MAP_PREFIX,
        ).select(&reveal_txid_bytes);
        let stored = pointer.get();
        if !stored.is_empty() && stored.len() == 32 {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&stored);
            bytes.reverse(); // LE → BE for EVM
            return B256::from(bytes);
        }

        // Also scan current block (handles same-block activation)
        for tx in &block.txdata {
            if tx.output.is_empty() || tx.input.is_empty() { continue; }
            if tx.input[0].previous_output.txid != reveal_txid { continue; }

            let script = tx.output[0].script_pubkey.as_bytes();
            let is_brc20prog = if script.len() >= 11 {
                script[0] == 0x6a && script[1] == 0x09 && &script[2..11] == b"BRC20PROG"
            } else {
                script.windows(9).any(|w| w == b"BRC20PROG")
            };
            if is_brc20prog {
                return Self::txid_to_b256_be(&tx.compute_txid());
            }
        }

        // No activation tx found — use reveal tx id (2-tx pattern)
        Self::txid_to_b256_be(&reveal_txid)
    }

    /// Check if an activation mapping exists for an inscription's reveal tx.
    fn has_activation_mapping(entry: &InscriptionEntry) -> bool {
        let reveal_txid_bytes = entry.id.txid[..].to_vec();
        let pointer = metashrew_core::index_pointer::IndexPointer::from_keyword(
            Self::ACTIVATION_MAP_PREFIX,
        ).select(&reveal_txid_bytes);
        !pointer.get().is_empty()
    }

    /// Execute a deferred inscription that was waiting for its activation tx.
    /// Uses the pre-stored sender address since the reveal block isn't available.
    fn execute_deferred(&mut self, deferred: &DeferredInscription, _block: &Block) {
        let entry = match InscriptionEntry::from_bytes(&deferred.entry_bytes) {
            Ok(e) => e,
            Err(_) => return,
        };

        let sender = if deferred.sender.len() == 20 {
            Address::from_slice(&deferred.sender)
        } else {
            Address::ZERO
        };

        // The op_return_tx_id is now available from the activation mapping
        let op_return_tx_id = Self::resolve_op_return_tx_id_from_entry(&entry);

        if let Ok(op) = serde_json::from_slice::<ProgOperation>(&deferred.content) {
            if op.p == "brc20-prog" {
                match op.op.as_str() {
                    "call" | "c" => {
                        if let Ok(call) = serde_json::from_value::<CallOp>(op.data) {
                            self.execute_call_with_sender(&entry, call, sender, op_return_tx_id);
                        }
                    }
                    "transact" | "t" => {
                        if let Ok(transact) = serde_json::from_value::<TransactOp>(op.data) {
                            self.execute_transact_with_sender(&entry, transact, sender, op_return_tx_id);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Resolve op_return_tx_id using only the stored activation mapping (no block scan).
    fn resolve_op_return_tx_id_from_entry(entry: &InscriptionEntry) -> B256 {
        let reveal_txid_bytes = entry.id.txid[..].to_vec();
        let pointer = metashrew_core::index_pointer::IndexPointer::from_keyword(
            Self::ACTIVATION_MAP_PREFIX,
        ).select(&reveal_txid_bytes);
        let stored = pointer.get();
        if !stored.is_empty() && stored.len() == 32 {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&stored);
            bytes.reverse();
            return B256::from(bytes);
        }
        Self::txid_to_b256_be(&entry.id.txid)
    }

    fn execute_deploy(&mut self, entry: &InscriptionEntry, op: DeployOp, block: &Block) {
        // Debug: log deploy bytecode info (stored per-deploy, indexed by sequence)
        {
            let d_len = op.d.len();
            let prefix = &op.d[..op.d.len().min(20)];
            let suffix = if op.d.len() > 20 { &op.d[op.d.len()-20..] } else { "" };
            let info = format!("d_len={} prefix={} suffix={}", d_len, prefix, suffix);
            // Store as last_deploy AND also as deploy/<d_len> for per-contract lookup
            let mut ptr = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_deploy");
            ptr.set(Arc::new(info.clone().into_bytes()));
            let key = format!("/debug/deploy/{}", d_len);
            let mut ptr2 = metashrew_core::index_pointer::IndexPointer::from_keyword(&key);
            ptr2.set(Arc::new(info.into_bytes()));
        }

        let hex_str = op.d.strip_prefix("0x").unwrap_or(&op.d);
        let data: Bytes = hex::decode(hex_str).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);
        let sender = derive_sender_address(entry, block);

        let mut evm = self.build_evm(Self::resolve_op_return_tx_id(entry, block));
        let tx = make_tx(TxKind::Create, data, gas_limit, sender);

        let result = evm.transact_commit(tx);
        // Debug: log deploy result
        {
            use revm::context::result::ExecutionResult;
            let result_str = match &result {
                Ok(ExecutionResult::Success { output, gas_used, .. }) => {
                    let addr = match output {
                        Output::Create(_, Some(a)) => format!("0x{}", hex::encode(a.as_slice())),
                        Output::Create(_, None) => "create_no_addr".to_string(),
                        Output::Call(_) => "call_output".to_string(),
                    };
                    format!("success,gas={},addr={}", gas_used, addr)
                }
                Ok(ExecutionResult::Revert { output, gas_used }) =>
                    format!("revert,gas={},out_len={}", gas_used, output.len()),
                Ok(ExecutionResult::Halt { reason, gas_used }) =>
                    format!("halt,{:?},gas={}", reason, gas_used),
                Err(e) => format!("error,{:?}", e),
            };
            let mut ptr = metashrew_core::index_pointer::IndexPointer::from_keyword("/debug/last_deploy_result");
            ptr.set(Arc::new(result_str.into_bytes()));
        }

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
        // Debug: store the inscription for diagnosis
        {
            let debug_content = format!(
                "op=call c={:?} d_len={} d_prefix={}",
                op.c.as_deref().unwrap_or("none"),
                op.d.len(),
                &op.d[..op.d.len().min(20)],
            );
            let mut ptr = metashrew_core::index_pointer::IndexPointer::from_keyword(DEBUG_LAST_INSCRIPTION_KEY);
            ptr.set(Arc::new(debug_content.into_bytes()));
        }

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

        let mut evm = self.build_evm(Self::resolve_op_return_tx_id(entry, block));
        let tx = make_tx(TxKind::Call(address), data, gas_limit, sender);

        // Store execution result for debugging
        {
            use revm::context::result::ExecutionResult;
            let result_str = match evm.transact_commit(tx) {
                Ok(ExecutionResult::Success { gas_used, .. }) =>
                    format!("success,gas={}", gas_used),
                Ok(ExecutionResult::Revert { output, gas_used }) => {
                    let rh = hex::encode(&output);
                    format!("revert,gas={},out={}", gas_used, &rh[..rh.len().min(200)])
                }
                Ok(ExecutionResult::Halt { reason, gas_used }) =>
                    format!("halt,{:?},gas={}", reason, gas_used),
                Err(e) =>
                    format!("error,{:?}", e),
            };
            let mut ptr = metashrew_core::index_pointer::IndexPointer::from_keyword(DEBUG_LAST_RESULT_KEY);
            ptr.set(Arc::new(result_str.into_bytes()));
        }
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

        let mut evm = self.build_evm(Self::resolve_op_return_tx_id(entry, block));
        let tx = make_tx(TxKind::Call(address), data, gas_limit, sender);

        let _ = evm.transact_commit(tx);
    }

    /// Execute a call with pre-computed sender and op_return_tx_id (for deferred execution).
    fn execute_call_with_sender(&mut self, _entry: &InscriptionEntry, op: CallOp, sender: Address, op_return_tx_id: B256) {
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
            return;
        };

        let data_hex = op.d.strip_prefix("0x").unwrap_or(&op.d);
        let data: Bytes = hex::decode(data_hex).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);

        let mut evm = self.build_evm(op_return_tx_id);
        let tx = make_tx(TxKind::Call(address), data, gas_limit, sender);
        let _ = evm.transact_commit(tx);
    }

    /// Execute a transact with pre-computed sender and op_return_tx_id (for deferred execution).
    fn execute_transact_with_sender(&mut self, _entry: &InscriptionEntry, op: TransactOp, sender: Address, op_return_tx_id: B256) {
        let addr_hex = op.to.as_deref().or(op.c.as_deref()).unwrap_or("");
        let hex = addr_hex.strip_prefix("0x").unwrap_or(addr_hex);
        let to_bytes = hex::decode(hex).unwrap_or_default();
        if to_bytes.len() != 20 { return; }

        let address = Address::from_slice(&to_bytes);
        let data_hex = op.d.strip_prefix("0x").unwrap_or(&op.d);
        let data: Bytes = hex::decode(data_hex).unwrap_or_default().into();
        let gas_limit = (data.len() as u64 * BRC20_PROG_GAS_PER_BYTE).min(BRC20_PROG_MAX_CALL_GAS);

        let mut evm = self.build_evm(op_return_tx_id);
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
