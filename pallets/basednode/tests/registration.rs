use frame_support::traits::Currency;

use crate::mock::*;
use frame_support::{assert_ok, assert_err};
use frame_support::dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, Pays};
use frame_support::sp_runtime::DispatchError;
use frame_system::Config;
use pallet_basednode::{BrainportInfoOf, Error};
use sp_core::U256;

mod mock;

/********************************************
    subscribing::subscribe() tests
*********************************************/

// Tests a basic registration dispatch passes.
#[test]
fn test_registration_subscribe_ok_dispatch_info_ok() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let nonce: u64 = 0;
        let netuid: u16 = 1;
        let work: Vec<u8> = vec![0; 32];
        let computekey: U256 = U256::from(0);
        let personalkey: U256 = U256::from(0);
        let call = RuntimeCall::BasedNode(BasednodeCall::register {
            netuid,
            block_number,
            nonce,
            work,
            computekey,
            personalkey,
        });
        assert_eq!(
            call.get_dispatch_info(),
            DispatchInfo {
                weight: frame_support::weights::Weight::from_ref_time(91000000),
                class: DispatchClass::Normal,
                pays_fee: Pays::No
            }
        );
    });
}

#[test]
fn test_registration_difficulty() {
    new_test_ext().execute_with(|| {
        assert_eq!(BasedNode::get_difficulty(1).as_u64(), 10000);
    });
}

#[test]
fn test_registration_invalid_seal_computekey() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id_1: U256 = U256::from(1);
        let computekey_account_id_2: U256 = U256::from(2);
        let personalkey_account_id: U256 = U256::from(667); // Neighbour of the beast, har har
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id_1,
        );
        let (nonce2, work2): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id_1,
        );

        //add network
        add_network(netuid, tempo, 0);

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id_1),
            netuid,
            block_number,
            nonce,
            work.clone(),
            computekey_account_id_1,
            personalkey_account_id
        ));
        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id_2),
            netuid,
            block_number,
            nonce2,
            work2.clone(),
            computekey_account_id_2,
            personalkey_account_id,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidSeal.into()));
    });
}

#[test]
fn test_registration_ok() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id: U256 = U256::from(1);
        let personalkey_account_id = U256::from(667); // Neighbour of the beast, har har
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            129123813,
            &computekey_account_id,
        );

        //add network
        add_network(netuid, tempo, 0);

        // Subscribe and check extrinsic output
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id
        ));

        // Check if agent has added to the specified network(netuid)
        assert_eq!(BasedNode::get_brain_n(netuid), 1);

        //check if computekey is added to the Computekeys
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey_account_id),
            personalkey_account_id
        );

        // Check if the agent has added to the Keys
        let agent_uid =
            BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).unwrap();

        assert!(BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).is_ok());
        // Check if agent has added to Uids
        let neuro_uid =
            BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).unwrap();
        assert_eq!(neuro_uid, agent_uid);

        // Check if the balance of this computekey account for this brain == 0
        assert_eq!(
            BasedNode::get_stake_for_uid_and_brain(netuid, agent_uid),
            0
        );
    });
}

/********************************************
    registration::do_burned_registration tests
*********************************************/

#[test]
fn test_burned_registration_ok() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let burn_cost = 1000;
        let personalkey_account_id = U256::from(667); // Neighbour of the beast, har har
                                                  //add network
        BasedNode::set_burn(netuid, burn_cost);
        add_network(netuid, tempo, 0);
        // Give it some $$$ in his personalkey balance
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, 10000);
        // Subscribe and check extrinsic output
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            netuid,
            computekey_account_id
        ));
        // Check if balance has  decreased to pay for the burn.
        assert_eq!(
            BasedNode::get_personalkey_balance(&personalkey_account_id) as u64,
            10000 - burn_cost
        ); // funds drained on reg.
           // Check if agent has added to the specified network(netuid)
        assert_eq!(BasedNode::get_brain_n(netuid), 1);
        //check if computekey is added to the Computekeys
        assert_eq!(
            BasedNode::get_owning_personalkey_for_computekey(&computekey_account_id),
            personalkey_account_id
        );
        // Check if the agent has added to the Keys
        let agent_uid =
            BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).unwrap();
        assert!(BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).is_ok());
        // Check if agent has added to Uids
        let neuro_uid =
            BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).unwrap();
        assert_eq!(neuro_uid, agent_uid);
        // Check if the balance of this computekey account for this brain == 0
        assert_eq!(
            BasedNode::get_stake_for_uid_and_brain(netuid, agent_uid),
            0
        );
    });
}

