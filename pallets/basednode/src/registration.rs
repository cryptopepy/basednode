use super::*;
use crate::system::ensure_root;
use frame_support::pallet_prelude::{DispatchResult, DispatchResultWithPostInfo};
use frame_system::ensure_signed;
use sp_core::{H256, U256, Get};
use sp_io::hashing::{keccak_256, sha2_256};
use sp_runtime::MultiAddress;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;
use frame_support::storage::IterableStorageDoubleMap;

const LOG_TARGET: &'static str = "runtime::basednode::registration";

impl<T: Config> Pallet<T> {
    // ---- The implementation for the extrinsic do_burned_registration: registering by burning BASED.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling personalkey.
    //             Burned registers can only be created by the personalkey.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'computekey' ( T::AccountId ):
    // 		- Computekey to be registered to the network.
    //
    // # Event:
    // 	* AgentRegistered;
    // 		- On successfully registereing a uid to a agent slot on a brain.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to registed to a non existent network.
    //
    // 	* 'TooManyRegistrationsThisBlock':
    // 		- This registration exceeds the total allowed on this network this block.
    //
    // 	* 'AlreadyRegistered':
    // 		- The computekey is already registered on this network.
    //
    pub fn do_burned_registration(
        origin: T::RuntimeOrigin,
        netuid: u16,
        computekey: T::AccountId,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction. (the personalkey of the pairing)
        let personalkey = ensure_signed(origin)?;
        log::info!(
            "do_registration( personalkey:{:?} netuid:{:?} computekey:{:?} )",
            personalkey,
            netuid,
            computekey
        );

        // --- 2. Ensure the passed network is valid.
        ensure!(
            netuid != Self::get_root_netuid(),
            Error::<T>::OperationNotPermittedonRootBrain
        );
        ensure!(
            Self::if_brain_exist(netuid),
            Error::<T>::NetworkDoesNotExist
        );

        // --- 3. Ensure the passed network allows registrations.
        ensure!(
            Self::get_network_registration_allowed(netuid),
            Error::<T>::RegistrationDisabled
        );

        // --- 4. Ensure we are not exceeding the max allowed registrations per block.
        ensure!(
            Self::get_registrations_this_block(netuid)
                < Self::get_max_registrations_per_block(netuid),
            Error::<T>::TooManyRegistrationsThisBlock
        );

        // --- 4. Ensure we are not exceeding the max allowed registrations per interval.
        ensure!(
            Self::get_registrations_this_interval(netuid)
                < Self::get_target_registrations_per_interval(netuid) * 3,
            Error::<T>::TooManyRegistrationsThisInterval
        );

        // --- 4. Ensure that the key is not already registered.
        ensure!(
            !Uids::<T>::contains_key(netuid, &computekey),
            Error::<T>::AlreadyRegistered
        );

        // DEPRECATED --- 6. Ensure that the key passes the registration requirement
        // ensure!(
        //     Self::passes_network_connection_requirement(netuid, &computekey),
        //     Error::<T>::DidNotPassConnectedNetworkRequirement
        // );

        // --- 7. Ensure the callers personalkey has enough stake to perform the transaction.
        let current_block_number: u64 = Self::get_current_block_as_u64();
        let registration_cost_as_u64 = Self::get_burn_as_u64(netuid);
        let registration_cost_as_balance = Self::u128_to_balance(registration_cost_as_u64).unwrap();
        ensure!(
            Self::can_remove_balance_from_personalkey_account(&personalkey, registration_cost_as_balance),
            Error::<T>::NotEnoughBalanceToStake
        );

        // --- 8. Ensure the remove operation from the personalkey is a success.
        ensure!(
            Self::remove_balance_from_personalkey_account(&personalkey, registration_cost_as_balance)
                == true,
            Error::<T>::BalanceWithdrawalError
        );

        // The burn occurs here.
        Self::burn_tokens(Self::get_burn_as_u64(netuid));

        // --- 9. If the network account does not exist we will create it here.
        Self::create_account_if_non_existent(&personalkey, &computekey);

        // --- 10. Ensure that the pairing is correct.
        ensure!(
            Self::personalkey_owns_computekey(&personalkey, &computekey),
            Error::<T>::NonAssociatedpersonalkey
        );

        // --- 11. Append agent or prune it.
        let brain_uid: u16;
        let current_brain_n: u16 = Self::get_brain_n(netuid);

        // Possibly there is no agent slots at all.
        ensure!(
            Self::get_max_allowed_uids(netuid) != 0,
            Error::<T>::NetworkDoesNotExist
        );

        if current_brain_n < Self::get_max_allowed_uids(netuid) {
            // --- 12.1.1 No replacement required, the uid appends the brain.
            // We increment the brain count here but not below.
            brain_uid = current_brain_n;

            // --- 12.1.2 Expand brain with new account.
            Self::append_agent(netuid, &computekey, current_block_number);
            log::info!("add new agent account");
        } else {
            // --- 13.1.1 Replacement required.
            // We take the agent with the lowest pruning score here.
            brain_uid = Self::get_agent_to_prune(netuid);

            // --- 13.1.1 Replace the agent account with the new info.
            Self::replace_agent(netuid, brain_uid, &computekey, current_block_number);
            log::info!("prune agent");
        }

        // --- 14. Record the registration and increment block and interval counters.
        BurnRegistrationsThisInterval::<T>::mutate(netuid, |val| *val += 1);
        RegistrationsThisInterval::<T>::mutate(netuid, |val| *val += 1);
        RegistrationsThisBlock::<T>::mutate(netuid, |val| *val += 1);
        Self::increase_rao_recycled(netuid, Self::get_burn_as_u64(netuid));

        // --- 15. Deposit successful event.
        log::info!(
            "AgentRegistered( netuid:{:?} uid:{:?} computekey:{:?}  ) ",
            netuid,
            brain_uid,
            computekey
        );
        Self::deposit_event(Event::AgentRegistered(netuid, brain_uid, computekey));

        // --- 16. Ok and done.
        Ok(())
    }

