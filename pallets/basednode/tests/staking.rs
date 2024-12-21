use frame_support::{assert_noop, assert_ok, traits::Currency};
use frame_system::Config;
mod mock;
use frame_support::dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, Pays};
use frame_support::sp_runtime::DispatchError;
use mock::*;
use pallet_basednode::Error;
use sp_core::{H256, U256};

/***********************************************************
    staking::add_stake() tests
************************************************************/

#[test]
#[cfg(not(tarpaulin))]
fn test_add_stake_dispatch_info_ok() {
    new_test_ext().execute_with(|| {
        let computekey = U256::from(0);
        let amount_staked = 5000;
        let call = RuntimeCall::BasedNode(BasednodeCall::add_stake {
            computekey,
            amount_staked,
        });
        assert_eq!(
            call.get_dispatch_info(),
            DispatchInfo {
                weight: frame_support::weights::Weight::from_ref_time(65000000),
                class: DispatchClass::Normal,
                pays_fee: Pays::No
            }
        );
    });
}
#[test]
fn test_add_stake_ok_no_emission() {
    new_test_ext().execute_with(|| {
        let computekey_account_id = U256::from(533453);
        let personalkey_account_id = U256::from(55453);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        // Register agent
        register_ok_agent(netuid, computekey_account_id, personalkey_account_id, start_nonce);

        // Give it some $$$ in his personalkey balance
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, 10000);

        // Check we have zero staked before transfer
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );

        // Also total stake should be zero
        assert_eq!(BasedNode::get_total_stake(), 0);

        // Transfer to computekey account, and check if the result is ok
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            computekey_account_id,
            10000
        ));

        // Check if stake has increased
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            10000
        );

        // Check if balance has  decreased
        assert_eq!(BasedNode::get_personalkey_balance(&personalkey_account_id), 0);

        // Check if total stake has increased accordingly.
        assert_eq!(BasedNode::get_total_stake(), 10000);
    });
}

#[test]
fn test_dividends_with_run_to_block() {
    new_test_ext().execute_with(|| {
        let agent_src_computekey_id = U256::from(1);
        let agent_dest_computekey_id = U256::from(2);
        let personalkey_account_id = U256::from(667);
        let netuid: u16 = 1;

        let initial_stake: u64 = 5000;

        //add network
        add_network(netuid, 13, 0);

        // Register agent, this will set a self weight
        BasedNode::set_max_registrations_per_block(netuid, 3);
        BasedNode::set_max_allowed_uids(1, 5);

        register_ok_agent(netuid, U256::from(0), personalkey_account_id, 2112321);
        register_ok_agent(netuid, agent_src_computekey_id, personalkey_account_id, 192213123);
        register_ok_agent(netuid, agent_dest_computekey_id, personalkey_account_id, 12323);

        // Add some stake to the computekey account, so we can test for emission before the transfer takes place
        BasedNode::increase_stake_on_computekey_account(&agent_src_computekey_id, initial_stake);

        // Check if the initial stake has arrived
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&agent_src_computekey_id),
            initial_stake
        );

        // Check if all three agents are registered
        assert_eq!(BasedNode::get_brain_n(netuid), 3);

        // Run a couple of blocks to check if emission works
        run_to_block(2);

        // Check if the stake is equal to the inital stake + transfer
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&agent_src_computekey_id),
            initial_stake
        );

        // Check if the stake is equal to the inital stake + transfer
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&agent_dest_computekey_id),
            0
        );
    });
}

#[test]
fn test_add_stake_err_signature() {
    new_test_ext().execute_with(|| {
        let computekey_account_id = U256::from(654); // bogus
        let amount = 20000; // Not used

        let result = BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::none(),
            computekey_account_id,
            amount,
        );
        assert_eq!(result, DispatchError::BadOrigin.into());
    });
}

#[test]
fn test_add_stake_not_registered_key_pair() {
    new_test_ext().execute_with(|| {
        let personalkey_account_id = U256::from(435445);
        let computekey_account_id = U256::from(54544);
        let amount = 1337;
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, 1800);
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
                computekey_account_id,
                amount
            ),
            Err(Error::<Test>::NotRegistered.into())
        );
    });
}

#[test]
fn test_add_stake_err_agent_does_not_belong_to_personalkey() {
    new_test_ext().execute_with(|| {
        let personalkey_id = U256::from(544);
        let computekey_id = U256::from(54544);
        let other_cold_key = U256::from(99498);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);
        // Give it some $$$ in his personalkey balance
        BasedNode::add_balance_to_personalkey_account(&other_cold_key, 100000);

        // Perform the request which is signed by a different cold key
        let result = BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(other_cold_key),
            computekey_id,
            1000,
        );
        assert_eq!(result, Err(Error::<Test>::NonAssociatedpersonalkey.into()));
    });
}

#[test]
fn test_add_stake_err_not_enough_belance() {
    new_test_ext().execute_with(|| {
        let personalkey_id = U256::from(544);
        let computekey_id = U256::from(54544);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);

        // Lets try to stake with 0 balance in cold key account
        assert_eq!(BasedNode::get_personalkey_balance(&personalkey_id), 0);
        let result = BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_id),
            computekey_id,
            60000,
        );

        assert_eq!(result, Err(Error::<Test>::NotEnoughBalanceToStake.into()));
    });
}

#[test]
#[ignore]
fn test_add_stake_total_balance_no_change() {
    // When we add stake, the total balance of the personalkey account should not change
    //    this is because the stake should be part of the personalkey account balance (reserved/locked)
    new_test_ext().execute_with(|| {
        let computekey_account_id = U256::from(551337);
        let personalkey_account_id = U256::from(51337);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        // Register agent
        register_ok_agent(netuid, computekey_account_id, personalkey_account_id, start_nonce);

        // Give it some $$$ in his personalkey balance
        let initial_balance = 10000;
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, initial_balance);

        // Check we have zero staked before transfer
        let initial_stake = BasedNode::get_total_stake_for_computekey(&computekey_account_id);
        assert_eq!(initial_stake, 0);

        // Check total balance is equal to initial balance
        let initial_total_balance = Balances::total_balance(&personalkey_account_id);
        assert_eq!(initial_total_balance, initial_balance);

        // Also total stake should be zero
        assert_eq!(BasedNode::get_total_stake(), 0);

        // Stake to computekey account, and check if the result is ok
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            computekey_account_id,
            10000
        ));

        // Check if stake has increased
        let new_stake = BasedNode::get_total_stake_for_computekey(&computekey_account_id);
        assert_eq!(new_stake, 10000);

        // Check if free balance has decreased
        let new_free_balance = BasedNode::get_personalkey_balance(&personalkey_account_id);
        assert_eq!(new_free_balance, 0);

        // Check if total stake has increased accordingly.
        assert_eq!(BasedNode::get_total_stake(), 10000);

        // Check if total balance has remained the same. (no fee, includes reserved/locked balance)
        let total_balance = Balances::total_balance(&personalkey_account_id);
        assert_eq!(total_balance, initial_total_balance);
    });
}