#[test]
fn test_burn_adjustment() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let burn_cost: u64 = 1000;
        let adjustment_interval = 1;
        let target_registrations_per_interval = 1;
        add_network(netuid, tempo, 0);
        BasedNode::set_burn(netuid, burn_cost);
        BasedNode::set_adjustment_interval(netuid, adjustment_interval);
        BasedNode::set_target_registrations_per_interval(
            netuid,
            target_registrations_per_interval,
        );

        // Register key 1.
        let computekey_account_id_1 = U256::from(1);
        let personalkey_account_id_1 = U256::from(1);
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id_1, 10000);
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id_1),
            netuid,
            computekey_account_id_1
        ));

        // Register key 2.
        let computekey_account_id_2 = U256::from(2);
        let personalkey_account_id_2 = U256::from(2);
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id_2, 10000);
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id_2),
            netuid,
            computekey_account_id_2
        ));

        // We are over the number of regs allowed this interval.
        // Step the block and trigger the adjustment.
        step_block(1);

        // Check the adjusted burn.
        assert_eq!(BasedNode::get_burn_as_u64(netuid), 1500);
    });
}

#[test]
#[cfg(not(tarpaulin))]
fn test_registration_too_many_registrations_per_block() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        add_network(netuid, tempo, 0);
        BasedNode::set_max_registrations_per_block(netuid, 10);
        BasedNode::set_target_registrations_per_interval(netuid, 10);
        assert_eq!(BasedNode::get_max_registrations_per_block(netuid), 10);

        let block_number: u64 = 0;
        let (nonce0, work0): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            3942084,
            &U256::from(0),
        );
        let (nonce1, work1): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            11231312312,
            &U256::from(1),
        );
        let (nonce2, work2): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            212312414,
            &U256::from(2),
        );
        let (nonce3, work3): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            21813123,
            &U256::from(3),
        );
        let (nonce4, work4): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            148141209,
            &U256::from(4),
        );
        let (nonce5, work5): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            1245235534,
            &U256::from(5),
        );
        let (nonce6, work6): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            256234,
            &U256::from(6),
        );
        let (nonce7, work7): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            6923424,
            &U256::from(7),
        );
        let (nonce8, work8): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            124242,
            &U256::from(8),
        );
        let (nonce9, work9): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            153453,
            &U256::from(9),
        );
        let (nonce10, work10): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            345923888,
            &U256::from(10),
        );
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 10000);

        // Subscribe and check extrinsic output
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(0)),
            netuid,
            block_number,
            nonce0,
            work0,
            U256::from(0),
            U256::from(0)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 1);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(1)),
            netuid,
            block_number,
            nonce1,
            work1,
            U256::from(1),
            U256::from(1)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 2);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(2)),
            netuid,
            block_number,
            nonce2,
            work2,
            U256::from(2),
            U256::from(2)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 3);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(3)),
            netuid,
            block_number,
            nonce3,
            work3,
            U256::from(3),
            U256::from(3)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 4);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(4)),
            netuid,
            block_number,
            nonce4,
            work4,
            U256::from(4),
            U256::from(4)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 5);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(5)),
            netuid,
            block_number,
            nonce5,
            work5,
            U256::from(5),
            U256::from(5)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 6);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(6)),
            netuid,
            block_number,
            nonce6,
            work6,
            U256::from(6),
            U256::from(6)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 7);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(7)),
            netuid,
            block_number,
            nonce7,
            work7,
            U256::from(7),
            U256::from(7)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 8);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(8)),
            netuid,
            block_number,
            nonce8,
            work8,
            U256::from(8),
            U256::from(8)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 9);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(9)),
            netuid,
            block_number,
            nonce9,
            work9,
            U256::from(9),
            U256::from(9)
        ));
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 10);
        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(10)),
            netuid,
            block_number,
            nonce10,
            work10,
            U256::from(10),
            U256::from(10),
        );
        assert_eq!(
            result,
            Err(Error::<Test>::TooManyRegistrationsThisBlock.into())
        );
    });
}