    // ---- The implementation for the extrinsic do_registration.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling computekey.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'block_number' ( u64 ):
    // 		- Block hash used to prove work done.
    //
    // 	* 'nonce' ( u64 ):
    // 		- Positive integer nonce used in POW.
    //
    // 	* 'work' ( Vec<u8> ):
    // 		- Vector encoded bytes representing work done.
    //
    // 	* 'computekey' ( T::AccountId ):
    // 		- Computekey to be registered to the network.
    //
    // 	* 'personalkey' ( T::AccountId ):
    // 		- Associated personalkey account.
    //
    // # Event:
    // 	* AgentRegistered;
    // 		- On successfully registereing a uid to a agent slot on a brain.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to registed to a non existent network.
    //
    // 	* 'TooManyRegistrationsThisBlock':
    // 		- This registration exceeds the total allowed on this network this block.
    //
    // 	* 'AlreadyRegistered':
    // 		- The computekey is already registered on this network.
    //
    // 	* 'InvalidWorkBlock':
    // 		- The work has been performed on a stale, future, or non existent block.
    //
    // 	* 'InvalidDifficulty':
    // 		- The work does not match the difficutly.
    //
    // 	* 'InvalidSeal':
    // 		- The seal is incorrect.
    //
    pub fn do_registration(
        origin: T::RuntimeOrigin,
        netuid: u16,
        block_number: u64,
        nonce: u64,
        work: Vec<u8>,
        computekey: T::AccountId,
        personalkey: T::AccountId,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        // TODO( const ): This not be the computekey signature or else an exterior actor can register the computekey and potentially control it?
        let signing_origin = ensure_signed(origin)?;
        log::info!(
            "do_registration( origin:{:?} netuid:{:?} computekey:{:?}, personalkey:{:?} )",
            signing_origin,
            netuid,
            computekey,
            personalkey
        );

        ensure!(signing_origin == computekey, Error::<T>::ComputekeyOriginMismatch);

        // --- 2. Ensure the passed network is valid.
        ensure!(
            netuid != Self::get_root_netuid(),
            Error::<T>::OperationNotPermittedonRootBrain
        );
        ensure!(
            Self::if_brain_exist(netuid),
            Error::<T>::NetworkDoesNotExist
        );

        // --- 3. Ensure the passed network allows registrations.
        ensure!(
            Self::get_network_pow_registration_allowed(netuid),
            Error::<T>::RegistrationDisabled
        );

        // --- 4. Ensure we are not exceeding the max allowed registrations per block.
        ensure!(
            Self::get_registrations_this_block(netuid)
                < Self::get_max_registrations_per_block(netuid),
            Error::<T>::TooManyRegistrationsThisBlock
        );

        // --- 5. Ensure we are not exceeding the max allowed registrations per interval.
        ensure!(
            Self::get_registrations_this_interval(netuid)
                < Self::get_target_registrations_per_interval(netuid) * 3,
            Error::<T>::TooManyRegistrationsThisInterval
        );

        // --- 6. Ensure that the key is not already registered.
        ensure!(
            !Uids::<T>::contains_key(netuid, &computekey),
            Error::<T>::AlreadyRegistered
        );

        // --- 7. Ensure the passed block number is valid, not in the future or too old.
        // Work must have been done within 3 blocks (stops long range attacks).
        let current_block_number: u64 = Self::get_current_block_as_u64();
        ensure!(
            block_number <= current_block_number,
            Error::<T>::InvalidWorkBlock
        );
        ensure!(
            current_block_number - block_number < 3,
            Error::<T>::InvalidWorkBlock
        );

        // --- 8. Ensure the supplied work passes the difficulty.
        let difficulty: U256 = Self::get_difficulty(netuid);
        let work_hash: H256 = Self::vec_to_hash(work.clone());
        ensure!(
            Self::hash_meets_difficulty(&work_hash, difficulty),
            Error::<T>::InvalidDifficulty
        ); // Check that the work meets difficulty.

        // --- 7. Check Work is the product of the nonce, the block number, and computekey. Add this as used work.
        let seal: H256 = Self::create_seal_hash(block_number, nonce, &computekey);
        ensure!(seal == work_hash, Error::<T>::InvalidSeal);
        UsedWork::<T>::insert(&work.clone(), current_block_number);

        // DEPRECATED --- 8. Ensure that the key passes the registration requirement
        // ensure!(
        //     Self::passes_network_connection_requirement(netuid, &computekey),
        //     Error::<T>::DidNotPassConnectedNetworkRequirement
        // );

        // --- 9. If the network account does not exist we will create it here.
        Self::create_account_if_non_existent(&personalkey, &computekey);

        // --- 10. Ensure that the pairing is correct.
        ensure!(
            Self::personalkey_owns_computekey(&personalkey, &computekey),
            Error::<T>::NonAssociatedpersonalkey
        );

        // --- 11. Append agent or prune it.
        let brain_uid: u16;
        let current_brain_n: u16 = Self::get_brain_n(netuid);

        // Possibly there is no agent slots at all.
        ensure!(
            Self::get_max_allowed_uids(netuid) != 0,
            Error::<T>::NetworkDoesNotExist
        );

        if current_brain_n < Self::get_max_allowed_uids(netuid) {
            // --- 11.1.1 No replacement required, the uid appends the brain.
            // We increment the brain count here but not below.
            brain_uid = current_brain_n;

            // --- 11.1.2 Expand brain with new account.
            Self::append_agent(netuid, &computekey, current_block_number);
            log::info!("add new agent account");
        } else {
            // --- 11.1.1 Replacement required.
            // We take the agent with the lowest pruning score here.
            brain_uid = Self::get_agent_to_prune(netuid);

            // --- 11.1.1 Replace the agent account with the new info.
            Self::replace_agent(netuid, brain_uid, &computekey, current_block_number);
            log::info!("prune agent");
        }

        // --- 12. Record the registration and increment block and interval counters.
        POWRegistrationsThisInterval::<T>::mutate(netuid, |val| *val += 1);
        RegistrationsThisInterval::<T>::mutate(netuid, |val| *val += 1);
        RegistrationsThisBlock::<T>::mutate(netuid, |val| *val += 1);

        // --- 13. Deposit successful event.
        log::info!(
            "AgentRegistered( netuid:{:?} uid:{:?} computekey:{:?}  ) ",
            netuid,
            brain_uid,
            computekey
        );
        Self::deposit_event(Event::AgentRegistered(netuid, brain_uid, computekey));

        // --- 14. Ok and done.
        Ok(())
    }