#[test]
#[ignore]
fn test_add_stake_total_issuance_no_change() {
    // When we add stake, the total issuance of the balances pallet should not change
    //    this is because the stake should be part of the personalkey account balance (reserved/locked)
    new_test_ext().execute_with(|| {
        let computekey_account_id = U256::from(561337);
        let personalkey_account_id = U256::from(61337);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        // Register agent
        register_ok_agent(netuid, computekey_account_id, personalkey_account_id, start_nonce);

        // Give it some $$$ in his personalkey balance
        let initial_balance = 10000;
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, initial_balance);

        // Check we have zero staked before transfer
        let initial_stake = BasedNode::get_total_stake_for_computekey(&computekey_account_id);
        assert_eq!(initial_stake, 0);

        // Check total balance is equal to initial balance
        let initial_total_balance = Balances::total_balance(&personalkey_account_id);
        assert_eq!(initial_total_balance, initial_balance);

        // Check total issuance is equal to initial balance
        let initial_total_issuance = Balances::total_issuance();
        assert_eq!(initial_total_issuance, initial_balance);

        // Also total stake should be zero
        assert_eq!(BasedNode::get_total_stake(), 0);

        // Stake to computekey account, and check if the result is ok
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            computekey_account_id,
            10000
        ));

        // Check if stake has increased
        let new_stake = BasedNode::get_total_stake_for_computekey(&computekey_account_id);
        assert_eq!(new_stake, 10000);

        // Check if free balance has decreased
        let new_free_balance = BasedNode::get_personalkey_balance(&personalkey_account_id);
        assert_eq!(new_free_balance, 0);

        // Check if total stake has increased accordingly.
        assert_eq!(BasedNode::get_total_stake(), 10000);

        // Check if total issuance has remained the same. (no fee, includes reserved/locked balance)
        let total_issuance = Balances::total_issuance();
        assert_eq!(total_issuance, initial_total_issuance);
    });
}

// /***********************************************************
// 	staking::remove_stake() tests
// ************************************************************/
#[test]
#[cfg(not(tarpaulin))]
fn test_remove_stake_dispatch_info_ok() {
    new_test_ext().execute_with(|| {
        let computekey = U256::from(0);
        let amount_unstaked = 5000;
        let call = RuntimeCall::BasedNode(BasednodeCall::remove_stake {
            computekey,
            amount_unstaked,
        });
        assert_eq!(
            call.get_dispatch_info(),
            DispatchInfo {
                weight: frame_support::weights::Weight::from_ref_time(63000000)
                    .add_proof_size(43991),
                class: DispatchClass::Normal,
                pays_fee: Pays::No
            }
        );
    });
}

#[test]
fn test_remove_stake_ok_no_emission() {
    new_test_ext().execute_with(|| {
        let personalkey_account_id = U256::from(4343);
        let computekey_account_id = U256::from(4968585);
        let amount = 10000;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        // Let's spin up a agent
        register_ok_agent(netuid, computekey_account_id, personalkey_account_id, start_nonce);

        // Some basic assertions
        assert_eq!(BasedNode::get_total_stake(), 0);
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
        assert_eq!(BasedNode::get_personalkey_balance(&personalkey_account_id), 0);

        // Give the agent some stake to remove
        BasedNode::increase_stake_on_computekey_account(&computekey_account_id, amount);

        // Do the magic
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            computekey_account_id,
            amount
        ));

        assert_eq!(
            BasedNode::get_personalkey_balance(&personalkey_account_id),
            amount
        );
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
        assert_eq!(BasedNode::get_total_stake(), 0);
    });
}

#[test]
fn test_remove_stake_amount_zero() {
    new_test_ext().execute_with(|| {
        let personalkey_account_id = U256::from(4343);
        let computekey_account_id = U256::from(4968585);
        let amount = 10000;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        // Let's spin up a agent
        register_ok_agent(netuid, computekey_account_id, personalkey_account_id, start_nonce);

        // Some basic assertions
        assert_eq!(BasedNode::get_total_stake(), 0);
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
        assert_eq!(BasedNode::get_personalkey_balance(&personalkey_account_id), 0);

        // Give the agent some stake to remove
        BasedNode::increase_stake_on_computekey_account(&computekey_account_id, amount);

        // Do the magic
        assert_noop!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
                computekey_account_id,
                0
            ),
            Error::<Test>::NotEnoughStaketoWithdraw
        );
    });
}

#[test]
fn test_remove_stake_err_signature() {
    new_test_ext().execute_with(|| {
        let computekey_account_id = U256::from(4968585);
        let amount = 10000; // Amount to be removed

        let result = BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::none(),
            computekey_account_id,
            amount,
        );
        assert_eq!(result, DispatchError::BadOrigin.into());
    });
}

#[test]
fn test_remove_stake_err_computekey_does_not_belong_to_personalkey() {
    new_test_ext().execute_with(|| {
        let personalkey_id = U256::from(544);
        let computekey_id = U256::from(54544);
        let other_cold_key = U256::from(99498);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);

        // Perform the request which is signed by a different cold key
        let result = BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(other_cold_key),
            computekey_id,
            1000,
        );
        assert_eq!(result, Err(Error::<Test>::NonAssociatedpersonalkey.into()));
    });
}

