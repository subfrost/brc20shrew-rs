use crate::proto::{CallRequest, CallResponse};
use shrew_evm::database::MetashrewDB;
use revm::primitives::{TransactTo, ExecutionResult, Output, Address};
use revm::EVM;

/// Execute a read-only EVM call (eth_call style)
pub fn call(request: &CallRequest) -> Result<CallResponse, String> {
    let mut response = CallResponse::default();

    if request.to.len() != 20 {
        response.error = "Invalid 'to' address".to_string();
        return Ok(response);
    }

    let to = Address::from_slice(&request.to);
    let mut evm = EVM::<MetashrewDB>::new();
    evm.env.tx.transact_to = TransactTo::Call(to);
    evm.env.tx.data = request.data.clone().into();
    evm.env.tx.gas_limit = u64::MAX;

    // Use transact() for read-only (doesn't commit state changes)
    match evm.transact() {
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