#[test]
fn test_registration_too_many_registrations_per_interval() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        add_network(netuid, tempo, 0);
        BasedNode::set_max_registrations_per_block(netuid, 11);
        assert_eq!(BasedNode::get_max_registrations_per_block(netuid), 11);
        BasedNode::set_target_registrations_per_interval(netuid, 3);
        assert_eq!(
            BasedNode::get_target_registrations_per_interval(netuid),
            3
        );
        // Then the max is 3 * 3 = 9

        let block_number: u64 = 0;
        let (nonce0, work0): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            3942084,
            &U256::from(0),
        );
        let (nonce1, work1): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            11231312312,
            &U256::from(1),
        );
        let (nonce2, work2): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            212312414,
            &U256::from(2),
        );
        let (nonce3, work3): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            21813123,
            &U256::from(3),
        );
        let (nonce4, work4): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            148141209,
            &U256::from(4),
        );
        let (nonce5, work5): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            1245235534,
            &U256::from(5),
        );
        let (nonce6, work6): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            256234,
            &U256::from(6),
        );
        let (nonce7, work7): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            6923424,
            &U256::from(7),
        );
        let (nonce8, work8): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            124242,
            &U256::from(8),
        );
        let (nonce9, work9): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            153453,
            &U256::from(9),
        );
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 10000);

        // Subscribe and check extrinsic output
        // Try 10 registrations, this is less than the max per block, but more than the max per interval
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(0)),
            netuid,
            block_number,
            nonce0,
            work0,
            U256::from(0),
            U256::from(0)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 1);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(1)),
            netuid,
            block_number,
            nonce1,
            work1,
            U256::from(1),
            U256::from(1)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 2);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(2)),
            netuid,
            block_number,
            nonce2,
            work2,
            U256::from(2),
            U256::from(2)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 3);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(3)),
            netuid,
            block_number,
            nonce3,
            work3,
            U256::from(3),
            U256::from(3)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 4);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(4)),
            netuid,
            block_number,
            nonce4,
            work4,
            U256::from(4),
            U256::from(4)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 5);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(5)),
            netuid,
            block_number,
            nonce5,
            work5,
            U256::from(5),
            U256::from(5)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 6);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(6)),
            netuid,
            block_number,
            nonce6,
            work6,
            U256::from(6),
            U256::from(6)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 7);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(7)),
            netuid,
            block_number,
            nonce7,
            work7,
            U256::from(7),
            U256::from(7)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 8);
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(8)),
            netuid,
            block_number,
            nonce8,
            work8,
            U256::from(8),
            U256::from(8)
        ));
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 9);
        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(9)),
            netuid,
            block_number,
            nonce9,
            work9,
            U256::from(9),
            U256::from(9),
        );
        assert_eq!(
            result,
            Err(Error::<Test>::TooManyRegistrationsThisInterval.into())
        );
    });
}

#[test]
fn test_registration_immunity_period() { //impl this test when epoch impl and calculating pruning score is done
                                         /* TO DO */
}

#[test]
fn test_registration_already_active_computekey() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id,
        );

        //add network
        add_network(netuid, tempo, 0);

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id
        ));

        let block_number: u64 = 0;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id,
        );
        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id,
        );
        assert_eq!(result, Err(Error::<Test>::AlreadyRegistered.into()));
    });
}

#[test]
fn test_registration_invalid_seal() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);
        let (nonce, work): (u64, Vec<u8>) =
            BasedNode::create_work_for_block_number(netuid, 1, 0, &computekey_account_id);

        //add network
        add_network(netuid, tempo, 0);

        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidSeal.into()));
    });
}

#[test]
fn test_registration_invalid_block_number() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 1;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id,
        );

        //add network
        add_network(netuid, tempo, 0);

        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidWorkBlock.into()));
    });
}

#[test]
fn test_registration_invalid_difficulty() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id,
        );

        //add network
        add_network(netuid, tempo, 0);

        BasedNode::set_difficulty(
            netuid,
            18_446_744_073_709_551_615u64
        );

        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id,
        );
        assert_eq!(result, Err(Error::<Test>::InvalidDifficulty.into()));
    });
}

#[test]
fn test_registration_failed_no_signature() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 1;
        let netuid: u16 = 1;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667); // Neighbour of the beast, har har
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id,
        );

        // Subscribe and check extrinsic output
        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::none(),
            netuid,
            block_number,
            nonce,
            work,
            computekey_account_id,
            personalkey_account_id,
        );
        assert_eq!(result, Err(DispatchError::BadOrigin.into()));
    });
}

#[test]
fn test_registration_get_uid_to_prune_all_in_immunity_period() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        add_network(netuid, 0, 0);
        log::info!("add netweork");
        register_ok_agent(netuid, U256::from(0), U256::from(0), 39420842);
        register_ok_agent(netuid, U256::from(1), U256::from(1), 12412392);
        BasedNode::set_pruning_score_for_uid(netuid, 0, 100);
        BasedNode::set_pruning_score_for_uid(netuid, 1, 110);
        BasedNode::set_immunity_period(netuid, 2);
        assert_eq!(BasedNode::get_pruning_score_for_uid(netuid, 0), 100);
        assert_eq!(BasedNode::get_pruning_score_for_uid(netuid, 1), 110);
        assert_eq!(BasedNode::get_immunity_period(netuid), 2);
        assert_eq!(BasedNode::get_current_block_as_u64(), 0);
        assert_eq!(
            BasedNode::get_agent_block_at_registration(netuid, 0),
            0
        );
        assert_eq!(BasedNode::get_agent_to_prune(0), 0);
    });
}

#[test]
fn test_registration_get_uid_to_prune_none_in_immunity_period() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        add_network(netuid, 0, 0);
        log::info!("add netweork");
        register_ok_agent(netuid, U256::from(0), U256::from(0), 39420842);
        register_ok_agent(netuid, U256::from(1), U256::from(1), 12412392);
        BasedNode::set_pruning_score_for_uid(netuid, 0, 100);
        BasedNode::set_pruning_score_for_uid(netuid, 1, 110);
        BasedNode::set_immunity_period(netuid, 2);
        assert_eq!(BasedNode::get_pruning_score_for_uid(netuid, 0), 100);
        assert_eq!(BasedNode::get_pruning_score_for_uid(netuid, 1), 110);
        assert_eq!(BasedNode::get_immunity_period(netuid), 2);
        assert_eq!(BasedNode::get_current_block_as_u64(), 0);
        assert_eq!(
            BasedNode::get_agent_block_at_registration(netuid, 0),
            0
        );
        step_block(3);
        assert_eq!(BasedNode::get_current_block_as_u64(), 3);
        assert_eq!(BasedNode::get_agent_to_prune(0), 0);
    });
}

