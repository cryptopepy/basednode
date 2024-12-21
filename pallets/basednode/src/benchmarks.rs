//! Basednode pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]
//mod benchmarking;

use crate::Pallet as Basednode;
use crate::*;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::assert_ok;
use frame_support::inherent::Vec;
use frame_support::sp_std::vec;
use frame_system::RawOrigin;
pub use pallet::*;
//use mock::{Test, new_test_ext};

benchmarks! {
  // Add individual benchmarks here
  benchmark_register {
	// Lets create a single network.
	let n: u16 = 10;
	let netuid: u16 = 1; //11 is the benchmark network.
	let tempo: u16 = 1;
	let modality: u16 = 0;
	let seed : u32 = 1;

	let block_number: u64 = Basednode::<T>::get_current_block_as_u64();
	let start_nonce: u64 = (39420842u64 + 100u64*netuid as u64).into();
	let computekey: T::AccountId = account("Alice", 0, seed);
	let (nonce, work): (u64, Vec<u8>) = Basednode::<T>::create_work_for_block_number( netuid, block_number, start_nonce, &computekey);

	Basednode::<T>::init_new_network(netuid, tempo);
	Basednode::<T>::set_network_registration_allowed( netuid.try_into().unwrap(), true.into());

	let block_number: u64 = Basednode::<T>::get_current_block_as_u64();
	let personalkey: T::AccountId = account("Test", 0, seed);
  }: register( RawOrigin::Signed( computekey.clone() ), netuid, block_number, nonce, work, computekey.clone(), personalkey.clone() )

  benchmark_set_weights {

	// This is a whitelisted caller who can make transaction without weights.
	let netuid: u16 = 1;
	let version_key: u64 = 1;
	let tempo: u16 = 1;
	let modality: u16 = 0;

	Basednode::<T>::init_new_network(netuid, tempo);
	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );

	Basednode::<T>::set_network_registration_allowed( netuid.try_into().unwrap(), true.into() );
	Basednode::<T>::set_max_registrations_per_block( netuid.try_into().unwrap(), 4096 );
	Basednode::<T>::set_target_registrations_per_interval( netuid.try_into().unwrap(), 4096 );

	let mut seed : u32 = 1;
	let mut dests: Vec<u16> = vec![];
	let mut weights: Vec<u16> = vec![];
	let signer : T::AccountId = account("Alice", 0, seed);

	for id in 0..4096 as u16 {
	  let computekey: T::AccountId = account("Alice", 0, seed);
	  let personalkey: T::AccountId = account("Test", 0, seed);
	  seed = seed +1;

		Basednode::<T>::set_burn(netuid, 1);
		let amoun_to_be_staked = Basednode::<T>::u64_to_balance( 1000000 );
	  Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), amoun_to_be_staked.unwrap());

	  Basednode::<T>::do_burned_registration(RawOrigin::Signed(personalkey.clone()).into(), netuid, computekey.clone())?;

	  let uid = Basednode::<T>::get_uid_for_net_and_computekey(netuid, &computekey.clone()).unwrap();
	  Basednode::<T>::set_validator_permit_for_uid(netuid, uid.clone(), true);
	  dests.push(id.clone());
	  weights.push(id.clone());
	}

  }: set_weights(RawOrigin::Signed( signer.clone() ), netuid, dests, weights, version_key)


  benchmark_become_delegate {
	// This is a whitelisted caller who can make transaction without weights.
	let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>();
	let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
	let netuid: u16 = 1;
	let version_key: u64 = 1;
	let tempo: u16 = 1;
	let modality: u16 = 0;
	let seed : u32 = 1;

	Basednode::<T>::init_new_network(netuid, tempo);
	  Basednode::<T>::set_burn(netuid, 1);
	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );

	Basednode::<T>::set_network_registration_allowed( netuid.try_into().unwrap(), true.into());
	assert_eq!(Basednode::<T>::get_max_allowed_uids(netuid), 4096);

	let personalkey: T::AccountId = account("Test", 0, seed);
	let computekey: T::AccountId = account("Alice", 0, seed);

	let amoun_to_be_staked = Basednode::<T>::u64_to_balance( 1000000000);
	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), amoun_to_be_staked.unwrap());

	assert_ok!(Basednode::<T>::do_burned_registration(RawOrigin::Signed(personalkey.clone()).into(), netuid, computekey.clone()));
  }: become_delegate(RawOrigin::Signed( personalkey.clone() ), computekey.clone())

  benchmark_add_stake {
	let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>();
	let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
	let netuid: u16 = 1;
	let version_key: u64 = 1;
	let tempo: u16 = 1;
	let modality: u16 = 0;
	let seed : u32 = 1;

	Basednode::<T>::init_new_network(netuid, tempo);

	Basednode::<T>::set_burn(netuid, 1);
	Basednode::<T>::set_network_registration_allowed( netuid.try_into().unwrap(), true.into() );

	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );
	assert_eq!(Basednode::<T>::get_max_allowed_uids(netuid), 4096);

	let personalkey: T::AccountId = account("Test", 0, seed);
	let computekey: T::AccountId = account("Alice", 0, seed);

	let amount: u64 = 1;
	let amoun_to_be_staked = Basednode::<T>::u64_to_balance( 1000000000);
	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), amoun_to_be_staked.unwrap());

	assert_ok!(Basednode::<T>::do_burned_registration(RawOrigin::Signed(personalkey.clone()).into(), netuid, computekey.clone()));
  }: add_stake(RawOrigin::Signed( personalkey.clone() ), computekey, amount)

  benchmark_remove_stake{
	let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>();
	let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
	let netuid: u16 = 1;
	let version_key: u64 = 1;
	let tempo: u16 = 1;
	let modality: u16 = 0;
	let seed : u32 = 1;

	// Set our total stake to 1000 BASED
	Basednode::<T>::increase_total_stake(1_000_000_000_000);

	Basednode::<T>::init_new_network(netuid, tempo);
	Basednode::<T>::set_network_registration_allowed( netuid.try_into().unwrap(), true.into() );

	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );
	assert_eq!(Basednode::<T>::get_max_allowed_uids(netuid), 4096);

	let personalkey: T::AccountId = account("Test", 0, seed);
	let computekey: T::AccountId = account("Alice", 0, seed);
	  Basednode::<T>::set_burn(netuid, 1);

	let wallet_bal = Basednode::<T>::u64_to_balance(1000000);
	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), wallet_bal.unwrap());

	assert_ok!(Basednode::<T>::do_burned_registration(RawOrigin::Signed(personalkey.clone()).into(), netuid, computekey.clone()));
	assert_ok!(Basednode::<T>::do_become_delegate(RawOrigin::Signed(personalkey.clone()).into(), computekey.clone(), Basednode::<T>::get_default_take()));

	  // Stake 10% of our current total staked BASED
	  let u64_staked_amt = 100_000_000_000;
	let amount_to_be_staked = Basednode::<T>::u64_to_balance(u64_staked_amt);
	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), amount_to_be_staked.unwrap());

	assert_ok!( Basednode::<T>::add_stake(RawOrigin::Signed( personalkey.clone() ).into() , computekey.clone(), u64_staked_amt));

	let amount_unstaked: u64 = u64_staked_amt - 1;
  }: remove_stake(RawOrigin::Signed( personalkey.clone() ), computekey.clone(), amount_unstaked)

  benchmark_serve_brainport{
	let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>();
	let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
	let netuid: u16 = 1;
	let tempo: u16 = 1;
	let modality: u16 = 0;

	let version: u32 =  2;
	let ip: u128 = 1676056785;
	let port: u16 = 128;
	let ip_type: u8 = 4;
	let protocol: u8 = 0;
	let placeholder1: u8 = 0;
	let placeholder2: u8 = 0;

	Basednode::<T>::init_new_network(netuid, tempo);
	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );
	assert_eq!(Basednode::<T>::get_max_allowed_uids(netuid), 4096);

	Basednode::<T>::set_burn(netuid, 1);
	let amoun_to_be_staked = Basednode::<T>::u64_to_balance( 1000000 );
	Basednode::<T>::add_balance_to_personalkey_account(&caller.clone(), amoun_to_be_staked.unwrap());

	assert_ok!(Basednode::<T>::do_burned_registration(caller_origin.clone(), netuid, caller.clone()));

	Basednode::<T>::set_serving_rate_limit(netuid, 0);

  }: serve_brainport(RawOrigin::Signed( caller.clone() ), netuid, version, ip, port, ip_type, protocol, placeholder1, placeholder2)

  benchmark_serve_prometheus {
	let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>();
	let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
	let netuid: u16 = 1;
	let tempo: u16 = 1;
	let modality: u16 = 0;

	let version: u32 = 2;
	let ip: u128 = 1676056785;
	let port: u16 = 128;
	let ip_type: u8 = 4;

	Basednode::<T>::init_new_network(netuid, tempo);
	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );
	assert_eq!(Basednode::<T>::get_max_allowed_uids(netuid), 4096);

	Basednode::<T>::set_burn(netuid, 1);
	let amoun_to_be_staked = Basednode::<T>::u64_to_balance( 1000000 );
	Basednode::<T>::add_balance_to_personalkey_account(&caller.clone(), amoun_to_be_staked.unwrap());

	assert_ok!(Basednode::<T>::do_burned_registration(caller_origin.clone(), netuid, caller.clone()));
	Basednode::<T>::set_serving_rate_limit(netuid, 0);

  }: serve_prometheus(RawOrigin::Signed( caller.clone() ), netuid, version, ip, port, ip_type)

  /*
  benchmark_sudo_register {
	let caller: T::AccountId = whitelisted_caller::<AccountIdOf<T>>();
	let caller_origin = <T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone()));
	let netuid: u16 = 1;
	let tempo: u16 = 0;
	let modality: u16 = 0;
	let stake: u64 = 10;
	let balance: u64 = 1000000000;

	Basednode::<T>::init_new_network(netuid, tempo);
	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );
	assert_eq!(Basednode::<T>::get_max_allowed_uids(netuid), 4096);

	let seed : u32 = 1;
	let block_number: u64 = Basednode::<T>::get_current_block_as_u64();
	let computekey: T::AccountId = account("Alice", 0, seed);
	let personalkey: T::AccountId = account("Test", 0, seed);

	let amoun_to_be_staked = Basednode::<T>::u64_to_balance( balance );
	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), amoun_to_be_staked.unwrap());

  }: sudo_register(RawOrigin::<AccountIdOf<T>>::Root, netuid, computekey, personalkey, stake, balance)
  */
  benchmark_burned_register {
	let netuid: u16 = 1;
	let seed : u32 = 1;
	let computekey: T::AccountId = account("Alice", 0, seed);
	let personalkey: T::AccountId = account("Test", 0, seed);
	let modality: u16 = 0;
	let tempo: u16 = 1;

	Basednode::<T>::init_new_network(netuid, tempo);
	Basednode::<T>::set_burn(netuid, 1);

	let amoun_to_be_staked = Basednode::<T>::u64_to_balance( 1000000);
	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), amoun_to_be_staked.unwrap());

  }: burned_register(RawOrigin::Signed( personalkey.clone() ), netuid, computekey)


  benchmark_root_register {
	let netuid: u16 = 1;
	let version_key: u64 = 1;
	let tempo: u16 = 1;
	let seed : u32 = 1;

	Basednode::<T>::init_new_network(netuid, tempo);

	Basednode::<T>::set_burn(netuid, 1);
	Basednode::<T>::set_network_registration_allowed( netuid.try_into().unwrap(), true.into());

	Basednode::<T>::set_max_allowed_uids( netuid, 4096 );
	assert_eq!(Basednode::<T>::get_max_allowed_uids(netuid), 4096);

	let personalkey: T::AccountId = account("Test", 0, seed);
	let computekey: T::AccountId = account("Alice", 0, seed);

	let amount: u64 = 1;
	let amoun_to_be_staked = Basednode::<T>::u64_to_balance( 100_000_000_000_000);
	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), amoun_to_be_staked.unwrap());

	assert_ok!(Basednode::<T>::do_burned_registration(RawOrigin::Signed(personalkey.clone()).into(), netuid, computekey.clone()));
  }: root_register(RawOrigin::Signed(personalkey), computekey)

  swap_computekey {
	let seed: u32 = 1;
	let personalkey: T::AccountId = account("Alice", 0, seed);
	let old_computekey: T::AccountId = account("Bob", 0, seed);
	let new_computekey: T::AccountId = account("Charlie", 0, seed);

	let netuid = 1u16;
	Basednode::<T>::init_new_network(netuid, 100);
	Basednode::<T>::set_min_burn(netuid, 1);
	Basednode::<T>::set_max_burn(netuid, 1);
	Basednode::<T>::set_target_registrations_per_interval(netuid, 256);
	Basednode::<T>::set_max_registrations_per_block(netuid, 256);

	Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), Basednode::<T>::u64_to_balance(10_000_000_000).unwrap());
	assert_ok!(Basednode::<T>::burned_register(RawOrigin::Signed(personalkey.clone()).into(), netuid, old_computekey.clone()));
	assert_ok!(Basednode::<T>::become_delegate(RawOrigin::Signed(personalkey.clone()).into(), old_computekey.clone()));

	let max_uids = Basednode::<T>::get_max_allowed_uids(netuid) as u32;
	for i in 0..max_uids - 1 {
		let personalkey: T::AccountId = account("Brainport", 0, i);
		let computekey: T::AccountId = account("Computekey", 0, i);

		Basednode::<T>::add_balance_to_personalkey_account(&personalkey.clone(), Basednode::<T>::u64_to_balance(10_000_000_000).unwrap());
		assert_ok!(Basednode::<T>::burned_register(RawOrigin::Signed(personalkey.clone()).into(), netuid, computekey));
		assert_ok!(Basednode::<T>::add_stake(RawOrigin::Signed(personalkey).into(), old_computekey.clone(), 1_000_000_000));
	}
  }: _(RawOrigin::Signed(personalkey), old_computekey, new_computekey)
}