#[test]
fn test_remove_stake_no_enough_stake() {
    new_test_ext().execute_with(|| {
        let personalkey_id = U256::from(544);
        let computekey_id = U256::from(54544);
        let amount = 10000;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);

        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey_id), 0);

        let result = BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_id),
            computekey_id,
            amount,
        );
        assert_eq!(result, Err(Error::<Test>::NotEnoughStaketoWithdraw.into()));
    });
}

#[test]
fn test_remove_stake_total_balance_no_change() {
    // When we remove stake, the total balance of the personalkey account should not change
    //    this is because the stake should be part of the personalkey account balance (reserved/locked)
    //    then the removed stake just becomes free balance
    new_test_ext().execute_with(|| {
        let computekey_account_id = U256::from(571337);
        let personalkey_account_id = U256::from(71337);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;
        let amount = 10000;

        //add network
        add_network(netuid, tempo, 0);

        // Register agent
        register_ok_agent(netuid, computekey_account_id, personalkey_account_id, start_nonce);

        // Some basic assertions
        assert_eq!(BasedNode::get_total_stake(), 0);
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
        assert_eq!(BasedNode::get_personalkey_balance(&personalkey_account_id), 0);
        let initial_total_balance = Balances::total_balance(&personalkey_account_id);
        assert_eq!(initial_total_balance, 0);

        // Give the agent some stake to remove
        BasedNode::increase_stake_on_computekey_account(&computekey_account_id, amount);

        // Do the magic
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            computekey_account_id,
            amount
        ));

        assert_eq!(
            BasedNode::get_personalkey_balance(&personalkey_account_id),
            amount
        );
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
        assert_eq!(BasedNode::get_total_stake(), 0);

        // Check total balance is equal to the added stake. Even after remove stake (no fee, includes reserved/locked balance)
        let total_balance = Balances::total_balance(&personalkey_account_id);
        assert_eq!(total_balance, amount);
    });
}

#[test]
#[ignore]
fn test_remove_stake_total_issuance_no_change() {
    // When we remove stake, the total issuance of the balances pallet should not change
    //    this is because the stake should be part of the personalkey account balance (reserved/locked)
    //    then the removed stake just becomes free balance
    new_test_ext().execute_with(|| {
        let computekey_account_id = U256::from(581337);
        let personalkey_account_id = U256::from(81337);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;
        let amount = 10000;

        //add network
        add_network(netuid, tempo, 0);

        // Register agent
        register_ok_agent(netuid, computekey_account_id, personalkey_account_id, start_nonce);

        // Some basic assertions
        assert_eq!(BasedNode::get_total_stake(), 0);
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
        assert_eq!(BasedNode::get_personalkey_balance(&personalkey_account_id), 0);
        let initial_total_balance = Balances::total_balance(&personalkey_account_id);
        assert_eq!(initial_total_balance, 0);
        let inital_total_issuance = Balances::total_issuance();
        assert_eq!(inital_total_issuance, 0);

        // Give the agent some stake to remove
        BasedNode::increase_stake_on_computekey_account(&computekey_account_id, amount);

        let total_issuance_after_stake = Balances::total_issuance();

        // Do the magic
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            computekey_account_id,
            amount
        ));

        assert_eq!(
            BasedNode::get_personalkey_balance(&personalkey_account_id),
            amount
        );
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_account_id),
            0
        );
        assert_eq!(BasedNode::get_total_stake(), 0);

        // Check if total issuance is equal to the added stake, even after remove stake (no fee, includes reserved/locked balance)
        // Should also be equal to the total issuance after adding stake
        let total_issuance = Balances::total_issuance();
        assert_eq!(total_issuance, total_issuance_after_stake);
        assert_eq!(total_issuance, amount);
    });
}

/***********************************************************
    staking::get_personalkey_balance() tests
************************************************************/
#[test]
fn test_get_personalkey_balance_no_balance() {
    new_test_ext().execute_with(|| {
        let personalkey_account_id = U256::from(5454); // arbitrary
        let result = BasedNode::get_personalkey_balance(&personalkey_account_id);

        // Arbitrary account should have 0 balance
        assert_eq!(result, 0);
    });
}

#[test]
fn test_get_personalkey_balance_with_balance() {
    new_test_ext().execute_with(|| {
        let personalkey_account_id = U256::from(5454); // arbitrary
        let amount = 1337;

        // Put the balance on the account
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, amount);

        let result = BasedNode::get_personalkey_balance(&personalkey_account_id);

        // Arbitrary account should have 0 balance
        assert_eq!(result, amount);
    });
}

// /***********************************************************
// 	staking::add_stake_to_computekey_account() tests
// ************************************************************/
#[test]
fn test_add_stake_to_computekey_account_ok() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(5445);
        let personalkey_id = U256::from(5443433);
        let amount: u64 = 10000;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);

        // There is not stake in the system at first, so result should be 0;
        assert_eq!(BasedNode::get_total_stake(), 0);

        BasedNode::increase_stake_on_computekey_account(&computekey_id, amount);

        // The stake that is now in the account, should equal the amount
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_id),
            amount
        );

        // The total stake should have been increased by the amount -> 0 + amount = amount
        assert_eq!(BasedNode::get_total_stake(), amount);
    });
}

/************************************************************
    staking::remove_stake_from_computekey_account() tests
************************************************************/
#[test]
fn test_remove_stake_from_computekey_account() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(5445);
        let personalkey_id = U256::from(5443433);
        let amount: u64 = 10000;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        //add network
        add_network(netuid, tempo, 0);

        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);

        // Add some stake that can be removed
        BasedNode::increase_stake_on_computekey_account(&computekey_id, amount);

        // Prelimiary checks
        assert_eq!(BasedNode::get_total_stake(), amount);
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_id),
            amount
        );

        // Remove stake
        BasedNode::decrease_stake_on_computekey_account(&computekey_id, amount);

        // The stake on the computekey account should be 0
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey_id), 0);

        // The total amount of stake should be 0
        assert_eq!(BasedNode::get_total_stake(), 0);
    });
}

