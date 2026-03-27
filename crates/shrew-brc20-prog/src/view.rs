use crate::proto::{CallRequest, CallResponse};
use shrew_evm::database::MetashrewDB;
use shrew_evm::ShrewPrecompiles;
use revm::primitives::{Address, B256, U256, TxKind};
use revm::primitives::hardfork::SpecId;
use revm::context::result::{ExecutionResult, Output};
use revm::context::{Context, TxEnv, BlockEnv, CfgEnv, Journal, FrameStack, Evm};
use revm::handler::instructions::EthInstructions;
use revm::handler::EthFrame;
use revm::interpreter::interpreter::EthInterpreter;
use revm::ExecuteEvm;
use shrew_support::constants::BRC20_PROG_MAX_CALL_GAS;

type Ctx = Context<BlockEnv, TxEnv, CfgEnv, MetashrewDB, Journal<MetashrewDB>, ()>;
type ViewEvm = Evm<Ctx, (), EthInstructions<EthInterpreter, Ctx>, ShrewPrecompiles, EthFrame<EthInterpreter>>;

/// BRC20-prog chain ID: 0x4252433230 ("BRC20" in ASCII)
const BRC20_PROG_CHAIN_ID: u64 = 0x4252433230;

/// Build an EVM instance for view calls, matching the indexer's configuration exactly.
/// This ensures that precompiles, instruction tables, and spec handling are identical
/// to the execution environment used during indexing.
fn build_view_evm() -> ViewEvm {
    let spec = SpecId::PRAGUE;
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

    ctx.block.gas_limit = u64::MAX;
    ctx.block.basefee = 0;
    ctx.block.difficulty = U256::ZERO;

    // Build EVM with the same precompiles and instruction set as the indexer.
    // Previously used ctx.build_mainnet() which creates EthPrecompiles::default()
    // and EthInstructions::new_mainnet() — both ignoring the configured spec.
    let precompiles = ShrewPrecompiles::new(
        spec.into(),
        B256::ZERO, // No OP_RETURN tx context for view calls
        0,          // Height not needed for view-only precompile calls
    );

    Evm {
        ctx,
        inspector: (),
        instruction: EthInstructions::new_mainnet_with_spec(spec.into()),
        precompiles,
        frame_stack: FrameStack::new_prealloc(8),
    }
}

/// Execute a read-only EVM call (eth_call style)
pub fn call(request: &CallRequest) -> Result<CallResponse, String> {
    let mut response = CallResponse::default();

    if request.to.len() != 20 {
        response.error = "Invalid 'to' address".to_string();
        return Ok(response);
    }

    let to = Address::from_slice(&request.to);

    let mut evm = build_view_evm();

    let tx = TxEnv {
        kind: TxKind::Call(to),
        data: revm::primitives::Bytes::from(request.data.clone()),
        gas_limit: u64::MAX,
        gas_price: 0,
        value: U256::ZERO,
        chain_id: Some(BRC20_PROG_CHAIN_ID),
        ..Default::default()
    };

    // Use transact() for read-only (doesn't commit state changes)
    match evm.transact(tx) {
        Ok(result_and_state) => {
            match result_and_state.result {
                ExecutionResult::Success { output, .. } => {
                    match output {
                        Output::Call(bytes) => {
                            response.result = bytes.to_vec();
                            response.success = true;
                        }
                        _ => {
                            response.success = true;
                        }
                    }
                }
                ExecutionResult::Revert { output, .. } => {
                    response.result = output.to_vec();
                    response.error = "Execution reverted".to_string();
                }
                ExecutionResult::Halt { reason, .. } => {
                    response.error = format!("Execution halted: {:?}", reason);
                }
            }
        }
        Err(e) => {
            response.error = format!("EVM error: {:?}", e);
        }
    }

    Ok(response)
}