#[test]
fn test_registration_pruning() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let block_number: u64 = 0;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);
        let (nonce0, work0): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            3942084,
            &computekey_account_id,
        );

        //add network
        add_network(netuid, tempo, 0);

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce0,
            work0,
            computekey_account_id,
            personalkey_account_id
        ));
        //
        let agent_uid =
            BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id).unwrap();
        BasedNode::set_pruning_score_for_uid(netuid, agent_uid, 2);
        //
        let computekey_account_id1 = U256::from(2);
        let personalkey_account_id1 = U256::from(668);
        let (nonce1, work1): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            11231312312,
            &computekey_account_id1,
        );

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id1),
            netuid,
            block_number,
            nonce1,
            work1,
            computekey_account_id1,
            personalkey_account_id1
        ));
        //
        let agent_uid1 =
            BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id1).unwrap();
        BasedNode::set_pruning_score_for_uid(netuid, agent_uid1, 3);
        //
        let computekey_account_id2 = U256::from(3);
        let personalkey_account_id2 = U256::from(669);
        let (nonce2, work2): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            212312414,
            &computekey_account_id2,
        );

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id2),
            netuid,
            block_number,
            nonce2,
            work2,
            computekey_account_id2,
            personalkey_account_id2
        ));
    });
}

#[test]
fn test_registration_get_agent_metadata() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let block_number: u64 = 0;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);
        let (nonce0, work0): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            3942084,
            &computekey_account_id,
        );

        add_network(netuid, tempo, 0);

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce0,
            work0,
            computekey_account_id,
            personalkey_account_id
        ));
        //
        //let agent_id = BasedNode::get_uid_for_net_and_computekey(netuid, &computekey_account_id);
        // let agent_uid = BasedNode::get_uid_for_net_and_computekey( netuid, &computekey_account_id ).unwrap();
        let agent: BrainportInfoOf = BasedNode::get_brainport_info(netuid, &computekey_account_id);
        assert_eq!(agent.ip, 0);
        assert_eq!(agent.version, 0);
        assert_eq!(agent.port, 0);
    });
}

#[test]
fn test_registration_add_network_size() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let netuid2: u16 = 2;
        let block_number: u64 = 0;
        let computekey_account_id = U256::from(1);
        let computekey_account_id1 = U256::from(2);
        let computekey_account_id2 = U256::from(3);
        let (nonce0, work0): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            3942084,
            &computekey_account_id,
        );
        let (nonce1, work1): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid2,
            block_number,
            11231312312,
            &computekey_account_id1,
        );
        let (nonce2, work2): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid2,
            block_number,
            21813123,
            &computekey_account_id2,
        );
        let personalkey_account_id = U256::from(667);

        add_network(netuid, 13, 0);
        assert_eq!(BasedNode::get_brain_n(netuid), 0);

        add_network(netuid2, 13, 0);
        assert_eq!(BasedNode::get_brain_n(netuid2), 0);

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce0,
            work0,
            computekey_account_id,
            personalkey_account_id
        ));
        assert_eq!(BasedNode::get_brain_n(netuid), 1);
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 1);

        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id1),
            netuid2,
            block_number,
            nonce1,
            work1,
            computekey_account_id1,
            personalkey_account_id
        ));
        assert_ok!(BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id2),
            netuid2,
            block_number,
            nonce2,
            work2,
            computekey_account_id2,
            personalkey_account_id
        ));
        assert_eq!(BasedNode::get_brain_n(netuid2), 2);
        assert_eq!(BasedNode::get_registrations_this_interval(netuid2), 2);
    });
}

#[test]
fn test_burn_registration_increase_recycled_rao() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let netuid2: u16 = 2;

        let computekey_account_id = U256::from(1);
        let personalkey_account_id = U256::from(667);

        // Give funds for burn. 1000 BASED
        let _ = Balances::deposit_creating(
            &personalkey_account_id,
            Balance::from(1_000_000_000_000 as u64),
        );

        add_network(netuid, 13, 0);
        assert_eq!(BasedNode::get_brain_n(netuid), 0);

        add_network(netuid2, 13, 0);
        assert_eq!(BasedNode::get_brain_n(netuid2), 0);

        run_to_block(1);

        let burn_amount = BasedNode::get_burn_as_u64(netuid);
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            computekey_account_id
        ));
        assert_eq!(BasedNode::get_rao_recycled(netuid), burn_amount);

        run_to_block(2);

        let burn_amount2 = BasedNode::get_burn_as_u64(netuid2);
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid2,
            computekey_account_id
        ));
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(U256::from(2)),
            netuid2,
            U256::from(2)
        ));
        assert_eq!(BasedNode::get_rao_recycled(netuid2), burn_amount2 * 2);
        // Validate netuid is not affected.
        assert_eq!(BasedNode::get_rao_recycled(netuid), burn_amount);
    });
}