#[test]
fn test_remove_stake_from_computekey_account_registered_in_various_networks() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(5445);
        let personalkey_id = U256::from(5443433);
        let amount: u64 = 10000;
        let netuid: u16 = 1;
        let netuid_ex = 2;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;
        //
        add_network(netuid, tempo, 0);
        add_network(netuid_ex, tempo, 0);
        //
        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);
        register_ok_agent(netuid_ex, computekey_id, personalkey_id, 48141209);

        //let agent_uid = BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_id);
        let agent_uid;
        match BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_id) {
            Ok(k) => agent_uid = k,
            Err(e) => panic!("Error: {:?}", e),
        }
        //let agent_uid_ex = BasedNode::get_uid_for_net_and_computekey(netuid_ex, &computekey_id);
        let agent_uid_ex;
        match BasedNode::get_uid_for_net_and_computekey(netuid_ex, &computekey_id) {
            Ok(k) => agent_uid_ex = k,
            Err(e) => panic!("Error: {:?}", e),
        }
        //Add some stake that can be removed
        BasedNode::increase_stake_on_computekey_account(&computekey_id, amount);

        assert_eq!(
            BasedNode::get_stake_for_uid_and_brain(netuid, agent_uid),
            amount
        );
        assert_eq!(
            BasedNode::get_stake_for_uid_and_brain(netuid_ex, agent_uid_ex),
            amount
        );

        // Remove stake
        BasedNode::decrease_stake_on_computekey_account(&computekey_id, amount);
        //
        assert_eq!(
            BasedNode::get_stake_for_uid_and_brain(netuid, agent_uid),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_uid_and_brain(netuid_ex, agent_uid_ex),
            0
        );
    });
}

// /************************************************************
// 	staking::increase_total_stake() tests
// ************************************************************/
#[test]
fn test_increase_total_stake_ok() {
    new_test_ext().execute_with(|| {
        let increment = 10000;
        assert_eq!(BasedNode::get_total_stake(), 0);
        BasedNode::increase_total_stake(increment);
        assert_eq!(BasedNode::get_total_stake(), increment);
    });
}

// /************************************************************
// 	staking::decrease_total_stake() tests
// ************************************************************/
#[test]
fn test_decrease_total_stake_ok() {
    new_test_ext().execute_with(|| {
        let initial_total_stake = 10000;
        let decrement = 5000;

        BasedNode::increase_total_stake(initial_total_stake);
        BasedNode::decrease_total_stake(decrement);

        // The total stake remaining should be the difference between the initial stake and the decrement
        assert_eq!(
            BasedNode::get_total_stake(),
            initial_total_stake - decrement
        );
    });
}

// /************************************************************
// 	staking::add_balance_to_personalkey_account() tests
// ************************************************************/
#[test]
fn test_add_balance_to_personalkey_account_ok() {
    new_test_ext().execute_with(|| {
        let personalkey_id = U256::from(4444322);
        let amount = 50000;
        BasedNode::add_balance_to_personalkey_account(&personalkey_id, amount);
        assert_eq!(BasedNode::get_personalkey_balance(&personalkey_id), amount);
    });
}

// /***********************************************************
// 	staking::remove_balance_from_personalkey_account() tests
// ************************************************************/
#[test]
fn test_remove_balance_from_personalkey_account_ok() {
    new_test_ext().execute_with(|| {
        let personalkey_account_id = U256::from(434324); // Random
        let ammount = 10000; // Arbitrary
                             // Put some $$ on the bank
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, ammount);
        assert_eq!(
            BasedNode::get_personalkey_balance(&personalkey_account_id),
            ammount
        );
        // Should be able to withdraw without hassle
        let result =
            BasedNode::remove_balance_from_personalkey_account(&personalkey_account_id, ammount);
        assert_eq!(result, true);
    });
}

#[test]
fn test_remove_balance_from_personalkey_account_failed() {
    new_test_ext().execute_with(|| {
        let personalkey_account_id = U256::from(434324); // Random
        let ammount = 10000; // Arbitrary

        // Try to remove stake from the personalkey account. This should fail,
        // as there is no balance, nor does the account exist
        let result =
            BasedNode::remove_balance_from_personalkey_account(&personalkey_account_id, ammount);
        assert_eq!(result, false);
    });
}

//************************************************************
// 	staking::computekey_belongs_to_personalkey() tests
// ************************************************************/
#[test]
fn test_computekey_belongs_to_personalkey_ok() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(4434334);
        let personalkey_id = U256::from(34333);
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;
        add_network(netuid, tempo, 0);
        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey_id),
            personalkey_id
        );
    });
}
// /************************************************************
// 	staking::can_remove_balance_from_personalkey_account() tests
// ************************************************************/
#[test]
fn test_can_remove_balane_from_personalkey_account_ok() {
    new_test_ext().execute_with(|| {
        let personalkey_id = U256::from(87987984);
        let initial_amount = 10000;
        let remove_amount = 5000;
        BasedNode::add_balance_to_personalkey_account(&personalkey_id, initial_amount);
        assert_eq!(
            BasedNode::can_remove_balance_from_personalkey_account(&personalkey_id, remove_amount),
            true
        );
    });
}

#[test]
fn test_can_remove_balance_from_personalkey_account_err_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let personalkey_id = U256::from(87987984);
        let initial_amount = 10000;
        let remove_amount = 20000;
        BasedNode::add_balance_to_personalkey_account(&personalkey_id, initial_amount);
        assert_eq!(
            BasedNode::can_remove_balance_from_personalkey_account(&personalkey_id, remove_amount),
            false
        );
    });
}
/************************************************************
    staking::has_enough_stake() tests
************************************************************/
#[test]
fn test_has_enough_stake_yes() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(4334);
        let personalkey_id = U256::from(87989);
        let intial_amount = 10000;
        let netuid = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;
        add_network(netuid, tempo, 0);
        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);
        BasedNode::increase_stake_on_computekey_account(&computekey_id, intial_amount);
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_id),
            10000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey_id, &computekey_id),
            10000
        );
        assert_eq!(
            BasedNode::has_enough_stake(&personalkey_id, &computekey_id, 5000),
            true
        );
    });
}

#[test]
fn test_has_enough_stake_no() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(4334);
        let personalkey_id = U256::from(87989);
        let intial_amount = 0;
        let netuid = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;
        add_network(netuid, tempo, 0);
        register_ok_agent(netuid, computekey_id, personalkey_id, start_nonce);
        BasedNode::increase_stake_on_computekey_account(&computekey_id, intial_amount);
        assert_eq!(
            BasedNode::has_enough_stake(&personalkey_id, &computekey_id, 5000),
            false
        );
    });
}

