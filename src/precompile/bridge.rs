//! This example shows how to implement a node with a custom EVM that uses a stateful precompile
use alloy::hex::ToHexExt;
use alloy_primitives::{address, keccak256, Address, Bytes, U256};
use reth::chainspec::ChainSpec;
use reth::primitives::{Header, TransactionSigned};
use reth::revm::primitives::EvmStorageSlot;
use reth::revm::ContextStatefulPrecompile;
use reth::{
    api::NextBlockEnvAttributes,
    builder::{components::ExecutorBuilder, BuilderContext},
    primitives::revm_primitives::{BlockEnv, CfgEnvWithHandlerCfg, Env, PrecompileResult, TxEnv},
    revm::{
        handler::register::EvmHandler, inspector_handle_register, primitives::PrecompileOutput,
        ContextPrecompile, Database, Evm, EvmBuilder, GetInspector,
    },
};
use reth_node_api::{ConfigureEvm, ConfigureEvmEnv, FullNodeTypes, NodeTypes};
use reth_node_ethereum::{EthEvmConfig, EthExecutorProvider};
use std::sync::Arc;

/// Custom EVM configuration
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TwineEvmConfig {
    inner: EthEvmConfig,
}

impl TwineEvmConfig {
    /// Creates a new instance.
    pub fn _new(chain_spec: Arc<ChainSpec>) -> Self {
        Self {
            inner: EthEvmConfig::new(chain_spec),
        }
    }

    /// Sets the precompiles to the EVM handler
    ///
    /// This will be invoked when the EVM is created via [ConfigureEvm::evm] or
    /// [ConfigureEvm::evm_with_inspector]
    ///
    /// This will use the default mainnet precompiles and wrap them with a cache.
    pub fn set_precompiles<EXT, DB>(handler: &mut EvmHandler<EXT, DB>)
    where
        DB: Database,
    {
        // first we need the evm spec id, which determines the precompiles
        let prev_handle = handler.pre_execution.load_precompiles.clone();
        handler.pre_execution.load_precompiles = Arc::new(move || {
            let mut precompiles = prev_handle();

            precompiles.extend([(
                address!("9900000000000000000000000000000000000001"),
                ContextPrecompile::ContextStateful(Arc::new(TwinePrecompile {})),
            )]);
            precompiles
        });
    }
}

/// A custom precompile that contains the cache and precompile it wraps.
#[derive(Clone)]
pub struct TwinePrecompile {}

impl<DB: Database> ContextStatefulPrecompile<DB> for TwinePrecompile {
    fn call(
        &self,
        _bytes: &Bytes,
        _gas_limit: u64,
        evmctx: &mut reth::revm::InnerEvmContext<DB>,
    ) -> PrecompileResult {
        let mut key: Vec<u8> = Vec::new();
        key.append(&mut vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let initiator = evmctx.env.tx.caller;
        let initiator = initiator.0.as_slice();
        key.append(&mut initiator.to_vec());

        let mut position = vec![0u8; 64];
        key.append(&mut position);

        let key = keccak256(key);
        let encoded_key = key.encode_hex_with_prefix();

        reth_tracing::tracing::info!("the storage slot is: {}", encoded_key);

        let balance_address = address!("A51c1fc2f0D1a1b8494Ed1FE312d7C3a78Ed91C0");
        let balance_account = evmctx.load_account(balance_address);

        match balance_account {
            Ok(mut state) => {
                let key = U256::from_be_slice(key.as_slice());
                let storage_action = state.storage.insert(U256::from(0), EvmStorageSlot::new_changed(U256::from(2), U256::from(3)));
                match storage_action {
                    Some(slot) =>{
                        reth_tracing::tracing::info!("the storage slot that is changed is: {:?}", slot);
                    }
                    None => {
                        reth_tracing::tracing::error!("some error in updating the slot");
                    }
                }
                reth_tracing::tracing::info!("mutated the state is : {:?}", key);
    
            }
            Err(_) => {
                reth_tracing::tracing::error!("error in mutating the state");
            },
        }

        PrecompileResult::Ok(PrecompileOutput::new(0, Bytes::new()))
    }
}

impl ConfigureEvmEnv for TwineEvmConfig {
    type Header = Header;

    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
        self.inner.fill_tx_env(tx_env, transaction, sender)
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        self.inner
            .fill_tx_env_system_contract_call(env, caller, contract, data)
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Self::Header,
        total_difficulty: U256,
    ) {
        self.inner.fill_cfg_env(cfg_env, header, total_difficulty)
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> (CfgEnvWithHandlerCfg, BlockEnv) {
        self.inner.next_cfg_and_block_env(parent, attributes)
    }
}

impl ConfigureEvm for TwineEvmConfig {
    type DefaultExternalContext<'a> = ();

    fn evm<DB: Database>(&self, db: DB) -> Evm<'_, Self::DefaultExternalContext<'_>, DB> {
        EvmBuilder::default()
            .with_db(db)
            // add additional precompiles
            .append_handler_register_box(Box::new(move |handler| {
                TwineEvmConfig::set_precompiles(handler)
            }))
            .build()
    }

    fn evm_with_inspector<DB, I>(&self, db: DB, inspector: I) -> Evm<'_, I, DB>
    where
        DB: Database,
        I: GetInspector<DB>,
    {
        EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            // add additional precompiles
            .append_handler_register_box(Box::new(move |handler| {
                TwineEvmConfig::set_precompiles(handler)
            }))
            .append_handler_register(inspector_handle_register)
            .build()
    }

    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {}
}

/// Builds a regular ethereum block executor that uses the custom EVM.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct MyExecutorBuilder {}

impl<Node> ExecutorBuilder<Node> for MyExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec>>,
{
    type EVM = TwineEvmConfig;
    type Executor = EthExecutorProvider<Self::EVM>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let evm_config = TwineEvmConfig {
            inner: EthEvmConfig::new(ctx.chain_spec()),
        };
        Ok((
            evm_config.clone(),
            EthExecutorProvider::new(ctx.chain_spec(), evm_config),
        ))
    }
}

// #[test]
// fn test_address() {
//     let BRIDGEOUT_ADDRESS: Address = address!("9900000000000000000000000000000000000001");
//     println!("{:?}", BRIDGEOUT_ADDRESS);
// }
