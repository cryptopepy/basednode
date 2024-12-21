use crate::mock::*;
mod mock;
use sp_core::U256;

#[test]
#[cfg(not(tarpaulin))]
fn test_registration_difficulty_adjustment() {
    new_test_ext().execute_with(|| {
        // Create Net 1
        let netuid: u16 = 1;
        let tempo: u16 = 1;
        let modality: u16 = 1;
        add_network(netuid, tempo, modality);
        BasedNode::set_min_difficulty(netuid, 10000);
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 10000); // Check initial difficulty.
        assert_eq!(BasedNode::get_last_adjustment_block(netuid), 0); // Last adjustment block starts at 0.
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 0); // No registrations this block.
        BasedNode::set_target_registrations_per_interval(netuid, 2);
        BasedNode::set_adjustment_interval(netuid,100);
        assert_eq!(
            BasedNode::get_network_registration_allowed(netuid),
            true
        ); // Default registration allowed.

        // Set values and check.
        BasedNode::set_difficulty(netuid, 20000);
        BasedNode::set_adjustment_interval(netuid, 1);
        BasedNode::set_target_registrations_per_interval(netuid, 1);
        BasedNode::set_max_registrations_per_block(netuid, 3);
        BasedNode::set_max_allowed_uids(netuid, 3);
        BasedNode::set_network_registration_allowed(netuid, true);
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 20000); // Check set difficutly.
        assert_eq!(BasedNode::get_adjustment_interval(netuid), 1); // Check set adjustment interval.
        assert_eq!(
            BasedNode::get_target_registrations_per_interval(netuid),
            1
        ); // Check set adjustment interval.
        assert_eq!(BasedNode::get_max_registrations_per_block(netuid), 3); // Check set registrations per block.
        assert_eq!(BasedNode::get_max_allowed_uids(netuid), 3); // Check set registrations per block.
        assert_eq!(
            BasedNode::get_network_registration_allowed(netuid),
            true
        ); // Check set registration allowed

        // Lets register 3 agents...
        let computekey0 = U256::from(0);
        let computekey1 = U256::from(100);
        let computekey2 = U256::from(2000);
        let personalkey0 = U256::from(0);
        let personalkey1 = U256::from(1000);
        let personalkey2 = U256::from(20000);
        register_ok_agent(netuid, computekey0, personalkey0, 39420842);
        register_ok_agent(netuid, computekey1, personalkey1, 12412392);
        register_ok_agent(netuid, computekey2, personalkey2, 21813123);
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid, 0).unwrap(),
            computekey0
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid, 1).unwrap(),
            computekey1
        );
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid, 2).unwrap(),
            computekey2
        );

        assert_eq!(BasedNode::get_brain_n(netuid), 3); // All 3 are registered.
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 3); // 3 Registrations.
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 3); // 3 Registrations this interval.

        // Fast forward 1 block.
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 20000); // Difficulty is unchanged.
        step_block(1);
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 0); // Registrations have been erased.
        assert_eq!(BasedNode::get_last_adjustment_block(netuid), 1); // We just adjusted on the first block.
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 40000); // Difficulty is increased ( 20000 * ( 3 + 1 ) / ( 1 + 1 ) ) = 80_000
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 0); // Registrations this interval has been wiped.

        // Lets change the adjustment interval
        BasedNode::set_adjustment_interval(netuid, 3);
        assert_eq!(BasedNode::get_adjustment_interval(netuid), 3); // Check set adjustment interval.

        BasedNode::set_target_registrations_per_interval(netuid, 3);
        assert_eq!(
            BasedNode::get_target_registrations_per_interval(netuid),
            3
        ); // Target is default.

        // Register 3 more
        register_ok_agent(netuid, computekey0 + 1, personalkey0 + 1, 3942084);
        register_ok_agent(netuid, computekey1 + 1, personalkey1 + 1, 1241239);
        register_ok_agent(netuid, computekey2 + 1, personalkey2 + 1, 2181312);
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid, 0).unwrap(),
            computekey0 + 1
        ); // replace 0
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid, 1).unwrap(),
            computekey1 + 1
        ); // replace 1
        assert_eq!(
            BasedNode::get_computekey_for_net_and_uid(netuid, 2).unwrap(),
            computekey2 + 1
        ); // replace 2
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 3); // Registrations have been erased.
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 3); // Registrations this interval = 3

        step_block(1); // Step
        assert_eq!(BasedNode::get_last_adjustment_block(netuid), 1); // Still previous adjustment block.
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 0); // Registrations have been erased.
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 3); // Registrations this interval = 3

        // Register 3 more.
        register_ok_agent(netuid, computekey0 + 2, personalkey0 + 2, 394208420);
        register_ok_agent(netuid, computekey1 + 2, personalkey1 + 2, 124123920);
        register_ok_agent(netuid, computekey2 + 2, personalkey2 + 2, 218131230);
        assert_eq!(BasedNode::get_registrations_this_block(netuid), 3); // Registrations have been erased.

        // We have 6 registrations this adjustment interval.
        step_block(1); // Step
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 6); // Registrations this interval = 6
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 40000); // Difficulty unchanged.
        step_block(1); // Step
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 60_000); // Difficulty changed ( 40000 ) * ( 6 + 3 / 3 + 3 ) = 40000 * 1.5 = 60_000
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 0); // Registrations this interval drops to 0.

        // Test min value.
        BasedNode::set_min_difficulty(netuid, 1);
        BasedNode::set_difficulty(netuid, 4);
        assert_eq!(BasedNode::get_min_difficulty(netuid), 1);
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 4);
        BasedNode::set_adjustment_interval(netuid, 1);
        step_block(1); // Step
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 2); // Difficulty dropped 4 * ( 0 + 1 ) / (1 + 1) = 1/2 = 2
        step_block(1); // Step
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 1); // Difficulty dropped 2 * ( 0 + 1 ) / (1 + 1) = 1/2 = 1
        step_block(1); // Step
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 1); // Difficulty dropped 2 * ( 0 + 1 ) / (1 + 1) = 1/2 = max(0.5, 1)

        // Test max value.
        BasedNode::set_max_difficulty(netuid, 10000);
        BasedNode::set_difficulty(netuid, 5000);
        assert_eq!(BasedNode::get_max_difficulty(netuid), 10000);
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 5000);
        BasedNode::set_max_registrations_per_block(netuid, 4);
        register_ok_agent(netuid, computekey0 + 3, personalkey0 + 3, 294208420);
        register_ok_agent(netuid, computekey1 + 3, personalkey1 + 3, 824123920);
        register_ok_agent(netuid, computekey2 + 3, personalkey2 + 3, 324123920);
        register_ok_agent(netuid, computekey2 + 4, personalkey2 + 4, 524123920);
        assert_eq!(BasedNode::get_registrations_this_interval(netuid), 4);
        assert_eq!(
            BasedNode::get_target_registrations_per_interval(netuid),
            3
        );
        step_block(1); // Step
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 5833); // Difficulty increased 5000 * ( 4 + 3 ) / (3 + 3) = 1.16 * 5000 = 5833

        register_ok_agent(netuid, computekey0 + 4, personalkey0 + 4, 124208420);
        register_ok_agent(netuid, computekey1 + 4, personalkey1 + 4, 314123920);
        register_ok_agent(netuid, computekey2 + 4, personalkey2 + 4, 834123920);
        step_block(1); // Step
        assert_eq!(BasedNode::get_difficulty_as_u64(netuid), 5833); // Difficulty unchanged
    });
}