#[test]
fn test_non_existent_account() {
    new_test_ext().execute_with(|| {
        BasedNode::increase_stake_on_personalkey_computekey_account(
            &U256::from(0),
            &(U256::from(0)),
            10,
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&U256::from(0), &U256::from(0)),
            10
        );
        assert_eq!(
            BasedNode::get_total_stake_for_personalkey(&(U256::from(0))),
            10
        );
    });
}

/************************************************************
    staking::delegating
************************************************************/

#[test]
fn test_delegate_stake_division_by_zero_check() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 1;
        let computekey = U256::from(1);
        let personalkey = U256::from(3);
        add_network(netuid, tempo, 0);
        register_ok_agent(netuid, computekey, personalkey, 2341312);
        assert_ok!(BasedNode::become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey),
            computekey
        ));
        BasedNode::emit_inflation_through_computekey_account(&computekey, 0, 1000);
    });
}

#[test]
#[cfg(not(tarpaulin))]
fn test_full_with_delegating() {
    new_test_ext().execute_with(|| {
        let netuid = 1;
        // Make two accounts.
        let computekey0 = U256::from(1);
        let computekey1 = U256::from(2);

        let personalkey0 = U256::from(3);
        let personalkey1 = U256::from(4);
        add_network(netuid, 0, 0);
        BasedNode::set_max_registrations_per_block(netuid, 4);
        BasedNode::set_target_registrations_per_interval(netuid, 4);
        BasedNode::set_max_allowed_uids(netuid, 4); // Allow all 4 to be registered at once

        // Neither key can add stake because they dont have fundss.
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                60000
            ),
            Err(Error::<Test>::NotEnoughBalanceToStake.into())
        );
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey1,
                60000
            ),
            Err(Error::<Test>::NotEnoughBalanceToStake.into())
        );

        // Add balances.
        BasedNode::add_balance_to_personalkey_account(&personalkey0, 60000);
        BasedNode::add_balance_to_personalkey_account(&personalkey1, 60000);

        // We have enough, but the keys are not registered.
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                100
            ),
            Err(Error::<Test>::NotRegistered.into())
        );
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                100
            ),
            Err(Error::<Test>::NotRegistered.into())
        );

        // Cant remove either.
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                10
            ),
            Err(Error::<Test>::NotRegistered.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey1,
                10
            ),
            Err(Error::<Test>::NotRegistered.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey1,
                10
            ),
            Err(Error::<Test>::NotRegistered.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey0,
                10
            ),
            Err(Error::<Test>::NotRegistered.into())
        );

        // Neither key can become a delegate either because we are not registered.
        assert_eq!(
            BasedNode::do_become_delegate(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                100
            ),
            Err(Error::<Test>::NotRegistered.into())
        );
        assert_eq!(
            BasedNode::do_become_delegate(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                100
            ),
            Err(Error::<Test>::NotRegistered.into())
        );

        // Register the 2 agents to a new network.
        register_ok_agent(netuid, computekey0, personalkey0, 124124);
        register_ok_agent(netuid, computekey1, personalkey1, 987907);
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey0),
            personalkey0
        );
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey1),
            personalkey1
        );
        assert!(BasedNode::personalkey_owns_computekey(&personalkey0, &computekey0));
        assert!(BasedNode::personalkey_owns_computekey(&personalkey1, &computekey1));

        // We try to delegate stake but niether are allowing delegation.
        assert!(!BasedNode::computekey_is_delegate(&computekey0));
        assert!(!BasedNode::computekey_is_delegate(&computekey1));
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey1,
                100
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey0,
                100
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );

        // We stake and all is ok.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey0,
            100
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey1,
            100
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            100
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            100
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 100);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 100);
        //assert_eq!( BasedNode::get_total_stake_for_personalkey( &personalkey0 ), 100 );
        //assert_eq!( BasedNode::get_total_stake_for_personalkey( &personalkey1 ), 100 );
        assert_eq!(BasedNode::get_total_stake(), 200);

        // Cant remove these funds because we are not delegating.
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey1,
                10
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey0,
                10
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );

        // Emit inflation through non delegates.
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 0, 100);
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 0, 100);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 200);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 200);

        // Try allowing the keys to become delegates, fails because of incorrect personalkeys.
        // Set take to be 0.
        assert_eq!(
            BasedNode::do_become_delegate(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey1,
                0
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );
        assert_eq!(
            BasedNode::do_become_delegate(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey0,
                0
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );

        // Become delegates all is ok.
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey0,
            10
        ));
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey1,
            10
        ));
        assert!(BasedNode::computekey_is_delegate(&computekey0));
        assert!(BasedNode::computekey_is_delegate(&computekey1));

        // Cant become a delegate twice.
        assert_eq!(
            BasedNode::do_become_delegate(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                1000
            ),
            Err(Error::<Test>::AlreadyDelegate.into())
        );
        assert_eq!(
            BasedNode::do_become_delegate(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey1,
                1000
            ),
            Err(Error::<Test>::AlreadyDelegate.into())
        );

        // This add stake works for delegates.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            200
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            200
        );
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey1,
            200
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey0,
            300
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            200
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            200
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            300
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            200
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 500);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 400);
        //assert_eq!( BasedNode::get_total_stake_for_personalkey( &personalkey0 ), 400 );
        //assert_eq!( BasedNode::get_total_stake_for_personalkey( &personalkey1 ), 500 );
        assert_eq!(BasedNode::get_total_stake(), 900);

        // Lets emit inflation through the hot and personalkeys.
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 0, 1000);
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 0, 1000);
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            601
        ); // 200 + 1000 x ( 200 / 500 ) = 200 + 400 = 600 ~= 601
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            700
        ); // 200 + 1000 x ( 200 / 400 ) = 200 + 500 = 700
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            899
        ); // 300 + 1000 x ( 300 / 500 ) = 300 + 600 = 900 ~= 899
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            700
        ); // 200 + 1000 x ( 200 / 400 ) = 300 + 600 = 700
        assert_eq!(BasedNode::get_total_stake(), 2900); // 600 + 700 + 900 + 700 = 2900

        // // Try unstaking too much.
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                100000
            ),
            Err(Error::<Test>::NotEnoughStaketoWithdraw.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey1,
                100000
            ),
            Err(Error::<Test>::NotEnoughStaketoWithdraw.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey1,
                100000
            ),
            Err(Error::<Test>::NotEnoughStaketoWithdraw.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey0,
                100000
            ),
            Err(Error::<Test>::NotEnoughStaketoWithdraw.into())
        );

        // unstaking is ok.
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey0,
            100
        ));
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey1,
            100
        ));
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey1,
            100
        ));
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey0,
            100
        ));

        // All the amounts have been decreased.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            501
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            600
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            799
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            600
        );

        // Lets register and stake a new key.
        let computekey2 = U256::from(5);
        let personalkey2 = U256::from(6);
        register_ok_agent(netuid, computekey2, personalkey2, 248_123);
        assert!(BasedNode::is_computekey_registered_on_any_network(
            &computekey0
        ));
        assert!(BasedNode::is_computekey_registered_on_any_network(
            &computekey1
        ));

        BasedNode::add_balance_to_personalkey_account(&personalkey2, 60_000);
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            1000
        ));
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            100
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey2),
            900
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey2,
                10
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey2,
                10
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );

        // Lets make this new key a delegate with a 50% take.
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            u16::MAX / 2
        ));

        // Add nominate some stake.
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey2,
            1_000
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey2,
            1_000
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            100
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey2),
            1_000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey2),
            1_000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey2),
            1_000
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey2), 3_000);
        assert_eq!(BasedNode::get_total_stake(), 5_500);

        // Lets emit inflation through this new key with distributed ownership.
        BasedNode::emit_inflation_through_computekey_account(&computekey2, 0, 1000);
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey2),
            1_668
        ); // 1000 + 500 + 500 * (1000/3000) = 1500 + 166.6666666667 = 1,668
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey2),
            1_166
        ); // 1000 + 500 * (1000/3000) = 1000 + 166.6666666667 = 1166.6
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey2),
            1_166
        ); // 1000 + 500 * (1000/3000) = 1000 + 166.6666666667 = 1166.6
        assert_eq!(BasedNode::get_total_stake(), 6_500); // before + 1_000 = 5_500 + 1_000 = 6_500

        step_block(1);

        // Lets register and stake a new key.
        let computekey3 = U256::from(7);
        let personalkey3 = U256::from(8);
        register_ok_agent(netuid, computekey3, personalkey3, 4124124);
        BasedNode::add_balance_to_personalkey_account(&personalkey3, 60000);
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey3),
            computekey3,
            1000
        ));

        step_block(3);

        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey3),
            computekey3,
            u16::MAX
        )); // Full take.
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey3,
            1000
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey3,
            1000
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey3,
            1000
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey3),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey3),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey3),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey3, &computekey3),
            1000
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey3), 4000);
        assert_eq!(BasedNode::get_total_stake(), 10_500);
        BasedNode::emit_inflation_through_computekey_account(&computekey3, 0, 1000);
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey3),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey3),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey3),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey3, &computekey3),
            2000
        );
        assert_eq!(BasedNode::get_total_stake(), 11_500); // before + 1_000 = 10_500 + 1_000 = 11_500
    });
}

