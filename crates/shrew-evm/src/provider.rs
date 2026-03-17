//! Custom PrecompileProvider for BRC20-prog EVM.
//!
//! Wraps standard Ethereum precompiles + our 5 custom Bitcoin precompiles.
//! Used by prog_indexer instead of `build_mainnet()`.

use crate::precompiles;
use revm::primitives::{Address, Bytes, B256};
use revm::interpreter::{Gas, InterpreterResult, CallInput, CallInputs, InstructionResult};
use revm::context_interface::{Cfg, ContextTr};
use revm::handler::{EthPrecompiles, PrecompileProvider};
use revm::primitives::hardfork::SpecId;

/// Custom precompile provider that combines Ethereum precompiles with
/// BRC20-prog Bitcoin precompiles (0xFA-0xFE).
pub struct ShrewPrecompiles {
    eth: EthPrecompiles,
    pub op_return_tx_id: B256,
    pub current_height: u32,
}

impl ShrewPrecompiles {
    pub fn new(spec: SpecId, op_return_tx_id: B256, current_height: u32) -> Self {
        Self {
            eth: EthPrecompiles::new(spec),
            op_return_tx_id,
            current_height,
        }
    }
}

impl<CTX: ContextTr> PrecompileProvider<CTX> for ShrewPrecompiles {
    type Output = InterpreterResult;

    fn set_spec(&mut self, spec: <CTX::Cfg as Cfg>::Spec) -> bool {
        let spec_id: SpecId = spec.into();
        if spec_id == self.eth.spec {
            return false;
        }
        self.eth.precompiles = revm::precompile::Precompiles::new(
            revm::precompile::PrecompileSpecId::from_spec_id(spec_id)
        );
        self.eth.spec = spec_id;
        true
    }

    fn run(
        &mut self,
        context: &mut CTX,
        inputs: &CallInputs,
    ) -> Result<Option<InterpreterResult>, String> {
        // Try custom BRC20-prog precompiles first
        if precompiles::is_precompile(&inputs.bytecode_address) {
            // Extract input bytes from CallInput
            let input_bytes: Vec<u8> = match &inputs.input {
                CallInput::Bytes(bytes) => bytes.0.to_vec(),
                _ => vec![],
            };

            let result = precompiles::execute_precompile(
                &inputs.bytecode_address,
                &input_bytes,
                inputs.gas_limit,
                self.op_return_tx_id,
                self.current_height,
            );

            if let Some(precompile_result) = result {
                let mut gas = Gas::new(inputs.gas_limit);
                let _ = gas.record_cost(precompile_result.gas_used);

                return Ok(Some(InterpreterResult {
                    result: if precompile_result.success {
                        InstructionResult::Return
                    } else {
                        InstructionResult::Revert
                    },
                    gas,
                    output: Bytes::from(precompile_result.output),
                }));
            }
        }

        // Fall through to standard Ethereum precompiles
        self.eth.run(context, inputs)
    }

    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        let custom = precompiles::precompile_addresses();
        let eth_addrs: Vec<Address> = self.eth.precompiles.addresses().cloned().collect();
        Box::new(custom.into_iter().chain(eth_addrs.into_iter()))
    }

    fn contains(&self, address: &Address) -> bool {
        precompiles::is_precompile(address) || self.eth.contains(address)
    }
}
