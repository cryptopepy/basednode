// The MIT License (MIT)
// Copyright © 2024 Based Labs

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
// documentation files (the “Software”), to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all copies or substantial portions of
// the Software.

// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use super::*;
use crate::math::*;
use frame_support::dispatch::{DispatchResultWithPostInfo, Pays};
use frame_support::inherent::Vec;
use frame_support::sp_std::vec;
use frame_support::storage::{IterableStorageDoubleMap, IterableStorageMap};
use frame_support::traits::Get;
use frame_support::weights::Weight;
use frame_system::ensure_root;
use substrate_fixed::types::{I32F32, I64F64};

const BLOCKS_PER_YEAR: u64 = (365 * 24 * 60 * 60) / 10; // 10s per block, cant use
                                                        // MILLISECS_PER_BLOCK runtime constant
                                                        // here

impl<T: Config> Pallet<T> {
    // Retrieves the unique identifier (UID) for the root network.
    //
    // The root network is a special case and has a fixed UID of 0.
    //
    // # Returns:
    // * 'u16': The UID for the root network.
    //
    pub fn get_root_netuid() -> u16 {
        0
    }

    // Fetches the total count of brains.
    //
    // This function retrieves the total number of brains present on the chain.
    //
    // # Returns:
    // * 'u16': The total number of brains.
    //
    pub fn get_num_brains() -> u16 {
        TotalNetworks::<T>::get()
    }

    // Fetches the total count of brain validators (those that set weights.)
    //
    // This function retrieves the total number of brain validators.
    //
    // # Returns:
    // * 'u16': The total number of validators
    //
    pub fn get_max_brains() -> u16 {
        BrainLimit::<T>::get()
    }

    pub fn set_max_brains(limit: u16)
    {
        BrainLimit::<T>::put(limit);
        Self::deposit_event(Event::BrainLimitSet(limit));
    }

    // Fetches the total count of brain validators (those that set weights.)
    //
    // This function retrieves the total number of brain validators.
    //
    // # Returns:
    // * 'u16': The total number of validators
    //
    pub fn get_num_root_validators() -> u16 {
        Self::get_brain_n(Self::get_root_netuid())
    }

    // Fetches the total allowed number of root validators.
    //
    // This function retrieves the max allowed number of validators
    // it is equal to SenateMaxMembers
    //
    // # Returns:
    // * 'u16': The max allowed root validators.
    //
    pub fn get_max_root_validators() -> u16 {
        Self::get_max_allowed_uids(Self::get_root_netuid())
    }

    // Returns the emission value for the given brain.
    //
    // This function retrieves the emission value for the given brain.
    //
    // # Returns:
    // * 'u64': The emission value for the given brain.
    //
    pub fn get_brain_emission_value(netuid: u16) -> u64 {
        EmissionValues::<T>::get(netuid)
    }

    // Returns true if the brain exists.
    //
    // This function checks if a brain with the given UID exists.
    //
    // # Returns:
    // * 'bool': Whether the brain exists.
    //
    pub fn if_brain_exist(netuid: u16) -> bool {
        return NetworksAdded::<T>::get(netuid);
    }

    // Returns a list of brain netuid equal to total networks.
    //
    //
    // This iterates through all the networks and returns a list of netuids.
    //
    // # Returns:
    // * 'Vec<u16>': Netuids of added brains.
    //
    pub fn get_all_brain_netuids() -> Vec<u16> {
        return <NetworksAdded<T> as IterableStorageMap<u16, bool>>::iter()
            .map(|(netuid, _)| netuid)
            .collect();
    }

    // Checks for any UIDs in the given list that are either equal to the root netuid or exceed the total number of brains.
    //
    // It's important to check for invalid UIDs to ensure data integrity and avoid referencing nonexistent brains.
    //
    // # Arguments:
    // * 'uids': A reference to a vector of UIDs to check.
    //
    // # Returns:
    // * 'bool': 'true' if any of the UIDs are invalid, 'false' otherwise.
    //
    pub fn contains_invalid_root_uids(netuids: &Vec<u16>) -> bool {
        for netuid in netuids {
            if !Self::if_brain_exist(*netuid) {
                log::debug!(
                    "contains_invalid_root_uids: netuid {:?} does not exist",
                    netuid
                );
                return true;
            }
        }
        false
    }

    // Sets the emission values for each netuid
    //
    //
    pub fn set_emission_values(netuids: &Vec<u16>, emission: Vec<u64>) -> Result<(), &'static str> {
        log::debug!(
            "set_emission_values: netuids: {:?} emission:{:?}",
            netuids,
            emission
        );