// Verify delegates with servers get the full server inflation.
#[test]
fn test_full_with_delegating_some_servers() {
    new_test_ext().execute_with(|| {
        let netuid = 1;
        // Make two accounts.
        let computekey0 = U256::from(1);
        let computekey1 = U256::from(2);

        let personalkey0 = U256::from(3);
        let personalkey1 = U256::from(4);
        BasedNode::set_max_registrations_per_block(netuid, 4);
        BasedNode::set_max_allowed_uids(netuid, 10); // Allow at least 10 to be registered at once, so no unstaking occurs

        // Neither key can add stake because they dont have fundss.
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                60000
            ),
            Err(Error::<Test>::NotEnoughBalanceToStake.into())
        );
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey1,
                60000
            ),
            Err(Error::<Test>::NotEnoughBalanceToStake.into())
        );

        // Add balances.
        BasedNode::add_balance_to_personalkey_account(&personalkey0, 60000);
        BasedNode::add_balance_to_personalkey_account(&personalkey1, 60000);

        // Register the 2 agents to a new network.
        let netuid = 1;
        add_network(netuid, 0, 0);
        register_ok_agent(netuid, computekey0, personalkey0, 124124);
        register_ok_agent(netuid, computekey1, personalkey1, 987907);
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey0),
            personalkey0
        );
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey1),
            personalkey1
        );
        assert!(BasedNode::personalkey_owns_computekey(&personalkey0, &computekey0));
        assert!(BasedNode::personalkey_owns_computekey(&personalkey1, &computekey1));

        // We stake and all is ok.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey0,
            100
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey1,
            100
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            100
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            100
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 100);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 100);
        assert_eq!(BasedNode::get_total_stake(), 200);

        // Emit inflation through non delegates.
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 0, 100);
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 0, 100);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 200);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 200);

        // Become delegates all is ok.
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey0,
            10
        ));
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey1,
            10
        ));
        assert!(BasedNode::computekey_is_delegate(&computekey0));
        assert!(BasedNode::computekey_is_delegate(&computekey1));

        // This add stake works for delegates.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            200
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            200
        );
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey1,
            200
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey0,
            300
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            200
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            200
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            300
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            200
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 500);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 400);
        assert_eq!(BasedNode::get_total_stake(), 900);

        // Lets emit inflation through the hot and personalkeys.
        // fist emission arg is for a server. This should only go to the owner of the computekey.
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 200, 1_000); // 1_200 total emission.
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 123, 2_000); // 2_123 total emission.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            801
        ); // 200 + (200 + 1000 x ( 200 / 500 )) = 200 + (200 + 400) = 800 ~= 801
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            899
        ); // 300 + 1000 x ( 300 / 500 ) = 300 + 600 = 900 ~= 899
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 1_700); // initial + server emission + validator emission = 799 + 899 = 1_698

        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            1_200
        ); // 200 + (0 + 2000 x ( 200 / 400 )) = 200 + (1000) = 1_200
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            1_323
        ); // 200 + (123 + 2000 x ( 200 / 400 )) = 200 + (1_200) = 1_323
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 2_523); // 400 + 2_123
        assert_eq!(BasedNode::get_total_stake(), 4_223); // 1_700 + 2_523 = 4_223

        // Lets emit MORE inflation through the hot and personalkeys.
        // This time only server emission. This should go to the owner of the computekey.
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 350, 0);
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 150, 0);
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            1_151
        ); // + 350 = 1_151
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            1_200
        ); // No change.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            899
        ); // No change.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            1_473
        ); // 1_323 + 150 = 1_473
        assert_eq!(BasedNode::get_total_stake(), 4_723); // 4_223 + 500 = 4_823

        // Lets register and stake a new key.
        let computekey2 = U256::from(5);
        let personalkey2 = U256::from(6);
        register_ok_agent(netuid, computekey2, personalkey2, 248123);
        BasedNode::add_balance_to_personalkey_account(&personalkey2, 60_000);
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            1_000
        ));
        assert_ok!(BasedNode::remove_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            100
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey2),
            900
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey2,
                10
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );
        assert_eq!(
            BasedNode::remove_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey2,
                10
            ),
            Err(Error::<Test>::NonAssociatedpersonalkey.into())
        );

        assert_eq!(BasedNode::get_total_stake(), 5_623); // 4_723 + 900 = 5_623

        // Lets make this new key a delegate with a 50% take.
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            u16::MAX / 2
        ));

        // Add nominate some stake.
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey2,
            1000
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey2,
            1000
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey2),
            computekey2,
            100
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey2),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey2),
            1000
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey2),
            1000
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey2), 3_000);
        assert_eq!(BasedNode::get_total_stake(), 7_723); // 5_623 + (1_000 + 1_000 + 100) = 7_723

        // Lets emit inflation through this new key with distributed ownership.
        // We will emit 100 server emission, which should go in-full to the owner of the computekey.
        // We will emit 1000 validator emission, which should be distributed in-part to the nominators.
        BasedNode::emit_inflation_through_computekey_account(&computekey2, 100, 1000);
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey2),
            1_768
        ); // 1000 + 100 + 500 + 500 * (1000/3000) = 100 + 1500 + 166.6666666667 ~= 1,768.6666666667
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey2),
            1_166
        ); // 1000 + 500 * (1000/3000) = 1000 + 166.6666666667 = 1166.6
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey2),
            1_166
        ); // 1000 + 500 * (1000/3000) = 1000 + 166.6666666667 = 1166.6
        assert_eq!(BasedNode::get_total_stake(), 8_823); // 7_723 + 1_100 = 8_823

        // Lets emit MORE inflation through this new key with distributed ownership.
        // This time we do ONLY server emission
        // We will emit 123 server emission, which should go in-full to the owner of the computekey.
        // We will emit *0* validator emission.
        BasedNode::emit_inflation_through_computekey_account(&computekey2, 123, 0);
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2, &computekey2),
            1_891
        ); // 1_768 + 123 = 1_891
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey2),
            1_166
        ); // No change.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey2),
            1_166
        ); // No change.
        assert_eq!(BasedNode::get_total_stake(), 8_946); // 8_823 + 123 = 8_946
    });
}

