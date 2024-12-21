use super::*;
use frame_support::storage::IterableStorageDoubleMap;

impl<T: Config> Pallet<T> {
    // ---- The implementation for the extrinsic become_delegate: signals that this computekey allows delegated stake.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's personalkey.
    //
    // 	* 'computekey' (T::AccountId):
    // 		- The computekey we are delegating (must be owned by the personalkey.)
    //
    // 	* 'take' (u16):
    // 		- The stake proportion that this computekey takes from delegations.
    //
    // # Event:
    // 	* DelegateAdded;
    // 		- On successfully setting a computekey as a delegate.
    //
    // # Raises:
    // 	* 'NotRegistered':
    // 		- The computekey we are delegating is not registered on the network.
    //
    // 	* 'NonAssociatedpersonalkey':
    // 		- The computekey we are delegating is not owned by the calling coldket.
    //
    // 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
    pub fn do_become_delegate(
        origin: T::RuntimeOrigin,
        computekey: T::AccountId,
        take: u16,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the personalkey signuture.
        let personalkey = ensure_signed(origin)?;
        log::info!(
            "do_become_delegate( origin:{:?} computekey:{:?}, take:{:?} )",
            personalkey,
            computekey,
            take
        );

        // --- 2. Ensure we are delegating an known key.
        ensure!(
            Self::computekey_account_exists(&computekey),
            Error::<T>::NotRegistered
        );

        // --- 3. Ensure that the personalkey is the owner.
        ensure!(
            Self::personalkey_owns_computekey(&personalkey, &computekey),
            Error::<T>::NonAssociatedpersonalkey
        );

        // --- 4. Ensure we are not already a delegate (dont allow changing delegate take.)
        ensure!(
            !Self::computekey_is_delegate(&computekey),
            Error::<T>::AlreadyDelegate
        );

        // --- 5. Ensure we don't exceed tx rate limit
        let block: u64 = Self::get_current_block_as_u64();
        ensure!(
            !Self::exceeds_tx_rate_limit(Self::get_last_tx_block(&personalkey), block),
            Error::<T>::TxRateLimitExceeded
        );

        // --- 6. Delegate the key.
        Self::delegate_computekey(&computekey, take);

        // Set last block for rate limiting
        Self::set_last_tx_block(&personalkey, block);

        // --- 7. Emit the staking event.
        log::info!(
            "DelegateAdded( personalkey:{:?}, computekey:{:?}, take:{:?} )",
            personalkey,
            computekey,
            take
        );
        Self::deposit_event(Event::DelegateAdded(personalkey, computekey, take));

        // --- 8. Ok and return.
        Ok(())
    }

