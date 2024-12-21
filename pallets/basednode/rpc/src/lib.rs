//! RPC interface for the custom Basednode rpc methods

use jsonrpsee::{
    core::RpcResult,
    proc_macros::rpc,
    types::error::{CallError, ErrorObject},
};
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

use sp_api::ProvideRuntimeApi;
use sp_api::HeaderT;
use sp_api::Encode;

pub use basednode_custom_rpc_runtime_api::{
    DelegateInfoRuntimeApi, AgentInfoRuntimeApi, BrainInfoRuntimeApi,
    BrainRegistrationRuntimeApi, TftEnforcerDataRuntimeApi
};

#[rpc(client, server)]
pub trait BasednodeCustomApi<BlockHash> {
    #[method(name = "delegateInfo_getDelegates")]
    fn get_delegates(&self, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
    #[method(name = "delegateInfo_getDelegate")]
    fn get_delegate(
        &self,
        delegate_account_vec: Vec<u8>,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<u8>>;
    #[method(name = "delegateInfo_getDelegated")]
    fn get_delegated(
        &self,
        delegatee_account_vec: Vec<u8>,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<u8>>;

    #[method(name = "agentInfo_getAgentsLite")]
    fn get_agents_lite(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
    #[method(name = "agentInfo_getAgentLite")]
    fn get_agent_lite(&self, netuid: u16, uid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
    #[method(name = "agentInfo_getAgents")]
    fn get_agents(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
    #[method(name = "agentInfo_getAgent")]
    fn get_agent(&self, netuid: u16, uid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;

    #[method(name = "brainInfo_getBrainInfo")]
    fn get_brain_info(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
    #[method(name = "brainInfo_getBrainsInfo")]
    fn get_brains_info(&self, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
    #[method(name = "brainInfo_getBrainHyperparams")]
    fn get_brain_hyperparams(&self, netuid: u16, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;

    #[method(name = "brainInfo_getLockCost")]
    fn get_network_lock_cost(&self, at: Option<BlockHash>) -> RpcResult<u128>;

    #[method(name = "tftEnforcer_getTftEnforcerData")]
    fn get_tft_enforcer_data(&self, from_block: Option<BlockHash>, block_count: Option<u64>, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
}

pub struct BasednodeCustom<C, P> {
    /// Shared reference to the client.
    client: Arc<C>,
    _marker: std::marker::PhantomData<P>,
}

impl<C, P> BasednodeCustom<C, P> {
    /// Creates a new instance of the TransactionPayment Rpc helper.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

/// Error type of this RPC api.
pub enum Error {
    /// The call to runtime failed.
    RuntimeError,
}

impl From<Error> for i32 {
    fn from(e: Error) -> i32 {
        match e {
            Error::RuntimeError => 1,
        }
    }
}

impl<C, Block> BasednodeCustomApiServer<<Block as BlockT>::Hash> for BasednodeCustom<C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: DelegateInfoRuntimeApi<Block>,
    C::Api: AgentInfoRuntimeApi<Block>,
    C::Api: BrainInfoRuntimeApi<Block>,
    C::Api: BrainRegistrationRuntimeApi<Block>,
    C::Api: TftEnforcerDataRuntimeApi<Block>,
{
    fn get_delegates(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_delegates(at).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get delegates info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_delegate(
        &self,
        delegate_account_vec: Vec<u8>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_delegate(at, delegate_account_vec).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get delegate info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_delegated(
        &self,
        delegatee_account_vec: Vec<u8>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_delegated(at, delegatee_account_vec).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get delegated info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_agents_lite(
        &self,
        netuid: u16,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_agents_lite(at, netuid).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get agents lite info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_agent_lite(
        &self,
        netuid: u16,
        uid: u16,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_agent_lite(at, netuid, uid).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get agent lite info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_agents(&self, netuid: u16, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_agents(at, netuid).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get agents info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_agent(
        &self,
        netuid: u16,
        uid: u16,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_agent(at, netuid, uid).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get agent info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_brain_info(
        &self,
        netuid: u16,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_brain_info(at, netuid).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get brain info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_brain_hyperparams(
        &self,
        netuid: u16,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_brain_hyperparams(at, netuid).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get brain info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_brains_info(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_brains_info(at).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get brains info.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_network_lock_cost(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<u128> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        api.get_network_registration_cost(at).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Unable to get brain lock cost.",
                Some(e.to_string()),
            ))
            .into()
        })
    }

    fn get_tft_enforcer_data(&self, from_block: Option<<Block as BlockT>::Hash>, block_count: Option<u64>, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
        let api = self.client.runtime_api();
        let from_block = from_block.unwrap_or_else(|| self.client.info().best_hash);
        api.get_tft_enforcer_data(from_block, from_block.encode(), block_count).map_err(|e| {
            CallError::Custom(ErrorObject::owned(
                Error::RuntimeError.into(),
                "Cannot get TFT enforcer data",
                Some(e.to_string()),
            ))
            .into()
        })

    }
}