#[test]
fn test_full_block_emission_occurs() {
    new_test_ext().execute_with(|| {
        let netuid = 1;
        // Make two accounts.
        let computekey0 = U256::from(1);
        let computekey1 = U256::from(2);

        let personalkey0 = U256::from(3);
        let personalkey1 = U256::from(4);
        BasedNode::set_max_registrations_per_block(netuid, 4);
        BasedNode::set_max_allowed_uids(netuid, 10); // Allow at least 10 to be registered at once, so no unstaking occurs

        // Neither key can add stake because they dont have fundss.
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
                computekey0,
                60000
            ),
            Err(Error::<Test>::NotEnoughBalanceToStake.into())
        );
        assert_eq!(
            BasedNode::add_stake(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
                computekey1,
                60000
            ),
            Err(Error::<Test>::NotEnoughBalanceToStake.into())
        );

        // Add balances.
        BasedNode::add_balance_to_personalkey_account(&personalkey0, 60000);
        BasedNode::add_balance_to_personalkey_account(&personalkey1, 60000);

        // Register the 2 agents to a new network.
        let netuid = 1;
        add_network(netuid, 0, 0);
        register_ok_agent(netuid, computekey0, personalkey0, 124124);
        register_ok_agent(netuid, computekey1, personalkey1, 987907);
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey0),
            personalkey0
        );
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey1),
            personalkey1
        );
        assert!(BasedNode::personalkey_owns_computekey(&personalkey0, &computekey0));
        assert!(BasedNode::personalkey_owns_computekey(&personalkey1, &computekey1));

        // We stake and all is ok.
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            0
        );
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey0,
            100
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey1,
            100
        ));
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey0),
            100
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0, &computekey1),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey0),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1, &computekey1),
            100
        );
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey0), 100);
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey1), 100);
        assert_eq!(BasedNode::get_total_stake(), 200);

        // Emit inflation through non delegates.
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 0, 111);
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 0, 234);
        // Verify the full emission occurs.
        assert_eq!(BasedNode::get_total_stake(), 200 + 111 + 234); // 200 + 111 + 234 = 545

        // Become delegates all is ok.
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey0,
            10
        ));
        assert_ok!(BasedNode::do_become_delegate(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey1,
            10
        ));
        assert!(BasedNode::computekey_is_delegate(&computekey0));
        assert!(BasedNode::computekey_is_delegate(&computekey1));

        // Add some delegate stake
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey0),
            computekey1,
            200
        ));
        assert_ok!(BasedNode::add_stake(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey1),
            computekey0,
            300
        ));

        assert_eq!(BasedNode::get_total_stake(), 545 + 500); // 545 + 500 = 1045

        // Lets emit inflation with delegatees, with both validator and server emission
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 200, 1_000); // 1_200 total emission.
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 123, 2_000); // 2_123 total emission.

        assert_eq!(BasedNode::get_total_stake(), 1045 + 1_200 + 2_123); // before + 1_200 + 2_123 = 4_368

        // Lets emit MORE inflation through the hot and personalkeys.
        // This time JUSt server emission
        BasedNode::emit_inflation_through_computekey_account(&computekey0, 350, 0);
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 150, 0);

        assert_eq!(BasedNode::get_total_stake(), 4_368 + 350 + 150); // before + 350 + 150 = 4_868

        // Lastly, do only validator emission

        BasedNode::emit_inflation_through_computekey_account(&computekey0, 0, 12_948);
        BasedNode::emit_inflation_through_computekey_account(&computekey1, 0, 1_874);

        assert_eq!(BasedNode::get_total_stake(), 4_868 + 12_948 + 1_874); // before + 12_948 + 1_874 = 19_690
    });
}

