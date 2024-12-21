use super::*;
use frame_support::{sp_std::vec};
use sp_std::vec::Vec;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::DispatchError;
use frame_support::storage::IterableStorageDoubleMap;

impl<T: Config> Pallet<T> {

    // Returns the number of filled slots on a network.
    ///
    pub fn get_brain_n( netuid:u16 ) -> u16 {
        return BrainN::<T>::get( netuid )
    }

    // Replace the agent under this uid.
    pub fn replace_agent( netuid: u16, uid_to_replace: u16, new_computekey: &T::AccountId, block_number:u64 ) {

        log::debug!("replace_agent( netuid: {:?} | uid_to_replace: {:?} | new_computekey: {:?} ) ", netuid, uid_to_replace, new_computekey );

        // 1. Get the old computekey under this position.
        let old_computekey: T::AccountId = Keys::<T>::get( netuid, uid_to_replace );

        // 2. Remove previous set memberships.
        Uids::<T>::remove( netuid, old_computekey.clone() );
        IsNetworkMember::<T>::remove( old_computekey.clone(), netuid );
        Keys::<T>::remove( netuid, uid_to_replace );

        // 2a. Check if the uid is registered in any other brains.
        let computekey_is_registered_on_any_network: bool = Self::is_computekey_registered_on_any_network( &old_computekey.clone() );
        if !computekey_is_registered_on_any_network {
            // If not, unstake all personalkeys under this computekey.
            Self::unstake_all_personalkeys_from_computekey_account( &old_computekey.clone() );
        }

        // 3. Create new set memberships.
        Self::set_active_for_uid( netuid, uid_to_replace, true ); // Set to active by default.
        Keys::<T>::insert( netuid, uid_to_replace, new_computekey.clone() ); // Make computekey - uid association.
        Uids::<T>::insert( netuid, new_computekey.clone(), uid_to_replace ); // Make uid - computekey association.
        BlockAtRegistration::<T>::insert( netuid, uid_to_replace, block_number ); // Fill block at registration.
        IsNetworkMember::<T>::insert( new_computekey.clone(), netuid, true ); // Fill network is member.
    }

    // Appends the uid to the network.
    pub fn append_agent( netuid: u16, new_computekey: &T::AccountId, block_number:u64 ) {

        // 1. Get the next uid. This is always equal to brain_n.
        let next_uid: u16 = Self::get_brain_n( netuid );
        log::debug!("append_agent( netuid: {:?} | next_uid: {:?} | new_computekey: {:?} ) ", netuid, new_computekey, next_uid );

        // 2. Get and increase the uid count.
        BrainN::<T>::insert( netuid, next_uid + 1 );

        // 3. Expand Yuma Consensus with new position.
        Rank::<T>::mutate(netuid, |v| v.push(0) );
        Trust::<T>::mutate(netuid, |v| v.push(0) );
        Active::<T>::mutate(netuid, |v| v.push( true ) );
        Emission::<T>::mutate(netuid, |v| v.push(0) );
        Consensus::<T>::mutate(netuid, |v| v.push(0) );
        Incentive::<T>::mutate(netuid, |v| v.push(0) );
        Dividends::<T>::mutate(netuid, |v| v.push(0) );
        LastUpdate::<T>::mutate(netuid, |v| v.push( block_number ) );
        PruningScores::<T>::mutate(netuid, |v| v.push(0) );
        ValidatorTrust::<T>::mutate(netuid, |v| v.push(0) );
        ValidatorPermit::<T>::mutate(netuid, |v| v.push(false) );

        // 4. Insert new account information.
        Keys::<T>::insert( netuid, next_uid, new_computekey.clone() ); // Make computekey - uid association.
        Uids::<T>::insert( netuid, new_computekey.clone(), next_uid ); // Make uid - computekey association.
        BlockAtRegistration::<T>::insert( netuid, next_uid, block_number ); // Fill block at registration.
        IsNetworkMember::<T>::insert( new_computekey.clone(), netuid, true ); // Fill network is member.
    }

    // Returns true if the uid is set on the network.
    //
    pub fn is_uid_exist_on_network(netuid: u16, uid: u16) -> bool {
        return  Keys::<T>::contains_key(netuid, uid);
    }

    // Returns true if the computekey holds a slot on the network.
    //
    pub fn is_computekey_registered_on_network( netuid:u16, computekey: &T::AccountId ) -> bool {
        return Uids::<T>::contains_key( netuid, computekey )
    }

    // Returs the computekey under the network uid as a Result. Ok if the uid is taken.
    //
    pub fn get_computekey_for_net_and_uid( netuid: u16, agent_uid: u16) ->  Result<T::AccountId, DispatchError> {
        Keys::<T>::try_get(netuid, agent_uid).map_err(|_err| Error::<T>::NotRegistered.into())
    }

    // Returns the uid of the computekey in the network as a Result. Ok if the computekey has a slot.
    //
    pub fn get_uid_for_net_and_computekey( netuid: u16, computekey: &T::AccountId) -> Result<u16, DispatchError> {
        return Uids::<T>::try_get(netuid, &computekey).map_err(|_err| Error::<T>::NotRegistered.into())
    }

    // Returns the stake of the uid on network or 0 if it doesnt exist.
    //
    pub fn get_stake_for_uid_and_brain( netuid: u16, agent_uid: u16) -> u64 {
        if Self::is_uid_exist_on_network( netuid, agent_uid) {
            return Self::get_total_stake_for_computekey( &Self::get_computekey_for_net_and_uid( netuid, agent_uid ).unwrap() )
        } else {
            return 0;
        }
    }


    // Return the total number of brain available on the chain.
    //
    pub fn get_number_of_brains()-> u16 {
        let mut number_of_brains : u16 = 0;
        for (_, _)  in <BrainN<T> as IterableStorageMap<u16, u16>>::iter(){
            number_of_brains = number_of_brains + 1;
        }
        return number_of_brains;
    }

    // Return a list of all networks a computekey is registered on.
    //
    pub fn get_registered_networks_for_computekey( computekey: &T::AccountId )-> Vec<u16> {
        let mut all_networks: Vec<u16> = vec![];
        for ( network, is_registered)  in <IsNetworkMember<T> as IterableStorageDoubleMap< T::AccountId, u16, bool >>::iter_prefix( computekey ){
            if is_registered { all_networks.push( network ) }
        }
        all_networks
    }

    // Return true if a computekey is registered on any network.
    //
    pub fn is_computekey_registered_on_any_network( computekey: &T::AccountId )-> bool {
        for ( _, is_registered)  in <IsNetworkMember<T> as IterableStorageDoubleMap< T::AccountId, u16, bool >>::iter_prefix( computekey ){
            if is_registered { return true }
        }
        false
    }
}
