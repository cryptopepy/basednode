#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
use alloc::vec::Vec;

// Here we declare the runtime API. It is implemented it the `impl` block in
// src/tft_enforcer_data.rs, src/agent_info.rs, src/brain_info.rs,
// and src/delegate_info.rs
sp_api::decl_runtime_apis! {
    pub trait DelegateInfoRuntimeApi {
        fn get_delegates() -> Vec<u8>;
        fn get_delegate( delegate_account_vec: Vec<u8> ) -> Vec<u8>;
        fn get_delegated( delegatee_account_vec: Vec<u8> ) -> Vec<u8>;
    }

    pub trait AgentInfoRuntimeApi {
        fn get_agents(netuid: u16) -> Vec<u8>;
        fn get_agent(netuid: u16, uid: u16) -> Vec<u8>;
        fn get_agents_lite(netuid: u16) -> Vec<u8>;
        fn get_agent_lite(netuid: u16, uid: u16) -> Vec<u8>;
    }

    pub trait BrainInfoRuntimeApi {
        fn get_brain_info(netuid: u16) -> Vec<u8>;
        fn get_brains_info() -> Vec<u8>;
        fn get_brain_hyperparams(netuid: u16) -> Vec<u8>;
    }

    pub trait StakeInfoRuntimeApi {
        fn get_stake_info_for_personalkey( personalkey_account_vec: Vec<u8> ) -> Vec<u8>;
        fn get_stake_info_for_personalkeys( personalkey_account_vecs: Vec<Vec<u8>> ) -> Vec<u8>;
    }

    pub trait BrainRegistrationRuntimeApi {
        fn get_network_registration_cost() -> u128;
    }

    pub trait TftEnforcerDataRuntimeApi {
        fn get_tft_enforcer_data(from_block: Vec<u8>, block_count: Option<u64>) -> Vec<u8>;
    }
}
