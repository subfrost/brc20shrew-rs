use crate::proto::{CallRequest, CallResponse};
use shrew_evm::database::MetashrewDB;
use revm::primitives::{Address, U256, TxKind};
use revm::primitives::hardfork::SpecId;
use revm::context::result::{ExecutionResult, Output};
use revm::context::{Context, TxEnv};
use revm::{MainBuilder, ExecuteEvm};

type Ctx = Context<revm::context::BlockEnv, TxEnv, revm::context::CfgEnv, shrew_evm::database::MetashrewDB, revm::context::Journal<shrew_evm::database::MetashrewDB>, ()>;

/// BRC20-prog chain ID: 0x4252433230 ("BRC20" in ASCII)
const BRC20_PROG_CHAIN_ID: u64 = 0x4252433230;

/// Execute a read-only EVM call (eth_call style)
pub fn call(request: &CallRequest) -> Result<CallResponse, String> {
    let mut response = CallResponse::default();

    if request.to.len() != 20 {
        response.error = "Invalid 'to' address".to_string();
        return Ok(response);
    }

    let to = Address::from_slice(&request.to);

    // Build EVM context with CANCUN spec (view calls don't need height-specific spec)
    let spec = SpecId::CANCUN;
    let mut ctx: Ctx = Context::new(MetashrewDB, spec);

    ctx.cfg.chain_id = BRC20_PROG_CHAIN_ID;
    ctx.cfg.spec = SpecId::CANCUN;
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

    let mut evm = ctx.build_mainnet();

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
