use crate::mock::*;
use frame_support::assert_ok;
use frame_system::Config;
use sp_core::U256;

mod mock;

/********************************************
    tests for uids.rs file
*********************************************/

/********************************************
    tests uids::replace_agent()
*********************************************/

#[test]
fn test_replace_agent() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            111111,
            &computekey_account_id,
        );
        let personalkey_account_id = U256::from(1234);

        let new_computekey_account_id = U256::from(2);
        let _new_colkey_account_id = U256::from(12345);

        //add network
        add_network(netuid, tempo, 0);

        // Register a agent.
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id
        ));

        // Get UID
        let agent_uid = BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id);
        assert_ok!(agent_uid);

        // Replace the agent.
        BasedNode::replace_agent(
            netuid,
            agent_uid.unwrap(),
            &new_computekey_account_id,
            block_number,
        );

        // Check old computekey is not registered on any network.
        assert!(BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).is_err());
        assert!(!BasedNode::is_computekey_registered_on_any_network(
            &computekey_account_id
        ));

        let curr_computekey = BasedNode::get_computekey_for_net_and_uid(netuid, agent_uid.unwrap());
        assert_ok!(curr_computekey);
        assert_ne!(curr_computekey.unwrap(), computekey_account_id);

        // Check new computekey is registered on the network.
        assert!(
            BasedNode::get_uid_for_net_and_computekey(netuid, &new_computekey_account_id).is_ok()
        );
        assert!(BasedNode::is_computekey_registered_on_any_network(
            &new_computekey_account_id
        ));
        assert_eq!(curr_computekey.unwrap(), new_computekey_account_id);
    });
}

#[test]
fn test_replace_agent_multiple_brains() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let netuid1: u16 = 2;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let new_computekey_account_id = U256::from(2);

        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            111111,
            &computekey_account_id,
        );
        let (nonce1, work1): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid1,
            block_number,
            111111 * 5,
            &computekey_account_id,
        );

        let personalkey_account_id = U256::from(1234);

        let _new_colkey_account_id = U256::from(12345);

        //add network
        add_network(netuid, tempo, 0);
        add_network(netuid1, tempo, 0);

        // Register a agent on both networks.
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id
        ));
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid1,
            block_number,
            nonce1,
            work1,
            computekey_account_id,
            personalkey_account_id
        ));

        // Get UID
        let agent_uid = BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id);
        assert_ok!(agent_uid);

        // Verify agent is registered on both networks.
        assert!(BasedNode::is_computekey_registered_on_network(
            netuid,
            &computekey_account_id
        ));
        assert!(BasedNode::is_computekey_registered_on_network(
            netuid1,
            &computekey_account_id
        ));
        assert!(BasedNode::is_computekey_registered_on_any_network(
            &computekey_account_id
        ));

        // Replace the agent.
        // Only replace on ONE network.
        BasedNode::replace_agent(
            netuid,
            agent_uid.unwrap(),
            &new_computekey_account_id,
            block_number,
        );

        // Check old computekey is not registered on netuid network.
        assert!(BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).is_err());

        // Verify still registered on netuid1 network.
        assert!(BasedNode::is_computekey_registered_on_any_network(
            &computekey_account_id
        ));
        assert!(BasedNode::is_computekey_registered_on_network(
            netuid1,
            &computekey_account_id
        ));
    });
}

#[test]
fn test_replace_agent_multiple_brains_unstake_all() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let netuid1: u16 = 2;
        let tempo: u16 = 13;

        let computekey_account_id = U256::from(1);
        let new_computekey_account_id = U256::from(2);

        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            111111,
            &computekey_account_id,
        );
        let (nonce1, work1): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid1,
            block_number,
            111111 * 5,
            &computekey_account_id,
        );

        let personalkey_account_id = U256::from(1234);
        let personalkey_account1_id = U256::from(1235);
        let personalkey_account2_id = U256::from(1236);

        let stake_amount = 1000;

        //add network
        add_network(netuid, tempo, 0);
        add_network(netuid1, tempo, 0);

        // Register a agent on both networks.
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id
        ));
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid1,
            block_number,
            nonce1,
            work1,
            computekey_account_id,
            personalkey_account_id
        ));

        // Get UID
        let agent_uid = BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id);
        assert_ok!(agent_uid);

        // Stake on agent with multiple personalkeys.
        BasedNode::increase_stake_on_personalkey_computekey_account(
            &personalkey_account_id,
            &computekey_account_id,
            stake_amount,
        );
        BasedNode::increase_stake_on_personalkey_computekey_account(
            &personalkey_account1_id,
            &computekey_account_id,
            stake_amount + 1,
        );
        BasedNode::increase_stake_on_personalkey_computekey_account(
            &personalkey_account2_id,
            &computekey_account_id,
            stake_amount + 2,
        );

        // Check stake on agent
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account_id,
                &computekey_account_id
            ),
            stake_amount
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account1_id,
                &computekey_account_id
            ),
            stake_amount + 1
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account2_id,
                &computekey_account_id
            ),
            stake_amount + 2
        );

        // Check total stake on agent
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            (stake_amount * 3) + (1 + 2)
        );

        // Replace the agent.
        BasedNode::replace_agent(
            netuid,
            agent_uid.unwrap(),
            &new_computekey_account_id,
            block_number,
        );

        // The stakes should still be on the agent. It is still registered on one network.
        assert!(BasedNode::is_computekey_registered_on_any_network(
            &computekey_account_id
        ));

        // Check the stake is still on the personalkey accounts.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account_id,
                &computekey_account_id
            ),
            stake_amount
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account1_id,
                &computekey_account_id
            ),
            stake_amount + 1
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account2_id,
                &computekey_account_id
            ),
            stake_amount + 2
        );

        // Check total stake on agent
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            (stake_amount * 3) + (1 + 2)
        );

        // replace on second network
        BasedNode::replace_agent(
            netuid1,
            agent_uid.unwrap(),
            &new_computekey_account_id,
            block_number,
        );

        // The agent should be unregistered now.
        assert!(!BasedNode::is_computekey_registered_on_any_network(
            &computekey_account_id
        ));

        // Check the stake is now on the free balance of the personalkey accounts.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account_id,
                &computekey_account_id
            ),
            0
        );
        assert_eq!(Balances::free_balance(&personalkey_account_id), stake_amount);

        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account1_id,
                &computekey_account_id
            ),
            0
        );
        assert_eq!(
            Balances::free_balance(&personalkey_account1_id),
            stake_amount + 1
        );

        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(
                &personalkey_account2_id,
                &computekey_account_id
            ),
            0
        );
        assert_eq!(
            Balances::free_balance(&personalkey_account2_id),
            stake_amount + 2
        );

        // Check total stake on agent
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
    });
}