#[test]
fn test_full_pass_through() {
    new_test_ext().execute_with(|| {
        // Create 3 networks.
        let netuid0: u16 = 1;
        let netuid1: u16 = 2;
        let netuid2: u16 = 3;

        // With 3 tempos
        let tempo0: u16 = 2;
        let tempo1: u16 = 2;
        let tempo2: u16 = 2;

        // Create 3 keys.
        let computekey0 = U256::from(0);
        let computekey1 = U256::from(1);
        let computekey2 = U256::from(2);

        // With 3 different personalkeys.
        let personalkey0 = U256::from(0);
        let personalkey1 = U256::from(1);
        let personalkey2 = U256::from(2);

        // Add the 3 networks.
        add_network(netuid0, tempo0, 0);
        add_network(netuid1, tempo1, 0);
        add_network(netuid2, tempo2, 0);

        // Check their tempo.
        assert_eq!(BasedNode::get_tempo(netuid0), tempo0);
        assert_eq!(BasedNode::get_tempo(netuid1), tempo1);
        assert_eq!(BasedNode::get_tempo(netuid2), tempo2);

        // Check their emission value.
        assert_eq!(BasedNode::get_emission_value(netuid0), 0);
        assert_eq!(BasedNode::get_emission_value(netuid1), 0);
        assert_eq!(BasedNode::get_emission_value(netuid2), 0);

        // Set their max allowed uids.
        BasedNode::set_max_allowed_uids(netuid0, 2);
        BasedNode::set_max_allowed_uids(netuid1, 2);
        BasedNode::set_max_allowed_uids(netuid2, 2);

        // Check their max allowed.
        assert_eq!(BasedNode::get_max_allowed_uids(netuid0), 2);
        assert_eq!(BasedNode::get_max_allowed_uids(netuid0), 2);
        assert_eq!(BasedNode::get_max_allowed_uids(netuid0), 2);

        // Set the max registration per block.
        BasedNode::set_max_registrations_per_block(netuid0, 3);
        BasedNode::set_max_registrations_per_block(netuid1, 3);
        BasedNode::set_max_registrations_per_block(netuid2, 3);
        assert_eq!(BasedNode::get_max_registrations_per_block(netuid0), 3);
        assert_eq!(BasedNode::get_max_registrations_per_block(netuid1), 3);
        assert_eq!(BasedNode::get_max_registrations_per_block(netuid2), 3);

        // Check that no one has registered yet.
        assert_eq!(BasedNode::get_brain_n(netuid0), 0);
        assert_eq!(BasedNode::get_brain_n(netuid1), 0);
        assert_eq!(BasedNode::get_brain_n(netuid2), 0);

        // Registered the keys to all networks.
        register_ok_agent(netuid0, computekey0, personalkey0, 39420842);
        register_ok_agent(netuid0, computekey1, personalkey1, 12412392);
        register_ok_agent(netuid1, computekey0, personalkey0, 21813123);
        register_ok_agent(netuid1, computekey1, personalkey1, 25755207);
        register_ok_agent(netuid2, computekey0, personalkey0, 251232207);
        register_ok_agent(netuid2, computekey1, personalkey1, 159184122);

        // Check uids.
        // n0 [ h0, h1 ]
        // n1 [ h0, h1 ]
        // n2 [ h0, h1 ]
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid0, 0).unwrap(),
            computekey0
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid1, 0).unwrap(),
            computekey0
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid2, 0).unwrap(),
            computekey0
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid0, 1).unwrap(),
            computekey1
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid1, 1).unwrap(),
            computekey1
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid2, 1).unwrap(),
            computekey1
        );

        // Check registered networks.
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey0 ).contains( &netuid0 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey0 ).contains( &netuid1 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey0 ).contains( &netuid2 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey1 ).contains( &netuid0 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey1 ).contains( &netuid1 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey1 ).contains( &netuid2 ) );
        // assert!( !BasedNode::get_registered_networks_for_computekey( &computekey2 ).contains( &netuid0 ) );
        // assert!( !BasedNode::get_registered_networks_for_computekey( &computekey2 ).contains( &netuid1 ) );
        // assert!( !BasedNode::get_registered_networks_for_computekey( &computekey2 ).contains( &netuid2 ) );

        // Check the number of registrations.
        assert_eq!(BasedNode::get_registrations_this_interval(netuid0), 2);
        assert_eq!(BasedNode::get_registrations_this_interval(netuid1), 2);
        assert_eq!(BasedNode::get_registrations_this_interval(netuid2), 2);

        // Get the number of uids in each network.
        assert_eq!(BasedNode::get_brain_n(netuid0), 2);
        assert_eq!(BasedNode::get_brain_n(netuid1), 2);
        assert_eq!(BasedNode::get_brain_n(netuid2), 2);

        // Check the uids exist.
        assert!(BasedNode::is_uid_exist_on_network(netuid0, 0));
        assert!(BasedNode::is_uid_exist_on_network(netuid1, 0));
        assert!(BasedNode::is_uid_exist_on_network(netuid2, 0));

        // Check the other exists.
        assert!(BasedNode::is_uid_exist_on_network(netuid0, 1));
        assert!(BasedNode::is_uid_exist_on_network(netuid1, 1));
        assert!(BasedNode::is_uid_exist_on_network(netuid2, 1));

        // Get the computekey under each uid.
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid0, 0).unwrap(),
            computekey0
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid1, 0).unwrap(),
            computekey0
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid2, 0).unwrap(),
            computekey0
        );

        // Get the computekey under the other uid.
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid0, 1).unwrap(),
            computekey1
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid1, 1).unwrap(),
            computekey1
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid2, 1).unwrap(),
            computekey1
        );

        // Check for replacement.
        assert_eq!(BasedNode::get_brain_n(netuid0), 2);
        assert_eq!(BasedNode::get_brain_n(netuid1), 2);
        assert_eq!(BasedNode::get_brain_n(netuid2), 2);

        // Register the 3rd computekey.
        register_ok_agent(netuid0, computekey2, personalkey2, 59420842);
        register_ok_agent(netuid1, computekey2, personalkey2, 31813123);
        register_ok_agent(netuid2, computekey2, personalkey2, 451232207);

        // Check for replacement.
        assert_eq!(BasedNode::get_brain_n(netuid0), 2);
        assert_eq!(BasedNode::get_brain_n(netuid1), 2);
        assert_eq!(BasedNode::get_brain_n(netuid2), 2);

        // Check uids.
        // n0 [ h0, h1 ]
        // n1 [ h0, h1 ]
        // n2 [ h0, h1 ]
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid0, 0).unwrap(),
            computekey2
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid1, 0).unwrap(),
            computekey2
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid2, 0).unwrap(),
            computekey2
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid0, 1).unwrap(),
            computekey1
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid1, 1).unwrap(),
            computekey1
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid2, 1).unwrap(),
            computekey1
        );

        // Check registered networks.
        // computekey0 has been deregistered.
        // assert!( !BasedNode::get_registered_networks_for_computekey( &computekey0 ).contains( &netuid0 ) );
        // assert!( !BasedNode::get_registered_networks_for_computekey( &computekey0 ).contains( &netuid1 ) );
        // assert!( !BasedNode::get_registered_networks_for_computekey( &computekey0 ).contains( &netuid2 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey1 ).contains( &netuid0 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey1 ).contains( &netuid1 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey1 ).contains( &netuid2 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey2 ).contains( &netuid0 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey2 ).contains( &netuid1 ) );
        // assert!( BasedNode::get_registered_networks_for_computekey( &computekey2 ).contains( &netuid2 ) );

        // Check the registration counters.
        assert_eq!(BasedNode::get_registrations_this_interval(netuid0), 3);
        assert_eq!(BasedNode::get_registrations_this_interval(netuid1), 3);
        assert_eq!(BasedNode::get_registrations_this_interval(netuid2), 3);

        // Check the computekeys are expected.
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid0, 0).unwrap(),
            computekey2
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid1, 0).unwrap(),
            computekey2
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid2, 0).unwrap(),
            computekey2
        );
    });
}