/************************************************************
    staking::unstake_all_personalkeys_from_computekey_account() tests
************************************************************/

#[test]
fn test_unstake_all_personalkeys_from_computekey_account() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(123570);
        let personalkey0_id = U256::from(123560);

        let personalkey1_id = U256::from(123561);
        let personalkey2_id = U256::from(123562);
        let personalkey3_id = U256::from(123563);

        let amount: u64 = 10000;

        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        // Make brain
        add_network(netuid, tempo, 0);
        // Register delegate
        register_ok_agent(netuid, computekey_id, personalkey0_id, start_nonce);

        match BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_id) {
            Ok(_k) => (),
            Err(e) => panic!("Error: {:?}", e),
        }

        //Add some stake that can be removed
        BasedNode::increase_stake_on_personalkey_computekey_account(&personalkey0_id, &computekey_id, amount);
        BasedNode::increase_stake_on_personalkey_computekey_account(
            &personalkey1_id,
            &computekey_id,
            amount + 2,
        );
        BasedNode::increase_stake_on_personalkey_computekey_account(
            &personalkey2_id,
            &computekey_id,
            amount + 3,
        );
        BasedNode::increase_stake_on_personalkey_computekey_account(
            &personalkey3_id,
            &computekey_id,
            amount + 4,
        );

        // Verify free balance is 0 for all personalkeys
        assert_eq!(Balances::free_balance(personalkey0_id), 0);
        assert_eq!(Balances::free_balance(personalkey1_id), 0);
        assert_eq!(Balances::free_balance(personalkey2_id), 0);
        assert_eq!(Balances::free_balance(personalkey3_id), 0);

        // Verify total stake is correct
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_id),
            amount * 4 + (2 + 3 + 4)
        );

        // Run unstake_all_personalkeys_from_computekey_account
        BasedNode::unstake_all_personalkeys_from_computekey_account(&computekey_id);

        // Verify total stake is 0
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey_id), 0);

        // Vefify stake for all personalkeys is 0
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0_id, &computekey_id),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey1_id, &computekey_id),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey2_id, &computekey_id),
            0
        );
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey3_id, &computekey_id),
            0
        );

        // Verify free balance is correct for all personalkeys
        assert_eq!(Balances::free_balance(personalkey0_id), amount);
        assert_eq!(Balances::free_balance(personalkey1_id), amount + 2);
        assert_eq!(Balances::free_balance(personalkey2_id), amount + 3);
        assert_eq!(Balances::free_balance(personalkey3_id), amount + 4);
    });
}

#[test]
fn test_unstake_all_personalkeys_from_computekey_account_single_staker() {
    new_test_ext().execute_with(|| {
        let computekey_id = U256::from(123570);
        let personalkey0_id = U256::from(123560);

        let amount: u64 = 891011;

        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let start_nonce: u64 = 0;

        // Make brain
        add_network(netuid, tempo, 0);
        // Register delegate
        register_ok_agent(netuid, computekey_id, personalkey0_id, start_nonce);

        match BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_id) {
            Ok(_) => (),
            Err(e) => panic!("Error: {:?}", e),
        }

        //Add some stake that can be removed
        BasedNode::increase_stake_on_personalkey_computekey_account(&personalkey0_id, &computekey_id, amount);

        // Verify free balance is 0 for personalkey
        assert_eq!(Balances::free_balance(personalkey0_id), 0);

        // Verify total stake is correct
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&computekey_id),
            amount
        );

        // Run unstake_all_personalkeys_from_computekey_account
        BasedNode::unstake_all_personalkeys_from_computekey_account(&computekey_id);

        // Verify total stake is 0
        assert_eq!(BasedNode::get_total_stake_for_computekey(&computekey_id), 0);

        // Vefify stake for single personalkey is 0
        assert_eq!(
            BasedNode::get_stake_for_personalkey_and_computekey(&personalkey0_id, &computekey_id),
            0
        );

        // Verify free balance is correct for single personalkey
        assert_eq!(Balances::free_balance(personalkey0_id), amount);
    });
}

#[test]
fn test_faucet_ok() {
    new_test_ext().execute_with(|| {
        let personalkey = U256::from(123560);

        log::info!("Creating work for submission to faucet...");

        let block_number = BasedNode::get_current_block_as_u64();
        let difficulty: U256 = U256::from(10_000_000);
        let mut nonce: u64 = 0;
        let mut work: H256 = BasedNode::create_seal_hash(block_number, nonce, &personalkey);
        while !BasedNode::hash_meets_difficulty(&work, difficulty) {
            nonce = nonce + 1;
            work = BasedNode::create_seal_hash(block_number, nonce, &personalkey);
        }
        let vec_work: Vec<u8> = BasedNode::hash_to_vec(work);

        log::info!("Faucet state: {}", cfg!(feature = "pow-faucet"));

        #[cfg(feature = "pow-faucet")]
        assert_ok!(BasedNode::do_faucet(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey),
            0,
            nonce,
            vec_work
        ));

        #[cfg(not(feature = "pow-faucet"))]
        assert_ok!(
            BasedNode::do_faucet(
                <<Test as Config>::RuntimeOrigin>::signed(personalkey),
                0,
                nonce,
                vec_work
            )
        );
    });
}
