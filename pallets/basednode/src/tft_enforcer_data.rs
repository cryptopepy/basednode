use super::*;
use substrate_fixed::types::U64F64;
use frame_support::IterableStorageDoubleMap;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use sp_core::hexdisplay::AsBytesRef;
use codec::Compact;
use sp_runtime::{SaturatedConversion, Saturating, traits::{Zero, Block as BlockT, Header as HeaderT, NumberFor}};


#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct TftEnforcerDataBlock<T: Config> {
	block_number: Compact<u64>,
	block_hash: Vec<u8>,
	parent_hash: Vec<u8>,
	state_root: Vec<u8>,
	extrinsics_root: Vec<u8>,
	author: T::AccountId,
	validators: Compact<u16>,
	validator_stakes: Compact<u128>
}

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct TftEnforcerData<T: Config> {
	block_data: Vec<TftEnforcerDataBlock<T>>
}


impl<T: Config> Pallet<T> {
	pub fn get_tft_enforcer_data(from_block: Vec<u8>, block_count: Option<u64>) -> TftEnforcerData<T> {
        let block_count = block_count.unwrap_or(10);
		let mut result = Vec::new();
		let mut current_hash = T::Hash::decode(&mut from_block.as_bytes_ref()).unwrap();
		let current_block_number = frame_system::Pallet::<T>::block_number();

		for i in 1..block_count + 1 {
			let block_number = current_block_number.saturating_sub(SaturatedConversion::saturated_from::<u32>(i.try_into().unwrap()));
			let hash = frame_system::Pallet::<T>::block_hash(block_number);
			log::debug!("hash = {:?}, block_number = {:?}", hash, block_number);

			let mut validator_stakes = TotalComputekeyStake::<T>::iter_values().sum::<u64>() as u128;
			let mut validators = BrainN::<T>::iter_values().sum::<u16>() as u16;

			result.push(TftEnforcerDataBlock {
				block_number: block_number.saturated_into::<u64>().into(),
				block_hash: hash.clone().encode(),
				parent_hash: frame_system::Pallet::<T>::parent_hash().encode(),
				state_root: vec![],
				extrinsics_root: vec![],
				author: BrainOwner::<T>::get(0),
				validators: validators.into(),
				validator_stakes: validator_stakes.into()
			});
			if block_number == T::BlockNumber::zero() {
				// We've reached the genesis block
				break;
			}
		}
		TftEnforcerData { block_data: result }
	}

}