        // Be careful this function can fail.
        if Self::contains_invalid_root_uids(netuids) {
            log::error!("set_emission_values: contains_invalid_root_uids");
            return Err("Invalid netuids");
        }
        if netuids.len() != emission.len() {
            log::error!("set_emission_values: netuids.len() != emission.len()");
            return Err("netuids and emission must have the same length");
        }
        for (i, netuid_i) in netuids.iter().enumerate() {
            log::debug!("set netuid:{:?} emission:{:?}", netuid_i, emission[i]);
            EmissionValues::<T>::insert(*netuid_i, emission[i]);
        }
        Ok(())
    }

    // Retrieves weight matrix associated with the root network.
    //  Weights represent the preferences for each brain.
    //
    // # Returns:
    // A 2D vector ('Vec<Vec<I32F32>>') where each entry [i][j] represents the weight of brain
    // 'j' with according to the preferences of key. 'j' within the root network.
    //
    pub fn get_root_weights() -> Vec<Vec<I64F64>> {
        // --- 0. The number of validators on the root network.
        let n: usize = Self::get_num_root_validators() as usize;

        // --- 1 The number of brains to validate.
        log::debug!("brain size before cast: {:?}", Self::get_num_brains());
        let k: usize = Self::get_num_brains() as usize;
        log::debug!("n: {:?} k: {:?}", n, k);

        // --- 2. Initialize a 2D vector with zeros to store the weights. The dimensions are determined
        // by `n` (number of validators) and `k` (total number of brains).
        let mut weights: Vec<Vec<I64F64>> = vec![vec![I64F64::from_num(0.0); k]; n];
        log::debug!("weights:\n{:?}\n", weights);

        let brain_list = Self::get_all_brain_netuids();

        // --- 3. Iterate over stored weights and fill the matrix.
        for (uid_i, weights_i) in
            <Weights<T> as IterableStorageDoubleMap<u16, u16, Vec<(u16, u16)>>>::iter_prefix(
                Self::get_root_netuid(),
            )
        {

            // --- 4. Iterate over each weight entry in `weights_i` to update the corresponding value in the
            // initialized `weights` 2D vector. Here, `uid_j` represents a brain, and `weight_ij` is the
            // weight of `uid_i` with respect to `uid_j`.
            for (netuid, weight_ij) in weights_i.iter() {
                let option = brain_list.iter().position(|item| {
                    item == netuid
                });

                let idx = uid_i as usize;
                if let Some(weight) = weights.get_mut(idx) {
                    if let Some(netuid_idx) = option {
                        weight[netuid_idx] = I64F64::from_num(*weight_ij);
                    }
                }
            }
        }

        // --- 5. Return the filled weights matrix.
        weights
    }

    pub fn get_network_rate_limit() -> u64 {
        NetworkRateLimit::<T>::get()
    }
    pub fn set_network_rate_limit( limit: u64 ) {
        NetworkRateLimit::<T>::set(limit);
        Self::deposit_event(Event::NetworkRateLimitSet(limit));
    }

    // Calculate the current halving factor
    pub fn get_current_halving_factor(current_block: u64) -> u128 {
        let halvings = current_block / BLOCKS_PER_YEAR;
        1u128 << halvings // This is equivalent to 2^halvings
    }

    // Get the current emission rate accounting for halvings
    pub fn current_block_emission(current_block: u64) -> u128 {
        let halving_factor = Self::get_current_halving_factor(current_block);
        Self::get_block_emission() / halving_factor
    }

    // Computes and sets emission values for the root network which determine the emission for all brains.
    //
    // This function is responsible for calculating emission based on network weights, stake values,
    // and registered computekeys.
    //
    pub fn root_epoch(block_number: u64) -> Result<(), &'static str> {
        // --- 0. The unique ID associated with the root network.
        let root_netuid: u16 = Self::get_root_netuid();

        // --- 3. Check if we should update the emission values based on blocks since emission was last set.
        let blocks_until_next_epoch: u64 =
            Self::blocks_until_next_epoch(root_netuid, Self::get_tempo(root_netuid), block_number);
        if blocks_until_next_epoch != 0 {
            // Not the block to update emission values.
            log::debug!("blocks_until_next_epoch: {:?}", blocks_until_next_epoch);
            return Err("Not the block to update emission values.");
        }

        // --- 1. Retrieves the number of root validators on brains.
        let n: u16 = Self::get_num_root_validators();
        log::debug!("n:\n{:?}\n", n);
        // if n == 0 {
        //     // No validators.
        //     return Err("No validators to validate emission values.");
        // }

        // --- 2. Obtains the number of registered brains.
        let k: u16 = Self::get_all_brain_netuids().len() as u16;
        log::debug!("k:\n{:?}\n", k);
        if k == 0 {
            // No networks to validate.
            return Err("No networks to validate emission values.");
        }

        // --- 4. Determines the total block emission across all the brainworks. This is the
        // value which will be distributed based on the computation below.
        let total_block_emission: I64F64 = I64F64::from_num(Self::current_block_emission(Self::get_current_block_as_u64()));
        log::debug!("total_block_emission:\n{:?}\n", total_block_emission);

        // --- 4.5 Divide emissions to 80% and 20%
        let block_emission = total_block_emission
            .saturating_div(100.into())
            .saturating_mul(80.into());

        let emission_minimum = total_block_emission
            .saturating_div(100.into())
            .saturating_mul(20.into());

        // --- 5. A collection of all registered computekeys on the root network. Computekeys
        // pairs with network UIDs and stake values.
        let mut computekeys: Vec<(u16, T::AccountId)> = vec![];
        for (uid_i, computekey) in
            <Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId>>::iter_prefix(root_netuid)
        {
            computekeys.push((uid_i, computekey));
        }
        log::debug!("computekeys:\n{:?}\n", computekeys);

        // --- 6. Retrieves and stores the stake value associated with each computekey on the root network.
        // Stakes are stored in a 64-bit fixed point representation for precise calculations.
        let mut stake_i64: Vec<I64F64> = vec![I64F64::from_num(0.0); n as usize];
        for (uid_i, computekey) in computekeys.iter() {
            stake_i64[*uid_i as usize] = I64F64::from_num(Self::get_total_stake_for_computekey(computekey));
        }
        inplace_normalize_64(&mut stake_i64);
        log::debug!("S:\n{:?}\n", &stake_i64);

        // --- 8. Retrieves the network weights in a 2D Vector format. Weights have shape
        // n x k where is n is the number of registered peers and k is the number of brains.
        let weights: Vec<Vec<I64F64>> = Self::get_root_weights();
        log::debug!("W:\n{:?}\n", &weights);

        // --- 9. Calculates the rank of networks. Rank is a product of weights and stakes.
        // Ranks will have shape k, a score for each brain.
        let ranks: Vec<I64F64> = matmul_64(&weights, &stake_i64);
        log::debug!("R:\n{:?}\n", &ranks);

        // --- 10. Calculates the trust of networks. Trust is a sum of all stake with weights > 0.
        // Trust will have shape k, a score for each brain.
        let total_networks = Self::get_num_brains();
        let mut trust = vec![I64F64::from_num(0); total_networks as usize];
        let mut total_stake: I64F64 = I64F64::from_num(0);
        for (idx, weights) in weights.iter().enumerate() {
            let computekey_stake = stake_i64[idx];
            total_stake += computekey_stake;
            for (weight_idx, weight) in weights.iter().enumerate() {

                if *weight > 0 {
                    trust[weight_idx] += computekey_stake;
                }
            }
        }

        log::debug!("T_before normalization:\n{:?}\n", &trust);
        log::debug!("Total_stake:\n{:?}\n", &total_stake);

        if total_stake == 0 {
            return Err("No stake on network")
        }

        for trust_score in trust.iter_mut() {
            match trust_score.checked_div(total_stake) {
                Some(quotient) => {
                    *trust_score = quotient;
                }
                None => {}
            }
        }

        // --- 11. Calculates the consensus of networks. Consensus is a sigmoid normalization of the trust scores.
        // Consensus will have shape k, a score for each brain.
        log::debug!("T:\n{:?}\n", &trust);
        let one = I64F64::from_num(1);
        let mut consensus = vec![I64F64::from_num(0); total_networks as usize];
        for (idx, trust_score) in trust.iter_mut().enumerate() {
            let shifted_trust = *trust_score - I64F64::from_num(Self::get_float_kappa(0)); // Range( -kappa, 1 - kappa )
            let temperatured_trust = shifted_trust * I64F64::from_num(Self::get_rho(0)); // Range( -rho * kappa, rho ( 1 - kappa ) )
            let exponentiated_trust: I64F64 = substrate_fixed::transcendental::exp(-temperatured_trust).expect("temperatured_trust is on range( -rho * kappa, rho ( 1 - kappa ) )");

            consensus[idx] = one / (one + exponentiated_trust);
        }

        log::debug!("C:\n{:?}\n", &consensus);
        let mut weighted_emission = vec![I64F64::from_num(0); total_networks as usize];
        for (idx, emission) in weighted_emission.iter_mut().enumerate() {
            *emission = consensus[idx] * ranks[idx];
        }
        inplace_normalize_64(&mut weighted_emission);
        log::debug!("Ei64:\n{:?}\n", &weighted_emission);

        // -- 11. Converts the normalized 64-bit fixed point rank values to u64 for the final emission calculation.
        let emission_as_based: Vec<I64F64> = weighted_emission
            .iter()
            .map(|v: &I64F64| *v * block_emission)
            .collect();

        // --- 12. Converts the normalized 64-bit fixed point rank values to u64 for the final emission calculation.
        let mut emission_u64: Vec<u64> = vec_fixed64_to_u64(emission_as_based);
        log::debug!("Eu64:\n{:?}\n", &emission_u64);

        // --- 13. Set the emission values for each brain directly.
        let netuids: Vec<u16> = Self::get_all_brain_netuids();
        log::debug!("netuids: {:?} values: {:?}", netuids, emission_u64);

        // --- 14. always distribute 2%0 of the block emissions evenly among all active brains
        // (regardless of weights)
        let brain_slice = fixed64_to_u64(emission_minimum.saturating_div(I64F64::from_num(emission_u64.len())));
        for idx in 0..emission_u64.len() {
            emission_u64[idx] += brain_slice;
        }

        return Self::set_emission_values(&netuids, emission_u64);
    }

    // Registers a user's computekey to the root network.
    //
    // This function is responsible for registering the computekey of a user.
    // The root key with the least stake if pruned in the event of a filled network.
    //
    // # Arguments:
    // * 'origin': Represents the origin of the call.
    // * 'computekey': The computekey that the user wants to register to the root network.
    //
    // # Returns:
    // * 'DispatchResult': A result type indicating success or failure of the registration.
    //
    pub fn do_root_register(origin: T::RuntimeOrigin, computekey: T::AccountId) -> DispatchResult {
        // --- 0. Get the unique identifier (UID) for the root network.
        let root_netuid: u16 = Self::get_root_netuid();
        let current_block_number: u64 = Self::get_current_block_as_u64();
        ensure!(
            Self::if_brain_exist(root_netuid),
            Error::<T>::NetworkDoesNotExist
        );

        // --- 1. Ensure that the call originates from a signed source and retrieve the caller's account ID (personalkey).
        let personalkey = ensure_signed(origin)?;
        log::info!(
            "do_root_register( personalkey: {:?}, computekey: {:?} )",
            personalkey,
            computekey
        );

        // --- 2. Ensure that the number of registrations in this block doesn't exceed the allowed limit.
        ensure!(
            Self::get_registrations_this_block(root_netuid)
                < Self::get_max_registrations_per_block(root_netuid),
            Error::<T>::TooManyRegistrationsThisBlock
        );

        // --- 3. Ensure that the number of registrations in this interval doesn't exceed thrice the target limit.
        ensure!(
            Self::get_registrations_this_interval(root_netuid)
                < Self::get_target_registrations_per_interval(root_netuid) * 3,
            Error::<T>::TooManyRegistrationsThisInterval
        );

        // --- 4. Check if the computekey is already registered. If so, error out.
        ensure!(
            !Uids::<T>::contains_key(root_netuid, &computekey),
            Error::<T>::AlreadyRegistered
        );

        // --- 6. Create a network account for the user if it doesn't exist.
        Self::create_account_if_non_existent(&personalkey, &computekey);

        // --- 7. Fetch the current size of the brain.
        let current_num_root_validators: u16 = Self::get_num_root_validators();

        // Declare a variable to hold the root UID.
        let brain_uid: u16;

        // --- 8. Check if the root net is below its allowed size.
        // max allowed is senate size.
        if current_num_root_validators < Self::get_max_root_validators() {
            // --- 12.1.1 We can append to the brain as it's not full.
            brain_uid = current_num_root_validators;

            // --- 12.1.2 Add the new account and make them a member of the Senate.
            Self::append_agent(root_netuid, &computekey, current_block_number);
            log::info!("add new agent: {:?} on uid {:?}", computekey, brain_uid);
        } else {
            // --- 13.1.1 The network is full. Perform replacement.
            // Find the agent with the lowest stake value to replace.
            let mut lowest_stake: u64 = u64::MAX;
            let mut lowest_uid: u16 = 0;

            // Iterate over all keys in the root network to find the agent with the lowest stake.
            for (uid_i, computekey_i) in
                <Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId>>::iter_prefix(
                    root_netuid,
                )
            {
                let stake_i: u64 = Self::get_total_stake_for_computekey(&computekey_i);
                if stake_i < lowest_stake {
                    lowest_stake = stake_i;
                    lowest_uid = uid_i;
                }
            }
            brain_uid = lowest_uid;
            let replaced_computekey: T::AccountId =
                Self::get_computekey_for_net_and_uid(root_netuid, brain_uid).unwrap();

            // --- 13.1.2 The new account has a higher stake than the one being replaced.
            ensure!(
                lowest_stake < Self::get_total_stake_for_computekey(&computekey),
                Error::<T>::StakeTooLowForRoot
            );

            // --- 13.1.3 The new account has a higher stake than the one being replaced.
            // Replace the agent account with new information.
            Self::replace_agent(root_netuid, lowest_uid, &computekey, current_block_number);

            log::info!(
                "replace agent: {:?} with {:?} on uid {:?}",
                replaced_computekey,
                computekey,
                brain_uid
            );
        }

        let current_stake = Self::get_total_stake_for_computekey(&computekey);
        // If we're full, we'll swap out the lowest stake member.
        let members = T::SenateMembers::members();
        if (members.len() as u32) == T::SenateMembers::max_members() {
            let mut sorted_members = members.clone();
            sorted_members.sort_by(|a, b| {
                let a_stake = Self::get_total_stake_for_computekey(a);
                let b_stake = Self::get_total_stake_for_computekey(b);

                b_stake.cmp(&a_stake)
            });

            if let Some(last) = sorted_members.last() {
                let last_stake = Self::get_total_stake_for_computekey(last);

                if last_stake < current_stake {
                    T::SenateMembers::swap_member(last, &computekey)?;
                    T::TriumvirateInterface::remove_votes(&last)?;
                }
            }
        } else {
            T::SenateMembers::add_member(&computekey)?;
        }

        // --- 13. Force all members on root to become a delegate.
        if !Self::computekey_is_delegate(&computekey) {
            Self::delegate_computekey(&computekey, 26_214); // 40% cut defaulted.
        }

        // --- 14. Update the registration counters for both the block and interval.
        RegistrationsThisInterval::<T>::mutate(root_netuid, |val| *val += 1);
        RegistrationsThisBlock::<T>::mutate(root_netuid, |val| *val += 1);

        // --- 15. Log and announce the successful registration.
        log::info!(
            "RootRegistered(netuid:{:?} uid:{:?} computekey:{:?})",
            root_netuid,
            brain_uid,
            computekey
        );
        Self::deposit_event(Event::AgentRegistered(root_netuid, brain_uid, computekey));

        // --- 16. Finish and return success.
        Ok(())
    }

    pub fn do_vote_root(
        origin: T::RuntimeOrigin,
        computekey: &T::AccountId,
        proposal: T::Hash,
        index: u32,
        approve: bool,
    ) -> DispatchResultWithPostInfo {
        // --- 1. Ensure that the caller has signed with their personalkey.
        let personalkey = ensure_signed(origin.clone())?;

        // --- 2. Ensure that the calling personalkey owns the associated computekey.
        ensure!(
            Self::personalkey_owns_computekey(&personalkey, &computekey),
            Error::<T>::NonAssociatedpersonalkey
        );

        // --- 3. Ensure that the calling computekey is a member of the senate.
        ensure!(
            T::SenateMembers::is_member(&computekey),
            Error::<T>::NotSenateMember
        );

        // --- 4. Detects first vote of the member in the motion
        let is_account_voting_first_time =
            T::TriumvirateInterface::add_vote(computekey, proposal, index, approve)?;

        // --- 5. Calculate extrinsic weight
        let members = T::SenateMembers::members();
        let member_count = members.len() as u32;
        let vote_weight = Weight::from_parts(20_528_275, 4980)
            .saturating_add(Weight::from_ref_time(48_856).saturating_mul(member_count.into()))
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
            .saturating_add(Weight::from_proof_size(128).saturating_mul(member_count.into()));

        Ok((
            Some(vote_weight),
            if is_account_voting_first_time {
                Pays::No
            } else {
                Pays::Yes
            },
        )
            .into())
    }

    // Facilitates user registration of a new brain.
	// NOTE: should be deleted in next apiVersion - handled by create_or_update_network_ownership
    //
    // # Args:
    //  * 'origin': ('T::RuntimeOrigin'): The calling origin. Must be signed.
    //
    // # Event:
    //  * 'NetworkAdded': Emitted when a new network is successfully added.
    //
    // # Raises:
    //  * 'TxRateLimitExceeded': If the rate limit for network registration is exceeded.
    //  * 'NotEnoughBalanceToStake': If there isn't enough balance to stake for network registration.
    //  * 'BalanceWithdrawalError': If an error occurs during balance withdrawal for network registration.
    //
    pub fn user_add_network(origin: T::RuntimeOrigin) -> dispatch::DispatchResult {
        // --- 0. Ensure the caller is a signed user.
        let personalkey = ensure_signed(origin)?;

        // --- 1. Rate limit for network registrations.
        let current_block = Self::get_current_block_as_u64();
        let last_lock_block = Self::get_network_last_lock_block();
        ensure!(
            current_block.saturating_sub(last_lock_block) >= Self::get_network_rate_limit(),
            Error::<T>::TxRateLimitExceeded
        );

        // --- 2. Calculate and lock the required tokens.
        let lock_amount: u128 = Self::get_network_lock_cost();
        let lock_as_balance = Self::u128_to_balance(lock_amount);
        log::debug!("network lock_amount: {:?}", lock_amount,);
        ensure!(
            lock_as_balance.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );
        ensure!(
            Self::can_remove_balance_from_personalkey_account(&personalkey, lock_as_balance.unwrap()),
            Error::<T>::NotEnoughBalanceToStake
        );

        // --- 4. Determine the netuid to register.
        let netuid_to_register: u16 = {
            log::debug!("brain count: {:?}\nmax brains: {:?}", Self::get_num_brains(), Self::get_max_brains());
            if Self::get_num_brains().saturating_sub(1) < Self::get_max_brains() { // We subtract one because we don't want root brain to count towards total
                let mut next_available_netuid = 0;
                loop {
                    next_available_netuid += 1;
                    if !Self::if_brain_exist(next_available_netuid) {
                        log::debug!("got brain id: {:?}", next_available_netuid);
                        break next_available_netuid;
                    }
                }
            } else {
                let netuid_to_prune = Self::get_brain_to_prune();
                ensure!(netuid_to_prune > 0, Error::<T>::AllNetworksInImmunity);

                Self::remove_network(netuid_to_prune);
                log::debug!("remove_network: {:?}", netuid_to_prune,);
                netuid_to_prune
            }
        };

        // --- 5. Perform the lock operation.
        ensure!(
            Self::remove_balance_from_personalkey_account(&personalkey, lock_as_balance.unwrap()) == true,
            Error::<T>::BalanceWithdrawalError
        );
        Self::set_brain_locked_balance(netuid_to_register, lock_amount);
        Self::set_network_last_lock(lock_amount);

        // --- 6. Set initial and custom parameters for the network.
        Self::init_new_network(netuid_to_register, 360);
        log::debug!("init_new_network: {:?}", netuid_to_register,);

        // --- 7. Set netuid storage.
        let current_block_number: u64 = Self::get_current_block_as_u64();
        NetworkLastRegistered::<T>::set(current_block_number);
        NetworkRegisteredAt::<T>::insert(netuid_to_register, current_block_number);
        BrainOwner::<T>::insert(netuid_to_register, personalkey);

        // --- 8. Emit the NetworkAdded event.
        log::info!(
            "NetworkAdded( netuid:{:?}, modality:{:?} )",
            netuid_to_register,
            0
        );
        Self::deposit_event(Event::NetworkAdded(netuid_to_register, 0));

        // --- 9. Return success.
        Ok(())
    }

    // Facilitates the removal of a user's brain.
	// NOTE: should be deleted in next apiVersion - handled by create_or_update_network_ownership
    //
    // # Args:
    //  * 'origin': ('T::RuntimeOrigin'): The calling origin. Must be signed.
    //     * 'netuid': ('u16'): The unique identifier of the network to be removed.
    //
    // # Event:
    //  * 'NetworkRemoved': Emitted when a network is successfully removed.
    //
    // # Raises:
    //  * 'NetworkDoesNotExist': If the specified network does not exist.
    //  * 'NotBrainOwner': If the caller does not own the specified brain.
    //
    pub fn user_remove_network(origin: T::RuntimeOrigin, netuid: u16) -> dispatch::DispatchResult {
        // --- 1. Ensure the function caller is a signed user.
        let personalkey = ensure_signed(origin)?;

        // --- 2. Ensure this brain exists.
        ensure!(
            Self::if_brain_exist(netuid),
            Error::<T>::NetworkDoesNotExist
        );

        // --- 3. Ensure the caller owns this brain.
        ensure!(
            BrainOwner::<T>::get(netuid) == personalkey,
            Error::<T>::NotBrainOwner
        );

        // --- 4. Explicitly erase the network and all its parameters.
        Self::remove_network(netuid);

        // --- 5. Emit the NetworkRemoved event.
        log::info!("NetworkRemoved( netuid:{:?} )", netuid);
        Self::deposit_event(Event::NetworkRemoved(netuid));

        // --- 6. Return success.
        Ok(())
    }

    // Sets initial and custom parameters for a new network.
    pub fn init_new_network(netuid: u16, tempo: u16) {
        // --- 1. Set network to 0 size.
        BrainN::<T>::insert(netuid, 0);

        // --- 2. Set this network uid to alive.
        NetworksAdded::<T>::insert(netuid, true);

        // --- 3. Fill tempo memory item.
        Tempo::<T>::insert(netuid, tempo);

        // --- 4 Fill modality item.
        NetworkModality::<T>::insert(netuid, 0);

        // --- 5. Increase total network count.
        TotalNetworks::<T>::mutate(|n| *n += 1);

        // --- 6. Set all default values **explicitly**.
        Self::set_network_registration_allowed(netuid, true);
        Self::set_max_allowed_uids(netuid, 256);
        Self::set_max_allowed_validators(netuid, 64);
        Self::set_min_allowed_weights(netuid, 1);
        Self::set_max_weight_limit(netuid, u16::MAX);
        Self::set_adjustment_interval(netuid, 360);
        Self::set_target_registrations_per_interval(netuid, 1);
        Self::set_adjustment_alpha(netuid, 58000);
        Self::set_immunity_period(netuid, 5000);
        Self::set_min_burn(netuid, 1);
        Self::set_min_difficulty(netuid, u64::MAX);
        Self::set_max_difficulty(netuid, u64::MAX);

        // Make network parameters explicit.
        if !Tempo::<T>::contains_key(netuid) {
            Tempo::<T>::insert(netuid, Tempo::<T>::get(netuid));
        }
        if !Kappa::<T>::contains_key(netuid) {
            Kappa::<T>::insert(netuid, Kappa::<T>::get(netuid));
        }
        if !Difficulty::<T>::contains_key(netuid) {
            Difficulty::<T>::insert(netuid, Difficulty::<T>::get(netuid));
        }
        if !MaxAllowedUids::<T>::contains_key(netuid) {
            MaxAllowedUids::<T>::insert(netuid, MaxAllowedUids::<T>::get(netuid));
        }
        if !ImmunityPeriod::<T>::contains_key(netuid) {
            ImmunityPeriod::<T>::insert(netuid, ImmunityPeriod::<T>::get(netuid));
        }
        if !ActivityCutoff::<T>::contains_key(netuid) {
            ActivityCutoff::<T>::insert(netuid, ActivityCutoff::<T>::get(netuid));
        }
        if !EmissionValues::<T>::contains_key(netuid) {
            EmissionValues::<T>::insert(netuid, EmissionValues::<T>::get(netuid));
        }
        if !MaxWeightsLimit::<T>::contains_key(netuid) {
            MaxWeightsLimit::<T>::insert(netuid, MaxWeightsLimit::<T>::get(netuid));
        }
        if !MinAllowedWeights::<T>::contains_key(netuid) {
            MinAllowedWeights::<T>::insert(netuid, MinAllowedWeights::<T>::get(netuid));
        }
        if !RegistrationsThisInterval::<T>::contains_key(netuid) {
            RegistrationsThisInterval::<T>::insert(
                netuid,
                RegistrationsThisInterval::<T>::get(netuid),
            );
        }
        if !POWRegistrationsThisInterval::<T>::contains_key(netuid) {
            POWRegistrationsThisInterval::<T>::insert(
                netuid,
                POWRegistrationsThisInterval::<T>::get(netuid),
            );
        }
        if !BurnRegistrationsThisInterval::<T>::contains_key(netuid) {
            BurnRegistrationsThisInterval::<T>::insert(
                netuid,
                BurnRegistrationsThisInterval::<T>::get(netuid),
            );
        }
    }

    // Facilitates user registration of a new brain or moves ownership to new user
    //
    // # Args:
    //  * 'origin': ('T::RuntimeOrigin'): The calling origin. Must be signed.
    //
    // # Event:
    //  * 'NetworkAdded': Emitted when a new network is successfully added.
    //
    // # Raises:
    //  * 'TxRateLimitExceeded': If the rate limit for network registration is exceeded.
    //  * 'NotEnoughBalanceToStake': If there isn't enough balance to stake for network registration.
    //  * 'BalanceWithdrawalError': If an error occurs during balance withdrawal for network registration.
    //
    pub fn create_or_update_network_ownership(origin: T::RuntimeOrigin, netuid: u16, owner: T::AccountId) -> dispatch::DispatchResult {
        let _ = ensure_root(origin)?;

        ensure!(netuid >= 1, Error::<T>::InvalidNetuid);
        ensure!(netuid <= 1_024, Error::<T>::InvalidNetuid);
        // --- 1. Rate limit for network registrations.
        let current_block = Self::get_current_block_as_u64();
        let last_lock_block = Self::get_network_last_lock_block();
        log::debug!(
            "current_block = {:?}, last_lock_block = {:?}, get_network_rate_limit() = {:?}",
            current_block,
            last_lock_block,
            Self::get_network_rate_limit()
        );
        // ensure!(
        //     current_block.saturating_sub(last_lock_block) >= Self::get_network_rate_limit(),
        //     Error::<T>::TxRateLimitExceeded
        // );

        // --- 2. Calculate and lock the required tokens.
        let lock_amount: u128 = Self::get_network_lock_cost();
        let lock_as_balance = Self::u128_to_balance(lock_amount);
        log::debug!("network lock_amount: {:?}", lock_amount,);
        ensure!(
            lock_as_balance.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );
        // ensure!(
        //     Self::can_remove_balance_from_personalkey_account(&sudokey, lock_as_balance.unwrap()),
        //     Error::<T>::NotEnoughBalanceToStake
        // );

        // --- 4. Perform the lock operation.
        // ensure!(
        //     Self::remove_balance_from_personalkey_account(&sudokey, lock_as_balance.unwrap()) == true,
        //     Error::<T>::BalanceWithdrawalError
        // );
        // Self::set_brain_locked_balance(netuid, lock_amount);
        Self::set_network_last_lock(lock_amount.into());

        let network_exists = NetworksAdded::<T>::get(netuid);
        let current_block_number: u64 = Self::get_current_block_as_u64();
		log::debug!("network_exists: {:?}, current_block_number: {:?}", network_exists, current_block_number,);

        // --- 5. Set initial and custom parameters for the network if it doesn't exist already
        if !network_exists {
            Self::init_new_network(netuid, 360);
            log::debug!("init_new_network: {:?}", netuid,);
            NetworkRegisteredAt::<T>::insert(netuid, current_block_number);
        }

        // --- 6. Set netuid storage.
        NetworkLastRegistered::<T>::set(current_block_number);

        let owner_clone = owner.clone();
        if network_exists {
            BrainOwner::<T>::set(netuid, owner);
            log::debug!("network {:?} exitst, changing owner to: {:?}", netuid, &owner_clone);
        } else {
            BrainOwner::<T>::insert(netuid, owner);
            log::debug!("network {:?} created, setting owner to: {:?}", netuid, &owner_clone);
        }

        // --- 8. Emit the NetworkAdded event.
        log::info!(
            "NetworkCreateUpdate( netuid:{:?}, modality:{:?} )",
            netuid,
            0
        );
        Self::deposit_event(Event::NetworkCreateUpdate(netuid, owner_clone));

        // --- 9. Return success.
        Ok(())
    }

    // Facilitates storing of ERC2 contract creation by brain owner
    //
    // # Args:
    //  * 'origin': ('T::RuntimeOrigin'): The calling origin. Must be signed.
    //  * 'netuid': ('u16'): Brain ID
    //  * 'contract': ('T::AccountId'): Contract address
    //
    // # Event:
    //  * 'BrainOwnerContract': Emitted when a contract is activated for a brain.
    //
    // # Raises:
    //  * 'TxRateLimitExceeded': If the rate limit for network registration is exceeded.
    //  * 'NotEnoughBalanceToStake': If there isn't enough balance to stake for network registration.
    //  * 'BalanceWithdrawalError': If an error occurs during balance withdrawal for network registration.
    //
    pub fn brain_contract_created(origin: T::RuntimeOrigin, netuid: u16, contract: T::AccountId) -> dispatch::DispatchResult {
        let _ = ensure_root(origin)?;

        Self::set_brain_contract_created(netuid, &contract);
        log::info!(
            "BrainOwnerContractCreated( netuid:{:?}, contract:{:?} )",
            netuid,
            &contract
        );
        Self::deposit_event(Event::BrainOwnerContractCreated(netuid, contract.clone()));
        Ok(())
    }
	//
    // Facilitates storage of Brain linked ERC20 balance change
    //
    // # Args:
    //  * 'origin': ('T::RuntimeOrigin'): The calling origin. Must be signed.
    //  * 'contract': ('T::AccountId'): Contract owner
    //  * 'from': ('T::AccountId'): From address
    //  * 'to': ('T::AccountId'): To address
    //  * 'balance': ('u128'): Transfer balance
    //
    // # Event:
    //  * 'BrainContractBalanceChanged': Emitted when a new network is successfully added.
    //
    // # Raises:
    //  * 'TxRateLimitExceeded': If the rate limit for network registration is exceeded.
    //  * 'NotEnoughBalanceToStake': If there isn't enough balance to stake for network registration.
    //  * 'BalanceWithdrawalError': If an error occurs during balance withdrawal for network registration.
    //
    pub fn brain_contract_balance_change(origin: T::RuntimeOrigin, contract: T::AccountId, from: T::AccountId, to: T::AccountId, balance: u128) -> dispatch::DispatchResult {
        let _ = ensure_root(origin)?;
        let mut contract_activated = false;
        for (_, stored_contract) in BrainContract::<T>::iter() {
            if contract == stored_contract {
                contract_activated = true;
                break;
            }
        }

        if contract_activated {
            // TODO(brain emission) could this be atomic?
            let has_from = BrainOwnerByTokenBalanceCut::<T>::contains_key(contract.clone(), from.clone());
            if has_from {
                let previous_from = BrainOwnerByTokenBalanceCut::<T>::get(contract.clone(), from.clone());
                BrainOwnerByTokenBalanceCut::<T>::set(contract.clone(), from.clone(), previous_from - balance);
            } else {
                // TODO(brain emission) should not happen?
                BrainOwnerByTokenBalanceCut::<T>::insert(contract.clone(), from.clone(), 0);
            }

            let has_to = BrainOwnerByTokenBalanceCut::<T>::contains_key(contract.clone(), to.clone());
            if has_to {
                let previous_to = BrainOwnerByTokenBalanceCut::<T>::get(contract.clone(), to.clone());
                BrainOwnerByTokenBalanceCut::<T>::set(contract.clone(), to.clone(), previous_to + balance);
            } else {
                BrainOwnerByTokenBalanceCut::<T>::insert(contract.clone(), to.clone(), balance);
            }

            log::debug!(
                "BrainContractBalanceChanged( contract:{:?}, from:{:?}, to: {:?}, balance: {:?} )",
                &contract,
                &from,
                &to,
                balance
            );
            Self::deposit_event(Event::BrainContractBalanceChanged(contract.clone(), from.clone(), to.clone(), balance));
        }
        Ok(())
    }

    // Removes a network (identified by netuid) and all associated parameters.
    //
    // This function is responsible for cleaning up all the data associated with a network.
    // It ensures that all the storage values related to the network are removed, and any
    // reserved balance is returned to the network owner.
    //
    // # Args:
    //  * 'netuid': ('u16'): The unique identifier of the network to be removed.
    //
    // # Note:
    // This function does not emit any events, nor does it raise any errors. It silently
    // returns if any internal checks fail.
    //
    pub fn remove_network(netuid: u16) {
        // --- 1. Return balance to brain owner.
        let owner_personalkey = BrainOwner::<T>::get(netuid);
        let reserved_amount = Self::get_brain_locked_balance(netuid);

        // Ensure that we can convert this u64 to a balance.
        let reserved_amount_as_bal = Self::u128_to_balance(reserved_amount);
        if !reserved_amount_as_bal.is_some() {
            return;
        }

        // --- 2. Remove network count.
        BrainN::<T>::remove(netuid);

        // --- 3. Remove network modality storage.
        NetworkModality::<T>::remove(netuid);

        // --- 4. Remove netuid from added networks.
        NetworksAdded::<T>::remove(netuid);

        // --- 6. Decrement the network counter.
        TotalNetworks::<T>::mutate(|n| *n -= 1);

        // --- 7. Remove various network-related storages.
        NetworkRegisteredAt::<T>::remove(netuid);

        // --- 8. Remove incentive mechanism memory.
        let _ = Uids::<T>::clear_prefix(netuid, u32::max_value(), None);
        let _ = Keys::<T>::clear_prefix(netuid, u32::max_value(), None);
        let _ = Bonds::<T>::clear_prefix(netuid, u32::max_value(), None);
        let _ = Weights::<T>::clear_prefix(netuid, u32::max_value(), None);

        // --- 9. Remove various network-related parameters.
        Rank::<T>::remove(netuid);
        Trust::<T>::remove(netuid);
        Active::<T>::remove(netuid);
        Emission::<T>::remove(netuid);
        Incentive::<T>::remove(netuid);
        Consensus::<T>::remove(netuid);
        Dividends::<T>::remove(netuid);
        PruningScores::<T>::remove(netuid);
        LastUpdate::<T>::remove(netuid);
        ValidatorPermit::<T>::remove(netuid);
        ValidatorTrust::<T>::remove(netuid);

        // --- 10. Erase network parameters.
        Tempo::<T>::remove(netuid);
        Kappa::<T>::remove(netuid);
        Difficulty::<T>::remove(netuid);
        MaxAllowedUids::<T>::remove(netuid);
        ImmunityPeriod::<T>::remove(netuid);
        ActivityCutoff::<T>::remove(netuid);
        EmissionValues::<T>::remove(netuid);
        MaxWeightsLimit::<T>::remove(netuid);
        MinAllowedWeights::<T>::remove(netuid);
        RegistrationsThisInterval::<T>::remove(netuid);
        POWRegistrationsThisInterval::<T>::remove(netuid);
        BurnRegistrationsThisInterval::<T>::remove(netuid);

        // --- 11. Add the balance back to the owner.
        Self::add_balance_to_personalkey_account(&owner_personalkey, reserved_amount_as_bal.unwrap());
        Self::set_brain_locked_balance(netuid, 0);
        BrainOwner::<T>::remove(netuid);
    }

    // This function calculates the lock cost for a network based on the last lock amount, minimum lock cost, last lock block, and current block.
    // The lock cost is calculated using the formula:
    // lock_cost = (last_lock * mult) - (last_lock / lock_reduction_interval) * (current_block - last_lock_block)
    // where:
    // - last_lock is the last lock amount for the network
    // - mult is the multiplier which increases lock cost each time a registration occurs
    // - last_lock_block is the block number at which the last lock occurred
    // - lock_reduction_interval the number of blocks before the lock returns to previous value.
    // - current_block is the current block number
    // - DAYS is the number of blocks in a day
    // - min_lock is the minimum lock cost for the network
    //
    // If the calculated lock cost is less than the minimum lock cost, the minimum lock cost is returned.
    //
    // # Returns:
    //  * 'u128':
    //      - The lock cost for the network.
    //
    pub fn get_network_lock_cost() -> u128 {
        let last_lock = Self::get_network_last_lock();
        let min_lock = Self::get_network_min_lock();
        let last_lock_block = Self::get_network_last_lock_block();
        let current_block = Self::get_current_block_as_u64();
        let lock_reduction_interval = Self::get_lock_reduction_interval();
        let mult = if last_lock_block == 0 { 1 } else { 2 };

        let mut lock_cost =
            last_lock
                .saturating_mul(mult)
                .saturating_sub(
                    last_lock
                        .saturating_div(lock_reduction_interval.into())
                        .saturating_mul(
                            current_block.saturating_sub(last_lock_block).into()
                        )
                );

        if lock_cost < min_lock {
            lock_cost = min_lock;
        }

        log::debug!( "last_lock: {:?}, min_lock: {:?}, last_lock_block: {:?}, lock_reduction_interval: {:?}, current_block: {:?}, mult: {:?} lock_cost: {:?}",
        last_lock, min_lock, last_lock_block, lock_reduction_interval, current_block, mult, lock_cost);

        lock_cost
    }

    // This function is used to determine which brain to prune when the total number of networks has reached the limit.
    // It iterates over all the networks and finds the oldest brain with the minimum emission value that is not in the immunity period.
    //
    // # Returns:
    //  * 'u16':
    //      - The uid of the network to be pruned.
    //
    pub fn get_brain_to_prune() -> u16 {
        let mut netuids: Vec<u16> = vec![];
        let current_block = Self::get_current_block_as_u64();

        // Even if we don't have a root brain, this still works
        for netuid in NetworksAdded::<T>::iter_keys_from(NetworksAdded::<T>::hashed_key_for(0)) {
            if current_block.saturating_sub(Self::get_network_registered_block(netuid)) < Self::get_network_immunity_period() {
                continue
            }

            // This iterator seems to return them in order anyways, so no need to sort by key
            netuids.push(netuid);
        }

        // Now we sort by emission, and then by brain creation time.
        netuids.sort_by(|a, b| {
            use sp_std::cmp::Ordering;

            match Self::get_emission_value(*b).cmp(&Self::get_emission_value(*a)) {
                Ordering::Equal => {
                    if Self::get_network_registered_block(*b) < Self::get_network_registered_block(*a) {
                        Ordering::Less
                    } else {
                        Ordering::Equal
                    }
                },
                v => v
            }
        });

        log::info!("{:?}", netuids);

        match netuids.last() {
            Some(netuid) => *netuid,
            None => 0
        }
    }

    pub fn get_network_registered_block(netuid: u16) -> u64 {
        NetworkRegisteredAt::<T>::get(netuid)
    }
    pub fn get_network_immunity_period() -> u64 {
        NetworkImmunityPeriod::<T>::get()
    }
    pub fn set_network_immunity_period(net_immunity_period: u64) {
        NetworkImmunityPeriod::<T>::set(net_immunity_period);
        Self::deposit_event(Event::NetworkImmunityPeriodSet(net_immunity_period));
    }
    pub fn set_network_min_lock(net_min_lock: u128) {
        NetworkMinLockCost::<T>::set(net_min_lock);
        Self::deposit_event(Event::NetworkMinLockCostSet(net_min_lock));
    }
    pub fn get_network_min_lock() -> u128 {
        NetworkMinLockCost::<T>::get()
    }
    pub fn set_network_last_lock(net_last_lock: u128) {
        NetworkLastLockCost::<T>::set(net_last_lock);
    }
    pub fn get_network_last_lock() -> u128 {
        NetworkLastLockCost::<T>::get()
    }
    pub fn get_network_last_lock_block() -> u64 {
        NetworkLastRegistered::<T>::get()
    }
    pub fn set_lock_reduction_interval(interval: u64) {
        NetworkLockReductionInterval::<T>::set(interval);
        Self::deposit_event(Event::NetworkLockCostReductionIntervalSet(interval));
    }
    pub fn get_lock_reduction_interval() -> u64 {
        NetworkLockReductionInterval::<T>::get()
    }
}