    pub fn do_faucet(
        origin: T::RuntimeOrigin,
        block_number: u64,
        nonce: u64,
        work: Vec<u8>,
    ) -> DispatchResult {
        // --- 0. Ensure the faucet is enabled.
        // ensure!(AllowFaucet::<T>::get(), Error::<T>::FaucetDisabled);

        // --- 1. Check that the caller has signed the transaction.
        let personalkey = ensure_signed(origin)?;
        log::info!("do_faucet( personalkey:{:?} )", personalkey);

        // --- 2. Ensure the passed block number is valid, not in the future or too old.
        // Work must have been done within 3 blocks (stops long range attacks).
        let current_block_number: u64 = Self::get_current_block_as_u64();
        ensure!(
            block_number <= current_block_number,
            Error::<T>::InvalidWorkBlock
        );
        ensure!(
            current_block_number - block_number < 3,
            Error::<T>::InvalidWorkBlock
        );

        // --- 3. Ensure the supplied work passes the difficulty.
        let difficulty: U256 = U256::from(1_000_000); // Base faucet difficulty.
        let work_hash: H256 = Self::vec_to_hash(work.clone());
        ensure!(
            Self::hash_meets_difficulty(&work_hash, difficulty),
            Error::<T>::InvalidDifficulty
        ); // Check that the work meets difficulty.

        // --- 4. Check Work is the product of the nonce, the block number, and computekey. Add this as used work.
        let seal: H256 = Self::create_seal_hash(block_number, nonce, &personalkey);
        ensure!(seal == work_hash, Error::<T>::InvalidSeal);
        UsedWork::<T>::insert(&work.clone(), current_block_number);

        // --- 5. Add Balance via faucet.
        let balance_to_add: u64 = 100_000_000_000;
        let balance_to_be_added_as_balance = Self::u64_to_balance(balance_to_add);
        Self::add_balance_to_personalkey_account(&personalkey, balance_to_be_added_as_balance.unwrap());
        TotalIssuance::<T>::put(TotalIssuance::<T>::get().saturating_add(balance_to_add.into()));

        // --- 6. Deposit successful event.
        log::info!(
            "Faucet( personalkey:{:?} amount:{:?} ) ",
            personalkey,
            balance_to_add
        );
        Self::deposit_event(Event::Faucet(personalkey, balance_to_add));

        // --- 7. Ok and done.
        Ok(())
    }

