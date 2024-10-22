//! This example shows how to implement a node with a custom EVM that uses a stateful precompile
use alloy_primitives::{address, Address, Bytes, U256};
use eyre::Ok;
use reth::{
    api::NextBlockEnvAttributes,
    builder::{components::ExecutorBuilder, BuilderContext},
    primitives::revm_primitives::{BlockEnv, CfgEnvWithHandlerCfg, Env, PrecompileResult, TxEnv},
    revm::{
        handler::register::EvmHandler, inspector_handle_register, precompile::{Precompile, PrecompileSpecId}, primitives::PrecompileOutput, ContextPrecompile, ContextPrecompiles, Database, Evm, EvmBuilder, GetInspector
    },
};
use reth::chainspec::ChainSpec;
use reth_node_api::{ConfigureEvm, ConfigureEvmEnv, FullNodeTypes, NodeTypes};
use reth_node_ethereum::{ EthEvmConfig, EthExecutorProvider};
use reth::primitives::{
    revm_primitives::StatefulPrecompileMut,
    Header, TransactionSigned,
};
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
        Self { inner: EthEvmConfig::new(chain_spec) }
    }

    /// Sets the precompiles to the EVM handler
    ///
    /// This will be invoked when the EVM is created via [ConfigureEvm::evm] or
    /// [ConfigureEvm::evm_with_inspector]
    ///
    /// This will use the default mainnet precompiles and wrap them with a cache.
    pub fn set_precompiles<EXT, DB>(
        handler: &mut EvmHandler<EXT, DB>,
    ) where
        DB: Database,
    {
        // first we need the evm spec id, which determines the precompiles
        let spec_id = handler.cfg.spec_id;

        let mut loaded_precompiles: ContextPrecompiles<DB> =
            ContextPrecompiles::new(PrecompileSpecId::from_spec_id(spec_id));

        loaded_precompiles.extend([(
            address!("9900000000000000000000000000000000000001"),
            Self::twine_precompile()
        )]);

        // install the precompiles
        handler.pre_execution.load_precompiles = Arc::new(move || loaded_precompiles.clone());
    }

    /// Given a [`ContextPrecompile`] and cache for a specific precompile, create a new precompile
    /// that wraps the precompile with the cache.
    fn twine_precompile<DB>() -> ContextPrecompile<DB>
    where
        DB: Database,
    {
        let wrapped = TwinePrecompile {};

        ContextPrecompile::Ordinary(Precompile::StatefulMut(Box::new(wrapped)))
    }
}

/// A custom precompile that contains the cache and precompile it wraps.
#[derive(Clone)]
pub struct TwinePrecompile {}

impl StatefulPrecompileMut for TwinePrecompile {
    fn call_mut(&mut self, _bytes: &Bytes, _gas_price: u64, _env: &Env) -> PrecompileResult {
        let initiator = _env.tx.caller;
        reth_tracing::tracing::info!("the initiator is : {:?}", initiator);
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
        self.inner.fill_tx_env_system_contract_call(env, caller, contract, data)
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
        Ok((evm_config.clone(), EthExecutorProvider::new(ctx.chain_spec(), evm_config)))
    }
}