    // ---- The implementation for the extrinsic add_stake: Adds stake to a computekey account.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's personalkey.
    //
    // 	* 'computekey' (T::AccountId):
    // 		- The associated computekey account.
    //
    // 	* 'stake_to_be_added' (u64):
    // 		- The amount of stake to be added to the computekey staking account.
    //
    // # Event:
    // 	* StakeAdded;
    // 		- On the successfully adding stake to a global account.
    //
    // # Raises:
    // 	* 'CouldNotConvertToBalance':
    // 		- Unable to convert the passed stake value to a balance.
    //
    // 	* 'NotEnoughBalanceToStake':
    // 		- Not enough balance on the personalkey to add onto the global account.
    //
    // 	* 'NonAssociatedpersonalkey':
    // 		- The calling personalkey is not associated with this computekey.
    //
    // 	* 'BalanceWithdrawalError':
    // 		- Errors stemming from transaction pallet.
    //
    // 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
    pub fn do_add_stake(
        origin: T::RuntimeOrigin,
        computekey: T::AccountId,
        stake_to_be_added: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the T::AccountId personalkey information.
        let personalkey = ensure_signed(origin)?;
        log::info!(
            "do_add_stake( origin:{:?} computekey:{:?}, stake_to_be_added:{:?} )",
            personalkey,
            computekey,
            stake_to_be_added
        );

        // --- 2. We convert the stake u64 into a balancer.
        let stake_as_balance = Self::u64_to_balance(stake_to_be_added);
        ensure!(
            stake_as_balance.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        // --- 3. Ensure the callers personalkey has enough stake to perform the transaction.
        ensure!(
            Self::can_remove_balance_from_personalkey_account(&personalkey, stake_as_balance.unwrap()),
            Error::<T>::NotEnoughBalanceToStake
        );

        // --- 4. Ensure that the computekey account exists this is only possible through registration.
        ensure!(
            Self::computekey_account_exists(&computekey),
            Error::<T>::NotRegistered
        );

        // --- 5. Ensure that the computekey allows delegation or that the computekey is owned by the calling personalkey.
        ensure!(
            Self::computekey_is_delegate(&computekey) || Self::personalkey_owns_computekey(&personalkey, &computekey),
            Error::<T>::NonAssociatedpersonalkey
        );

        // --- 6. Ensure we don't exceed tx rate limit
        let block: u64 = Self::get_current_block_as_u64();
        ensure!(
            !Self::exceeds_tx_rate_limit(Self::get_last_tx_block(&personalkey), block),
            Error::<T>::TxRateLimitExceeded
        );

        // --- 7. Ensure the remove operation from the personalkey is a success.
        ensure!(
            Self::remove_balance_from_personalkey_account(&personalkey, stake_as_balance.unwrap()) == true,
            Error::<T>::BalanceWithdrawalError
        );

        // --- 8. If we reach here, add the balance to the computekey.
        Self::increase_stake_on_personalkey_computekey_account(&personalkey, &computekey, stake_to_be_added);

        // Set last block for rate limiting
        Self::set_last_tx_block(&personalkey, block);

        // --- 9. Emit the staking event.
        log::info!(
            "StakeAdded( computekey:{:?}, stake_to_be_added:{:?} )",
            computekey,
            stake_to_be_added
        );
        Self::deposit_event(Event::StakeAdded(computekey, stake_to_be_added));

        // --- 10. Ok and return.
        Ok(())
    }

    // ---- The implementation for the extrinsic remove_stake: Removes stake from a computekey account and adds it onto a personalkey.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller's personalkey.
    //
    // 	* 'computekey' (T::AccountId):
    // 		- The associated computekey account.
    //
    // 	* 'stake_to_be_added' (u64):
    // 		- The amount of stake to be added to the computekey staking account.
    //
    // # Event:
    // 	* StakeRemoved;
    // 		- On the successfully removing stake from the computekey account.
    //
    // # Raises:
    // 	* 'NotRegistered':
    // 		- Thrown if the account we are attempting to unstake from is non existent.
    //
    // 	* 'NonAssociatedpersonalkey':
    // 		- Thrown if the personalkey does not own the computekey we are unstaking from.
    //
    // 	* 'NotEnoughStaketoWithdraw':
    // 		- Thrown if there is not enough stake on the computekey to withdwraw this amount.
    //
    // 	* 'CouldNotConvertToBalance':
    // 		- Thrown if we could not convert this amount to a balance.
    //
    // 	* 'TxRateLimitExceeded':
    // 		- Thrown if key has hit transaction rate limit
    //
    //
    pub fn do_remove_stake(
        origin: T::RuntimeOrigin,
        computekey: T::AccountId,
        stake_to_be_removed: u64,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId personalkey information.
        let personalkey = ensure_signed(origin)?;
        log::info!(
            "do_remove_stake( origin:{:?} computekey:{:?}, stake_to_be_removed:{:?} )",
            personalkey,
            computekey,
            stake_to_be_removed
        );

        // --- 2. Ensure that the computekey account exists this is only possible through registration.
        ensure!(
            Self::computekey_account_exists(&computekey),
            Error::<T>::NotRegistered
        );

        // --- 3. Ensure that the computekey allows delegation or that the computekey is owned by the calling personalkey.
        ensure!(
            Self::computekey_is_delegate(&computekey) || Self::personalkey_owns_computekey(&personalkey, &computekey),
            Error::<T>::NonAssociatedpersonalkey
        );

        // --- Ensure that the stake amount to be removed is above zero.
        ensure!(
            stake_to_be_removed > 0,
            Error::<T>::NotEnoughStaketoWithdraw
        );

        // --- 4. Ensure that the computekey has enough stake to withdraw.
        ensure!(
            Self::has_enough_stake(&personalkey, &computekey, stake_to_be_removed),
            Error::<T>::NotEnoughStaketoWithdraw
        );

        // --- 5. Ensure that we can conver this u64 to a balance.
        let stake_to_be_added_as_currency = Self::u64_to_balance(stake_to_be_removed);
        ensure!(
            stake_to_be_added_as_currency.is_some(),
            Error::<T>::CouldNotConvertToBalance
        );

        // --- 6. Ensure we don't exceed tx rate limit
        let block: u64 = Self::get_current_block_as_u64();
        ensure!(
            !Self::exceeds_tx_rate_limit(Self::get_last_tx_block(&personalkey), block),
            Error::<T>::TxRateLimitExceeded
        );

        // --- 7. We remove the balance from the computekey.
        Self::decrease_stake_on_personalkey_computekey_account(&personalkey, &computekey, stake_to_be_removed);

        // --- 8. We add the balancer to the personalkey.  If the above fails we will not credit this personalkey.
        Self::add_balance_to_personalkey_account(&personalkey, stake_to_be_added_as_currency.unwrap());

        // Set last block for rate limiting
        Self::set_last_tx_block(&personalkey, block);

        // --- 9. Emit the unstaking event.
        log::info!(
            "StakeRemoved( computekey:{:?}, stake_to_be_removed:{:?} )",
            computekey,
            stake_to_be_removed
        );
        Self::deposit_event(Event::StakeRemoved(computekey, stake_to_be_removed));

        // --- 10. Done and ok.
        Ok(())
    }

    // Returns true if the passed computekey allow delegative staking.
    //
    pub fn computekey_is_delegate(computekey: &T::AccountId) -> bool {
        return Delegates::<T>::contains_key(computekey);
    }

    // Sets the computekey as a delegate with take.
    //
    pub fn delegate_computekey(computekey: &T::AccountId, take: u16) {
        Delegates::<T>::insert(computekey, take);
    }

    // Returns the total amount of stake in the staking table.
    //
    pub fn get_total_stake() -> u64 {
        return TotalStake::<T>::get();
    }

    // Increases the total amount of stake by the passed amount.
    //
    pub fn increase_total_stake(increment: u64) {
        TotalStake::<T>::put(Self::get_total_stake().saturating_add(increment));
    }

    // Decreases the total amount of stake by the passed amount.
    //
    pub fn decrease_total_stake(decrement: u64) {
        TotalStake::<T>::put(Self::get_total_stake().saturating_sub(decrement));
    }

    // Returns the total amount of stake under a computekey (delegative or otherwise)
    //
    pub fn get_total_stake_for_computekey(computekey: &T::AccountId) -> u64 {
        return TotalComputekeyStake::<T>::get(computekey);
    }

    // Returns the total amount of stake held by the personalkey (delegative or otherwise)
    //
    pub fn get_total_stake_for_personalkey(personalkey: &T::AccountId) -> u64 {
        return TotalPersonalkeyStake::<T>::get(personalkey);
    }

    // Returns the stake under the cold - hot pairing in the staking table.
    //
    pub fn get_stake_for_personalkey_and_computekey(personalkey: &T::AccountId, computekey: &T::AccountId) -> u64 {
        return Stake::<T>::get(computekey, personalkey);
    }

    // Creates a cold - hot pairing account if the computekey is not already an active account.
    //
    pub fn create_account_if_non_existent(personalkey: &T::AccountId, computekey: &T::AccountId) {
        if !Self::computekey_account_exists(computekey) {
            Stake::<T>::insert(computekey, personalkey, 0);
            Owner::<T>::insert(computekey, personalkey);
        }
    }

    // Returns the personalkey owning this computekey. This function should only be called for active accounts.
    //
    pub fn get_owning_personalkey_for_computekey(computekey: &T::AccountId) -> T::AccountId {
        return Owner::<T>::get(computekey);
    }

    // Returns true if the computekey account has been created.
    //
    pub fn computekey_account_exists(computekey: &T::AccountId) -> bool {
        return Owner::<T>::contains_key(computekey);
    }

    // Return true if the passed personalkey owns the computekey.
    //
    pub fn personalkey_owns_computekey(personalkey: &T::AccountId, computekey: &T::AccountId) -> bool {
        if Self::computekey_account_exists(computekey) {
            return Owner::<T>::get(computekey) == *personalkey;
        } else {
            return false;
        }
    }

    // Returns true if the cold-hot staking account has enough balance to fufil the decrement.
    //
    pub fn has_enough_stake(personalkey: &T::AccountId, computekey: &T::AccountId, decrement: u64) -> bool {
        return Self::get_stake_for_personalkey_and_computekey(personalkey, computekey) >= decrement;
    }

    // Increases the stake on the computekey account under its owning personalkey.
    //
    pub fn increase_stake_on_computekey_account(computekey: &T::AccountId, increment: u64) {
        log::debug!("increase_stake_on_computekey ck: {:?}, increment: {:?}", computekey, increment);
        Self::increase_stake_on_personalkey_computekey_account(
            &Self::get_owning_personalkey_for_computekey(computekey),
            computekey,
            increment,
        );
    }

    // Decreases the stake on the computekey account under its owning personalkey.
    //
    pub fn decrease_stake_on_computekey_account(computekey: &T::AccountId, decrement: u64) {
        Self::decrease_stake_on_personalkey_computekey_account(
            &Self::get_owning_personalkey_for_computekey(computekey),
            computekey,
            decrement,
        );
    }

    // Increases the stake on the cold - hot pairing by increment while also incrementing other counters.
    // This function should be called rather than set_stake under account.
    //
    pub fn increase_stake_on_personalkey_computekey_account(
        personalkey: &T::AccountId,
        computekey: &T::AccountId,
        increment: u64,
    ) {
        TotalPersonalkeyStake::<T>::insert(
            personalkey,
            TotalPersonalkeyStake::<T>::get(personalkey).saturating_add(increment),
        );
        TotalComputekeyStake::<T>::insert(
            computekey,
            TotalComputekeyStake::<T>::get(computekey).saturating_add(increment),
        );
        Stake::<T>::insert(
            computekey,
            personalkey,
            Stake::<T>::get(computekey, personalkey).saturating_add(increment),
        );
        TotalStake::<T>::put(TotalStake::<T>::get().saturating_add(increment));
        TotalIssuance::<T>::put(TotalIssuance::<T>::get().saturating_add(increment.into()));
    }

    // Decreases the stake on the cold - hot pairing by the decrement while decreasing other counters.
    //
    pub fn decrease_stake_on_personalkey_computekey_account(
        personalkey: &T::AccountId,
        computekey: &T::AccountId,
        decrement: u64,
    ) {
        TotalPersonalkeyStake::<T>::mutate(personalkey, |old| *old = old.saturating_sub(decrement));
        TotalComputekeyStake::<T>::insert(
            computekey,
            TotalComputekeyStake::<T>::get(computekey).saturating_sub(decrement),
        );
        Stake::<T>::insert(
            computekey,
            personalkey,
            Stake::<T>::get(computekey, personalkey).saturating_sub(decrement),
        );
        TotalStake::<T>::put(TotalStake::<T>::get().saturating_sub(decrement));
        TotalIssuance::<T>::put(TotalIssuance::<T>::get().saturating_sub(decrement.into()));
    }

    pub fn u64_to_balance(
        input: u64,
    ) -> Option<
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
    > {
        input.try_into().ok()
    }

    pub fn u128_to_balance(
        input: u128,
    ) -> Option<
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
    > {
        input.try_into().ok()
    }

    pub fn add_balance_to_personalkey_account(
        personalkey: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) {
        T::Currency::deposit_creating(&personalkey, amount); // Infallibe
    }

    pub fn set_balance_on_personalkey_account(
        personalkey: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) {
        T::Currency::make_free_balance_be(&personalkey, amount);
    }

    pub fn can_remove_balance_from_personalkey_account(
        personalkey: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) -> bool {
        let current_balance = Self::get_personalkey_balance(personalkey);
		log::debug!("can_remove_balance_from_personalkey_account current_balace = {:?} amount = {:?} personalkey = {:?}", current_balance, amount, personalkey);
        if amount > current_balance {
            return false;
        }

        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw = T::Currency::ensure_can_withdraw(
            &personalkey,
            amount,
            WithdrawReasons::except(WithdrawReasons::TIP),
            new_potential_balance,
        )
        .is_ok();
        can_withdraw
    }

    pub fn get_personalkey_balance(
        personalkey: &T::AccountId,
    ) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&personalkey);
    }

    pub fn remove_balance_from_personalkey_account(
        personalkey: &T::AccountId,
        amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance,
    ) -> bool {
        return match T::Currency::withdraw(
            &personalkey,
            amount,
            WithdrawReasons::except(WithdrawReasons::TIP),
            ExistenceRequirement::KeepAlive,
        ) {
            Ok(_result) => true,
            Err(_error) => false,
        };
    }

    pub fn unstake_all_personalkeys_from_computekey_account(computekey: &T::AccountId) {
        // Iterate through all personalkeys that have a stake on this computekey account.
        for (delegate_personalkey_i, stake_i) in
            <Stake<T> as IterableStorageDoubleMap<T::AccountId, T::AccountId, u64>>::iter_prefix(
                computekey,
            )
        {
            // Convert to balance and add to the personalkey account.
            let stake_i_as_balance = Self::u64_to_balance(stake_i);
            if stake_i_as_balance.is_none() {
                continue; // Don't unstake if we can't convert to balance.
            } else {
                // Stake is successfully converted to balance.

                // Remove the stake from the personalkey - computekey pairing.
                Self::decrease_stake_on_personalkey_computekey_account(
                    &delegate_personalkey_i,
                    computekey,
                    stake_i,
                );

                // Add the balance to the personalkey account.
                Self::add_balance_to_personalkey_account(
                    &delegate_personalkey_i,
                    stake_i_as_balance.unwrap(),
                );
            }
        }
    }
}
