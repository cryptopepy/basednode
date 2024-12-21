mod mock;
use mock::*;

use sp_core::U256;

#[test]
fn test_get_agent_none() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let uid: u16 = 42;

        let agent = BasedNode::get_agent(netuid, uid);
        assert_eq!(agent, None);
    });
}

#[test]
#[cfg(not(tarpaulin))]
fn test_get_agent_some() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;

        let tempo: u16 = 2;
        let modality: u16 = 2;

        let uid: u16 = 0;
        let computekey0 = U256::from(0);
        let personalkey0 = U256::from(0);

        add_network(netuid, tempo, modality);
        register_ok_agent(netuid, computekey0, personalkey0, 39420842);

        let agent = BasedNode::get_agent(netuid, uid);
        assert_ne!(agent, None);
    });
}

/* @TODO: Add more agents to list */
#[test]
fn test_get_agents_list() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;

        let tempo: u16 = 2;
        let modality: u16 = 2;

        add_network(netuid, tempo, modality);

        let _uid: u16 = 42;

        let agent_count = 1;
        for index in 0..agent_count {
            let computekey = U256::from(0 + index);
            let personalkey = U256::from(0 + index);
            let nonce: u64 = 39420842 + index;
            register_ok_agent(netuid, computekey, personalkey, nonce);
        }

        let agents = BasedNode::get_agents(netuid);
        assert_eq!(agents.len(), agent_count as usize);
    });
}

#[test]
fn test_get_agents_empty() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;

        let agent_count = 0;
        let agents = BasedNode::get_agents(netuid);
        assert_eq!(agents.len(), agent_count as usize);
    });
}
