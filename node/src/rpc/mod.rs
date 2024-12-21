//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use basednode_runtime::{opaque::Block, AccountId, Balance, Hash, Index};
use futures::channel::mpsc;
use jsonrpsee::RpcModule;
use sc_client_api::{
    backend::{Backend, StorageProvider},
    client::BlockchainEvents,
};
use sc_consensus_manual_seal::rpc::EngineCommand;
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool::ChainApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::traits::Block as BlockT;

pub use sc_rpc_api::DenyUnsafe;

mod eth;
pub use self::eth::{create_eth, overrides_handle, EthDeps};

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi, CT> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// Manual seal command sink
    pub command_sink: Option<mpsc::Sender<EngineCommand<Hash>>>,
    /// Ethereum-compatibility specific dependencies.
    pub eth: EthDeps<C, P, A, CT, Block>,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, BE, A, CT>(
    deps: FullDeps<C, P, A, CT>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
    C::Api: sp_block_builder::BlockBuilder<Block>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
    C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
    C::Api: basednode_custom_rpc_runtime_api::DelegateInfoRuntimeApi<Block>,
    C::Api: basednode_custom_rpc_runtime_api::AgentInfoRuntimeApi<Block>,
    C::Api: basednode_custom_rpc_runtime_api::BrainInfoRuntimeApi<Block>,
    C::Api: basednode_custom_rpc_runtime_api::BrainRegistrationRuntimeApi<Block>,
    C::Api: basednode_custom_rpc_runtime_api::TftEnforcerDataRuntimeApi<Block>,
    C: BlockchainEvents<Block> + 'static,
    C: HeaderBackend<Block>
        + HeaderMetadata<Block, Error = BlockChainError>
        + StorageProvider<Block, BE>,
    BE: Backend<Block> + 'static,
    P: TransactionPool<Block = Block> + 'static,
    A: ChainApi<Block = Block> + 'static,
    CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Send + Sync + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use sc_consensus_manual_seal::rpc::{ManualSeal, ManualSealApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};
    use basednode_custom_rpc::{BasednodeCustom, BasednodeCustomApiServer};

    let mut io = RpcModule::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        command_sink,
        eth,
    } = deps;

    io.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
    io.merge(BasednodeCustom::new(client).into_rpc())?;

    if let Some(command_sink) = command_sink {
        io.merge(
            // We provide the rpc handler with the sending end of the channel to allow the rpc
            // send EngineCommands to the background block authorship task.
            ManualSeal::new(command_sink).into_rpc(),
        )?;
    }

    // Ethereum compatibility RPCs
    let io = create_eth::<_, _, _, _, _, _>(io, eth, subscription_task_executor)?;

    Ok(io)
}