    pub fn vec_to_hash(vec_hash: Vec<u8>) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice(de_de_ref_hash);
        return real_hash;
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning score out of
    // immunity period. If all agents are in immunity period, return node with lowest prunning score.
    // This function will always return an element to prune.
    pub fn get_agent_to_prune(netuid: u16) -> u16 {
        let mut min_score: u16 = u16::MAX;
        let mut min_score_in_immunity_period = u16::MAX;
        let mut uid_with_min_score = 0;
        let mut uid_with_min_score_in_immunity_period: u16 = 0;
        if Self::get_brain_n(netuid) == 0 {
            return 0;
        } // If there are no agents in this network.
        for agent_uid_i in 0..Self::get_brain_n(netuid) {
            let pruning_score: u16 = Self::get_pruning_score_for_uid(netuid, agent_uid_i);
            let block_at_registration: u64 =
                Self::get_agent_block_at_registration(netuid, agent_uid_i);
            let current_block: u64 = Self::get_current_block_as_u64();
            let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;
            if min_score == pruning_score {
                if current_block - block_at_registration < immunity_period {
                    //agent is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                        min_score_in_immunity_period = pruning_score;
                        uid_with_min_score_in_immunity_period = agent_uid_i;
                    }
                } else {
                    min_score = pruning_score;
                    uid_with_min_score = agent_uid_i;
                }
            }
            // Find min pruning score.
            else if min_score > pruning_score {
                if current_block - block_at_registration < immunity_period {
                    //agent is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                        min_score_in_immunity_period = pruning_score;
                        uid_with_min_score_in_immunity_period = agent_uid_i;
                    }
                } else {
                    min_score = pruning_score;
                    uid_with_min_score = agent_uid_i;
                }
            }
        }
        if min_score == u16::MAX {
            //all neuorns are in immunity period
            Self::set_pruning_score_for_uid(
                netuid,
                uid_with_min_score_in_immunity_period,
                u16::MAX,
            );
            return uid_with_min_score_in_immunity_period;
        } else {
            // We replace the pruning score here with u16 max to ensure that all peers always have a
            // pruning score. In the event that every peer has been pruned this function will prune
            // the last element in the network continually.
            Self::set_pruning_score_for_uid(netuid, uid_with_min_score, u16::MAX);
            return uid_with_min_score;
        }
    }

    // Determine whether the given hash satisfies the given difficulty.
    // The test is done by multiplying the two together. If the product
    // overflows the bounds of U256, then the product (and thus the hash)
    // was too high.
    pub fn hash_meets_difficulty(hash: &H256, difficulty: U256) -> bool {
        let bytes: &[u8] = &hash.as_bytes();
        let num_hash: U256 = U256::from(bytes);
        let (value, overflowed) = num_hash.overflowing_mul(difficulty);

        log::trace!(
            target: LOG_TARGET,
            "Difficulty: hash: {:?}, hash_bytes: {:?}, hash_as_num: {:?}, difficulty: {:?}, value: {:?} overflowed: {:?}",
            hash,
            bytes,
            num_hash,
            difficulty,
            value,
            overflowed
        );
        !overflowed
    }

    pub fn get_block_hash_from_u64(block_number: u64) -> H256 {
        let block_number: T::BlockNumber = TryInto::<T::BlockNumber>::try_into(block_number)
            .ok()
            .expect("convert u64 to block number.");
        let block_hash_at_number: <T as frame_system::Config>::Hash =
            system::Pallet::<T>::block_hash(block_number);
        let vec_hash: Vec<u8> = block_hash_at_number.as_ref().into_iter().cloned().collect();
        let deref_vec_hash: &[u8] = &vec_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice(deref_vec_hash);

        log::trace!(
            target: LOG_TARGET,
            "block_number: {:?}, vec_hash: {:?}, real_hash: {:?}",
            block_number,
            vec_hash,
            real_hash
        );

        return real_hash;
    }

    pub fn hash_to_vec(hash: H256) -> Vec<u8> {
        let hash_as_bytes: &[u8] = hash.as_bytes();
        let hash_as_vec: Vec<u8> = hash_as_bytes.iter().cloned().collect();
        return hash_as_vec;
    }

    pub fn hash_block_and_computekey(block_hash_bytes: &[u8], computekey: &T::AccountId) -> H256 {
        // Get the public key from the account id.
        let computekey_pubkey: MultiAddress<T::AccountId, ()> = MultiAddress::Id(computekey.clone());
        let binding = computekey_pubkey.encode();
        // Skip extra 0th byte.
        let computekey_bytes: &[u8] = binding[1..].as_ref();
        let full_bytes: &[u8; 64] = &[
            block_hash_bytes[0],
            block_hash_bytes[1],
            block_hash_bytes[2],
            block_hash_bytes[3],
            block_hash_bytes[4],
            block_hash_bytes[5],
            block_hash_bytes[6],
            block_hash_bytes[7],
            block_hash_bytes[8],
            block_hash_bytes[9],
            block_hash_bytes[10],
            block_hash_bytes[11],
            block_hash_bytes[12],
            block_hash_bytes[13],
            block_hash_bytes[14],
            block_hash_bytes[15],
            block_hash_bytes[16],
            block_hash_bytes[17],
            block_hash_bytes[18],
            block_hash_bytes[19],
            block_hash_bytes[20],
            block_hash_bytes[21],
            block_hash_bytes[22],
            block_hash_bytes[23],
            block_hash_bytes[24],
            block_hash_bytes[25],
            block_hash_bytes[26],
            block_hash_bytes[27],
            block_hash_bytes[28],
            block_hash_bytes[29],
            block_hash_bytes[30],
            block_hash_bytes[31],
            computekey_bytes[0],
            computekey_bytes[1],
            computekey_bytes[2],
            computekey_bytes[3],
            computekey_bytes[4],
            computekey_bytes[5],
            computekey_bytes[6],
            computekey_bytes[7],
            computekey_bytes[8],
            computekey_bytes[9],
            computekey_bytes[10],
            computekey_bytes[11],
            computekey_bytes[12],
            computekey_bytes[13],
            computekey_bytes[14],
            computekey_bytes[15],
            computekey_bytes[16],
            computekey_bytes[17],
            computekey_bytes[18],
            computekey_bytes[19],
            computekey_bytes[20],
            computekey_bytes[21],
            computekey_bytes[22],
            computekey_bytes[23],
            computekey_bytes[24],
            computekey_bytes[25],
            computekey_bytes[26],
            computekey_bytes[27],
            computekey_bytes[28],
            computekey_bytes[29],
            computekey_bytes[30],
            computekey_bytes[31],
        ];
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(full_bytes);
        let seal_hash: H256 = H256::from_slice(&keccak_256_seal_hash_vec);

        return seal_hash;
    }

    pub fn create_seal_hash(block_number_u64: u64, nonce_u64: u64, computekey: &T::AccountId) -> H256 {
        let nonce = U256::from(nonce_u64);
        let block_hash_at_number: H256 = Self::get_block_hash_from_u64(block_number_u64);
        let block_hash_bytes: &[u8] = block_hash_at_number.as_bytes();
        let binding = Self::hash_block_and_computekey(block_hash_bytes, computekey);
        let block_and_computekey_hash_bytes: &[u8] = binding.as_bytes();

        let full_bytes: &[u8; 40] = &[
            nonce.byte(0),
            nonce.byte(1),
            nonce.byte(2),
            nonce.byte(3),
            nonce.byte(4),
            nonce.byte(5),
            nonce.byte(6),
            nonce.byte(7),
            block_and_computekey_hash_bytes[0],
            block_and_computekey_hash_bytes[1],
            block_and_computekey_hash_bytes[2],
            block_and_computekey_hash_bytes[3],
            block_and_computekey_hash_bytes[4],
            block_and_computekey_hash_bytes[5],
            block_and_computekey_hash_bytes[6],
            block_and_computekey_hash_bytes[7],
            block_and_computekey_hash_bytes[8],
            block_and_computekey_hash_bytes[9],
            block_and_computekey_hash_bytes[10],
            block_and_computekey_hash_bytes[11],
            block_and_computekey_hash_bytes[12],
            block_and_computekey_hash_bytes[13],
            block_and_computekey_hash_bytes[14],
            block_and_computekey_hash_bytes[15],
            block_and_computekey_hash_bytes[16],
            block_and_computekey_hash_bytes[17],
            block_and_computekey_hash_bytes[18],
            block_and_computekey_hash_bytes[19],
            block_and_computekey_hash_bytes[20],
            block_and_computekey_hash_bytes[21],
            block_and_computekey_hash_bytes[22],
            block_and_computekey_hash_bytes[23],
            block_and_computekey_hash_bytes[24],
            block_and_computekey_hash_bytes[25],
            block_and_computekey_hash_bytes[26],
            block_and_computekey_hash_bytes[27],
            block_and_computekey_hash_bytes[28],
            block_and_computekey_hash_bytes[29],
            block_and_computekey_hash_bytes[30],
            block_and_computekey_hash_bytes[31],
        ];
        let sha256_seal_hash_vec: [u8; 32] = sha2_256(full_bytes);
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(&sha256_seal_hash_vec);
        let seal_hash: H256 = H256::from_slice(&keccak_256_seal_hash_vec);

        log::trace!(
			"\n computekey:{:?} \nblock_number: {:?}, \nnonce_u64: {:?}, \nblock_hash: {:?}, \nfull_bytes: {:?}, \nsha256_seal_hash_vec: {:?},  \nkeccak_256_seal_hash_vec: {:?}, \nseal_hash: {:?}",
			computekey,
            block_number_u64,
			nonce_u64,
			block_hash_at_number,
			full_bytes,
			sha256_seal_hash_vec,
            keccak_256_seal_hash_vec,
			seal_hash
		);

        return seal_hash;
    }

    // Helper function for creating nonce and work.
    pub fn create_work_for_block_number(
        netuid: u16,
        block_number: u64,
        start_nonce: u64,
        computekey: &T::AccountId,
    ) -> (u64, Vec<u8>) {
        let difficulty: U256 = Self::get_difficulty(netuid);
        let mut nonce: u64 = start_nonce;
        let mut work: H256 = Self::create_seal_hash(block_number, nonce, &computekey);
        while !Self::hash_meets_difficulty(&work, difficulty) {
            nonce = nonce + 1;
            work = Self::create_seal_hash(block_number, nonce, &computekey);
        }
        let vec_work: Vec<u8> = Self::hash_to_vec(work);
        return (nonce, vec_work);
    }

    pub fn do_swap_computekey(origin: T::RuntimeOrigin, old_computekey: &T::AccountId, new_computekey: &T::AccountId) -> DispatchResultWithPostInfo {
        let personalkey = ensure_signed(origin)?;

        let mut weight = T::DbWeight::get().reads_writes(2, 0);
        ensure!(Self::personalkey_owns_computekey(&personalkey, old_computekey), Error::<T>::NonAssociatedpersonalkey);

        let block: u64 = Self::get_current_block_as_u64();
        ensure!(
            !Self::exceeds_tx_rate_limit(Self::get_last_tx_block(&personalkey), block),
            Error::<T>::TxRateLimitExceeded
        );

        weight.saturating_accrue(T::DbWeight::get().reads(2));

        ensure!(old_computekey != new_computekey, Error::<T>::AlreadyRegistered);
        ensure!(!Self::is_computekey_registered_on_any_network(new_computekey), Error::<T>::AlreadyRegistered);

        weight.saturating_accrue(T::DbWeight::get().reads((TotalNetworks::<T>::get() + 1u16) as u64));

        let swap_cost = 1_000_000_000_000_000_000u128;
        let swap_cost_as_balance = Self::u128_to_balance(swap_cost).unwrap();
        ensure!(
            Self::can_remove_balance_from_personalkey_account(&personalkey, swap_cost_as_balance),
            Error::<T>::NotEnoughBalance
        );
        ensure!(
            Self::remove_balance_from_personalkey_account(&personalkey, swap_cost_as_balance)
                == true,
            Error::<T>::BalanceWithdrawalError
        );
        Self::burn_tokens(swap_cost.into());

        Owner::<T>::remove(old_computekey);
        Owner::<T>::insert(new_computekey, personalkey.clone());
        weight.saturating_accrue(T::DbWeight::get().writes(2));

        if let Ok(total_computekey_stake) = TotalComputekeyStake::<T>::try_get(old_computekey) {
            TotalComputekeyStake::<T>::remove(old_computekey);
            TotalComputekeyStake::<T>::insert(new_computekey, total_computekey_stake);

            weight.saturating_accrue(T::DbWeight::get().writes(2));
        }

        if let Ok(delegate_take) = Delegates::<T>::try_get(old_computekey) {
            Delegates::<T>::remove(old_computekey);
            Delegates::<T>::insert(new_computekey, delegate_take);

            weight.saturating_accrue(T::DbWeight::get().writes(2));
        }

        if let Ok(last_tx) = LastTxBlock::<T>::try_get(old_computekey) {
            LastTxBlock::<T>::remove(old_computekey);
            LastTxBlock::<T>::insert(new_computekey, last_tx);

            weight.saturating_accrue(T::DbWeight::get().writes(2));
        }

        let mut personalkey_stake: Vec<(T::AccountId, u64)> = vec![];
        for (personalkey, stake_amount) in Stake::<T>::iter_prefix(old_computekey) {
            personalkey_stake.push((personalkey.clone(), stake_amount));
        }

        let _ = Stake::<T>::clear_prefix(old_computekey, personalkey_stake.len() as u32, None);
        weight.saturating_accrue(T::DbWeight::get().writes(personalkey_stake.len() as u64));

        for (personalkey, stake_amount) in personalkey_stake {
            Stake::<T>::insert(new_computekey, personalkey, stake_amount);
            weight.saturating_accrue(T::DbWeight::get().writes(1));
        }

        let mut netuid_is_member: Vec<u16> = vec![];
        for netuid in <IsNetworkMember<T> as IterableStorageDoubleMap<T::AccountId, u16, bool>>::iter_key_prefix(old_computekey) {
            netuid_is_member.push(netuid);
        }

        let _ = IsNetworkMember::<T>::clear_prefix(old_computekey, netuid_is_member.len() as u32, None);
        weight.saturating_accrue(T::DbWeight::get().writes(netuid_is_member.len() as u64));

        for netuid in netuid_is_member.iter() {
            IsNetworkMember::<T>::insert(new_computekey, netuid, true);
            weight.saturating_accrue(T::DbWeight::get().writes(1));
        }

        for netuid in netuid_is_member.iter() {
            if let Ok(brainport_info) = Brainports::<T>::try_get(netuid, old_computekey) {
                Brainports::<T>::remove(netuid, old_computekey);
                Brainports::<T>::insert(netuid, new_computekey, brainport_info);

                weight.saturating_accrue(T::DbWeight::get().writes(2));
            }
        }

        for netuid in netuid_is_member.iter() {
            if let Ok(uid) = Uids::<T>::try_get(netuid, old_computekey) {
                Uids::<T>::remove(netuid, old_computekey);
                Uids::<T>::insert(netuid, new_computekey, uid);

                weight.saturating_accrue(T::DbWeight::get().writes(2));

                Keys::<T>::insert(netuid, uid, new_computekey);

                weight.saturating_accrue(T::DbWeight::get().writes(1));

                LoadedEmission::<T>::mutate(netuid, |emission_exists| {
                    match emission_exists {
                        Some(emissions) => {
                            if let Some(emission) = emissions.get_mut(uid as usize) {
                                let (_, se, ve) = emission;
                                *emission = (new_computekey.clone(), *se, *ve);

                            }
                        }
                        None => {}
                    }
                });

                weight.saturating_accrue(T::DbWeight::get().writes(1));
            }
        }

        Self::set_last_tx_block(&personalkey, block);
        weight.saturating_accrue(T::DbWeight::get().writes(1));

        Self::deposit_event(Event::ComputekeySwapped{personalkey, old_computekey: old_computekey.clone(), new_computekey: new_computekey.clone()});

        Ok(Some(weight).into())
    }
}
