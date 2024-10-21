//! This is the precompile for the L2 bridge.

use reth_tracing::tracing::{error, info};
use revm::{
    handler::register::EvmHandler,
    primitives::{PrecompileOutput, PrecompileResult},
    ContextPrecompile, ContextStatefulPrecompile, Database,
};
use revm_primitives::{address, Bytes, SpecId, U256};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgePrecompile {}

/// for now lets implement the precompile for the deposit transaction only
impl<DB: Database> ContextStatefulPrecompile<DB> for BridgePrecompile {
    fn call(
        &self,
        bytes: &Bytes,
        gas_limit: u64,
        evmctx: &mut revm::InnerEvmContext<DB>,
    ) -> PrecompileResult {
        _=bytes;
        _=gas_limit;
        let balance = evmctx.balance(evmctx.env.tx.caller);

        match balance {
            Ok(b) => {
                info!("The balance is {:?}", b);
            }
            Err(_) => {
                error!("Balance couldnot be found");
            }
        }
        Ok(PrecompileOutput::new(0, Bytes::new()))
    }
}

pub fn set_evm_handles<EXT, DB>(handler: &mut EvmHandler<EXT, DB>)
where
    DB: Database,
{
    let precompiles = handler.pre_execution.load_precompiles.clone();

    handler.pre_execution.load_precompiles = Arc::new(move || {
        let mut precompiles = precompiles();
        precompiles.extend([(
            address!("9900000000000000000000000000000000000001"),
            ContextPrecompile::ContextStateful(Arc::new(BridgePrecompile {})),
        )]);
        precompiles
    })
}
