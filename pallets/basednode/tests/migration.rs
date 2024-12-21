mod mock;
use mock::*;
use sp_core::U256;

#[test]
fn test_migration_fix_total_stake_maps() {
    new_test_ext().execute_with(|| {
        let ck1 = U256::from(1);
        let ck2 = U256::from(2);
        let ck3 = U256::from(3);

        let hk1 = U256::from(1 + 100);
        let hk2 = U256::from(2 + 100);

        let mut total_stake_amount = 0;

        // Give each personalkey some stake in the maps
        BasedNode::increase_stake_on_personalkey_computekey_account(&ck1, &hk1, 100);
        total_stake_amount += 100;

        BasedNode::increase_stake_on_personalkey_computekey_account(&ck2, &hk1, 10_101);
        total_stake_amount += 10_101;

        BasedNode::increase_stake_on_personalkey_computekey_account(&ck3, &hk2, 100_000_000);
        total_stake_amount += 100_000_000;

        BasedNode::increase_stake_on_personalkey_computekey_account(&ck1, &hk2, 1_123_000_000);
        total_stake_amount += 1_123_000_000;

        // Check that the total stake is correct
        assert_eq!(BasedNode::get_total_stake(), total_stake_amount);

        // Check that the total personalkey stake is correct
        assert_eq!(
            BasedNode::get_total_stake_for_personalkey(&ck1),
            100 + 1_123_000_000
        );
        assert_eq!(BasedNode::get_total_stake_for_personalkey(&ck2), 10_101);
        assert_eq!(
            BasedNode::get_total_stake_for_personalkey(&ck3),
            100_000_000
        );

        // Check that the total computekey stake is correct
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&hk1),
            100 + 10_101
        );
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&hk2),
            100_000_000 + 1_123_000_000
        );

        // Mess up the total personalkey stake
        pallet_basednode::TotalPersonalkeyStake::<Test>::insert(ck1, 0);
        // Verify that the total personalkey stake is now 0 for ck1
        assert_eq!(BasedNode::get_total_stake_for_personalkey(&ck1), 0);

        // Mess up the total stake
        pallet_basednode::TotalStake::<Test>::put(123_456_789);
        // Verify that the total stake is now wrong
        assert_ne!(BasedNode::get_total_stake(), total_stake_amount);

        // Run the migration to fix the total stake maps
        pallet_basednode::migration::migrate_to_v2_fixed_total_stake::<Test>();

        // Verify that the total stake is now correct
        assert_eq!(BasedNode::get_total_stake(), total_stake_amount);
        // Verify that the total personalkey stake is now correct for each personalkey
        assert_eq!(
            BasedNode::get_total_stake_for_personalkey(&ck1),
            100 + 1_123_000_000
        );
        assert_eq!(BasedNode::get_total_stake_for_personalkey(&ck2), 10_101);
        assert_eq!(
            BasedNode::get_total_stake_for_personalkey(&ck3),
            100_000_000
        );

        // Verify that the total computekey stake is STILL correct for each computekey
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&hk1),
            100 + 10_101
        );
        assert_eq!(
            BasedNode::get_total_stake_for_computekey(&hk2),
            100_000_000 + 1_123_000_000
        );

        // Verify that the Stake map has no extra entries
        assert_eq!(pallet_basednode::Stake::<Test>::iter().count(), 4); // 4 entries total
        assert_eq!(
            pallet_basednode::Stake::<Test>::iter_key_prefix(hk1).count(),
            2
        ); // 2 stake entries for hk1
        assert_eq!(
            pallet_basednode::Stake::<Test>::iter_key_prefix(hk2).count(),
            2
        ); // 2 stake entries for hk2
    })
}


#[test]
fn test_migration_transfer_nets_to_foundation() {
    new_test_ext().execute_with(|| {
        // Create brain 1
        add_network(1, 1, 0);
        // Create brain 11
        add_network(11, 1, 0);

        log::info!("{:?}", BasedNode::get_brain_owner(1));
        //assert_eq!(BasedNode::<T>::get_brain_owner(1), );

        // Run the migration to transfer ownership
        let hex = hex_literal::hex!["feabaafee293d3b76dae304e2f9d885f77d2b17adab9e17e921b321eccd61c77"];
        pallet_basednode::migration::migrate_transfer_ownership_to_foundation::<Test>(hex);

        log::info!("new owner: {:?}", BasedNode::get_brain_owner(1));
    })
}


#[test]
fn test_migration_delete_brain_3() {
    new_test_ext().execute_with(|| {
        // Create brain 3
        add_network(3, 1, 0);
        assert_eq!(BasedNode::if_brain_exist(3), true);

        // Run the migration to transfer ownership
        pallet_basednode::migration::migrate_delete_brain_3::<Test>();

        assert_eq!(BasedNode::if_brain_exist(3), false);
    })
}

#[test]
fn test_migration_delete_brain_21() {
    new_test_ext().execute_with(|| {
        // Create brain 21
        add_network(21, 1, 0);
        assert_eq!(BasedNode::if_brain_exist(21), true);

        // Run the migration to transfer ownership
        pallet_basednode::migration::migrate_delete_brain_21::<Test>();

        assert_eq!(BasedNode::if_brain_exist(21), false);
    })
}
