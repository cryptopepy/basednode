use super::*;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;
use sp_core::hexdisplay::AsBytesRef;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct StakeInfo<T: Config> {
    computekey: T::AccountId,
    personalkey: T::AccountId,
    stake: Compact<u64>,
}

impl<T: Config> Pallet<T> {
    fn _get_stake_info_for_personalkeys(
        personalkeys: Vec<T::AccountId>,
    ) -> Vec<(T::AccountId, Vec<StakeInfo<T>>)> {
        if personalkeys.len() == 0 {
            return Vec::new(); // No personalkeys to check
        }

        let mut stake_info: Vec<(T::AccountId, Vec<StakeInfo<T>>)> = Vec::new();
        for personalkey_ in personalkeys {
            let mut stake_info_for_personalkey: Vec<StakeInfo<T>> = Vec::new();

            for (computekey, personalkey, stake) in <Stake<T>>::iter() {
                if personalkey == personalkey_ {
                    stake_info_for_personalkey.push(StakeInfo {
                        computekey,
                        personalkey,
                        stake: stake.into(),
                    });
                }
            }

            stake_info.push((personalkey_, stake_info_for_personalkey));
        }

        return stake_info;
    }

    pub fn get_stake_info_for_personalkeys(
        personalkey_account_vecs: Vec<Vec<u8>>,
    ) -> Vec<(T::AccountId, Vec<StakeInfo<T>>)> {
        let mut personalkeys: Vec<T::AccountId> = Vec::new();
        for personalkey_account_vec in personalkey_account_vecs {
            if personalkey_account_vec.len() != 32 {
                continue; // Invalid personalkey
            }
            let personalkey: AccountIdOf<T> =
                T::AccountId::decode(&mut personalkey_account_vec.as_bytes_ref()).unwrap();
            personalkeys.push(personalkey);
        }

        if personalkeys.len() == 0 {
            return Vec::new(); // Invalid personalkey
        }

        let stake_info = Self::_get_stake_info_for_personalkeys(personalkeys);

        return stake_info;
    }

    pub fn get_stake_info_for_personalkey(personalkey_account_vec: Vec<u8>) -> Vec<StakeInfo<T>> {
        if personalkey_account_vec.len() != 32 {
            return Vec::new(); // Invalid personalkey
        }

        let personalkey: AccountIdOf<T> =
            T::AccountId::decode(&mut personalkey_account_vec.as_bytes_ref()).unwrap();
        let stake_info = Self::_get_stake_info_for_personalkeys(vec![personalkey]);

        if stake_info.len() == 0 {
            return Vec::new(); // Invalid personalkey
        } else {
            return stake_info.get(0).unwrap().1.clone();
        }
    }
}