// DEPRECATED #[test]
// fn test_network_connection_requirement() {
//     new_test_ext().execute_with(|| {
//         // Add a networks and connection requirements.
//         let netuid_a: u16 = 0;
//         let netuid_b: u16 = 1;
//         add_network(netuid_a, 10, 0);
//         add_network(netuid_b, 10, 0);

//         // Bulk values.
//         let computekeys: Vec<U256> = (0..=10).map(|x| U256::from(x)).collect();
//         let personalkeys: Vec<U256> = (0..=10).map(|x| U256::from(x)).collect();

//         // Add a connection requirement between the A and B. A requires B.
//         BasedNode::add_connection_requirement(netuid_a, netuid_b, u16::MAX);
//         BasedNode::set_max_registrations_per_block(netuid_a, 10); // Enough for the below tests.
//         BasedNode::set_max_registrations_per_block(netuid_b, 10); // Enough for the below tests.
//         BasedNode::set_max_allowed_uids(netuid_a, 10); // Enough for the below tests.
//         BasedNode::set_max_allowed_uids(netuid_b, 10); // Enough for the below tests.

//         // Attempt registration on A fails because the computekey is not registered on network B.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_a, 0, 3942084, &U256::from(0));
//         assert_eq!(
//             BasedNode::register(
//                 <<Test as Config>::RuntimeOrigin>::signed(computekeys[0]),
//                 netuid_a,
//                 0,
//                 nonce,
//                 work,
//                 computekeys[0],
//                 personalkeys[0]
//             ),
//             Err(Error::<Test>::DidNotPassConnectedNetworkRequirement.into())
//         );

//         // Attempt registration on B passes because there is no exterior requirement.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_b, 0, 5942084, &U256::from(0));
//         assert_ok!(BasedNode::register(
//             <<Test as Config>::RuntimeOrigin>::signed(computekeys[0]),
//             netuid_b,
//             0,
//             nonce,
//             work,
//             computekeys[0],
//             personalkeys[0]
//         ));

//         // Attempt registration on A passes because this key is in the top 100 of keys on network B.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_a, 0, 6942084, &U256::from(0));
//         assert_ok!(BasedNode::register(
//             <<Test as Config>::RuntimeOrigin>::signed(computekeys[0]),
//             netuid_a,
//             0,
//             nonce,
//             work,
//             computekeys[0],
//             personalkeys[0]
//         ));

//         // Lets attempt the key registration on A. Fails because we are not in B.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_a, 0, 634242084, &U256::from(1));
//         assert_eq!(
//             BasedNode::register(
//                 <<Test as Config>::RuntimeOrigin>::signed(computekeys[1]),
//                 netuid_a,
//                 0,
//                 nonce,
//                 work,
//                 computekeys[1],
//                 personalkeys[1]
//             ),
//             Err(Error::<Test>::DidNotPassConnectedNetworkRequirement.into())
//         );

//         // Lets register the next key on B. Passes, np.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_b, 0, 7942084, &U256::from(1));
//         assert_ok!(BasedNode::register(
//             <<Test as Config>::RuntimeOrigin>::signed(computekeys[1]),
//             netuid_b,
//             0,
//             nonce,
//             work,
//             computekeys[1],
//             personalkeys[1]
//         ));

//         // Lets make the connection requirement harder. Top 0th percentile.
//         BasedNode::add_connection_requirement(netuid_a, netuid_b, 0);

//         // Attempted registration passes because the prunning score for computekey_1 is the top keys on network B.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_a, 0, 8942084, &U256::from(1));
//         assert_ok!(BasedNode::register(
//             <<Test as Config>::RuntimeOrigin>::signed(computekeys[1]),
//             netuid_a,
//             0,
//             nonce,
//             work,
//             computekeys[1],
//             personalkeys[1]
//         ));

//         // Lets register key 3 with lower prunning score.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_b, 0, 9942084, &U256::from(2));
//         assert_ok!(BasedNode::register(
//             <<Test as Config>::RuntimeOrigin>::signed(computekeys[2]),
//             netuid_b,
//             0,
//             nonce,
//             work,
//             computekeys[2],
//             personalkeys[2]
//         ));
//         BasedNode::set_pruning_score_for_uid(
//             netuid_b,
//             BasedNode::get_uid_for_net_and_computekey(netuid_b, &computekeys[2]).unwrap(),
//             0,
//         ); // Set prunning score to 0.
//         BasedNode::set_pruning_score_for_uid(
//             netuid_b,
//             BasedNode::get_uid_for_net_and_computekey(netuid_b, &computekeys[1]).unwrap(),
//             0,
//         ); // Set prunning score to 0.
//         BasedNode::set_pruning_score_for_uid(
//             netuid_b,
//             BasedNode::get_uid_for_net_and_computekey(netuid_b, &computekeys[0]).unwrap(),
//             0,
//         ); // Set prunning score to 0.

//         // Lets register key 4 with higher prunining score.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_b, 0, 10142084, &U256::from(3));
//         assert_ok!(BasedNode::register(
//             <<Test as Config>::RuntimeOrigin>::signed(computekeys[3]),
//             netuid_b,
//             0,
//             nonce,
//             work,
//             computekeys[3],
//             personalkeys[3]
//         ));
//         BasedNode::set_pruning_score_for_uid(
//             netuid_b,
//             BasedNode::get_uid_for_net_and_computekey(netuid_b, &computekeys[3]).unwrap(),
//             1,
//         ); // Set prunning score to 1.

//         // Attempted register of key 3 fails because of bad prunning score on B.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_a, 0, 11142084, &U256::from(2));
//         assert_eq!(
//             BasedNode::register(
//                 <<Test as Config>::RuntimeOrigin>::signed(computekeys[2]),
//                 netuid_a,
//                 0,
//                 nonce,
//                 work,
//                 computekeys[2],
//                 personalkeys[2]
//             ),
//             Err(Error::<Test>::DidNotPassConnectedNetworkRequirement.into())
//         );

//         // Attempt to register key 4 passes because of best prunning score on B.
//         let (nonce, work): (u64, Vec<u8>) =
//             BasedNode::create_work_for_block_number(netuid_b, 0, 12142084, &U256::from(3));
//         assert_ok!(BasedNode::register(
//             <<Test as Config>::RuntimeOrigin>::signed(computekeys[3]),
//             netuid_a,
//             0,
//             nonce,
//             work,
//             computekeys[3],
//             personalkeys[3]
//         ));
//     });
// }

#[test]
fn test_registration_origin_computekey_mismatch() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id_1: U256 = U256::from(1);
        let computekey_account_id_2: U256 = U256::from(2);
        let personalkey_account_id: U256 = U256::from(668);
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id_1,
        );

        //add network
        add_network(netuid, tempo, 0);

        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id_1),
            netuid,
            block_number,
            nonce,
            work.clone(),
            computekey_account_id_2, // Not the same as the origin.
            personalkey_account_id,
        );
        assert_eq!(result, Err(Error::<Test>::ComputekeyOriginMismatch.into()));
    });
}

#[test]
fn test_registration_disabled() {
    new_test_ext().execute_with(|| {
        let block_number: u64 = 0;
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id: U256 = U256::from(1);
        let personalkey_account_id: U256 = U256::from(668);
        let (nonce, work): (u64, Vec<u8>) = BasedNode::create_work_for_block_number(
            netuid,
            block_number,
            0,
            &computekey_account_id,
        );

        //add network
        add_network(netuid, tempo, 0);
        BasedNode::set_network_registration_allowed(netuid, false);
        BasedNode::set_network_pow_registration_allowed(netuid, false);

        let result = BasedNode::register(
            <<Test as Config>::RuntimeOrigin>::signed(computekey_account_id),
            netuid,
            block_number,
            nonce,
            work.clone(),
            computekey_account_id,
            personalkey_account_id,
        );
        assert_eq!(result, Err(Error::<Test>::RegistrationDisabled.into()));
    });
}

#[ignore]
#[test]
fn test_computekey_swap_ok() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let burn_cost = 1000;
        let personalkey_account_id = U256::from(667);

        BasedNode::set_burn(netuid, burn_cost);
        add_network(netuid, tempo, 0);

        // Give it some $$$ in his personalkey balance
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, 10_000_000_000);

        // Subscribe and check extrinsic output
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            netuid,
            computekey_account_id
        ));

        let new_computekey = U256::from(1337);
        assert_ok!(BasedNode::swap_computekey(<<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id), computekey_account_id, new_computekey));
        assert_ne!(BasedNode::get_owning_personalkey_for_computekey(&computekey_account_id), personalkey_account_id);
        assert_eq!(BasedNode::get_owning_personalkey_for_computekey(&new_computekey), personalkey_account_id);
    });
}

#[ignore]
#[test]
fn test_computekey_swap_not_owner() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let burn_cost = 1000;
        let personalkey_account_id = U256::from(2);
        let not_owner_personalkey = U256::from(3);

        BasedNode::set_burn(netuid, burn_cost);
        add_network(netuid, tempo, 0);

        // Give it some $$$ in his personalkey balance
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, 10000);

        // Subscribe and check extrinsic output
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            netuid,
            computekey_account_id
        ));

        let new_computekey = U256::from(4);
        assert_err!(BasedNode::swap_computekey(<<Test as Config>::RuntimeOrigin>::signed(not_owner_personalkey), computekey_account_id, new_computekey), Error::<Test>::NonAssociatedpersonalkey);
    });
}

#[ignore]
#[test]
fn test_computekey_swap_same_key() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let burn_cost = 1000;
        let personalkey_account_id = U256::from(2);

        BasedNode::set_burn(netuid, burn_cost);
        add_network(netuid, tempo, 0);

        // Give it some $$$ in his personalkey balance
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, 10000);

        // Subscribe and check extrinsic output
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            netuid,
            computekey_account_id
        ));

        assert_err!(BasedNode::swap_computekey(<<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id), computekey_account_id, computekey_account_id), Error::<Test>::AlreadyRegistered);
    });
}

#[ignore]
#[test]
fn test_computekey_swap_registered_key() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 1;
        let tempo: u16 = 13;
        let computekey_account_id = U256::from(1);
        let burn_cost = 1000;
        let personalkey_account_id = U256::from(2);

        BasedNode::set_burn(netuid, burn_cost);
        add_network(netuid, tempo, 0);

        // Give it some $$$ in his personalkey balance
        BasedNode::add_balance_to_personalkey_account(&personalkey_account_id, 100_000_000_000);

        // Subscribe and check extrinsic output
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            netuid,
            computekey_account_id
        ));

        let new_computekey = U256::from(3);
        assert_ok!(BasedNode::burned_register(
            <<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id),
            netuid,
            new_computekey
        ));

        assert_err!(BasedNode::swap_computekey(<<Test as Config>::RuntimeOrigin>::signed(personalkey_account_id), computekey_account_id, new_computekey), Error::<Test>::AlreadyRegistered);
    });
}
