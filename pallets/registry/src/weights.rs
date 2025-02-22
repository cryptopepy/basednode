
//! Autogenerated weights for `pallet_registry`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-10-24, STEPS: `2`, REPEAT: `1`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `rustys-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("local")`, DB CACHE: `1024`

// Executed Command:
// ./target/release/basednode
// benchmark
// pallet
// --chain=local
// --execution=wasm
// --wasm-execution=compiled
// --pallet=pallet_registry
// --extrinsic=*
// --output=pallets/registry/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pallet_registry`.
pub trait WeightInfo {
	fn set_identity() -> Weight;
	fn clear_identity() -> Weight;
}

/// Weights for `pallet_registry` using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: Registry IdentityOf (r:1 w:1)
	/// Proof Skipped: Registry IdentityOf (max_values: None, max_size: None, mode: Measured)
	fn set_identity() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1024`
		//  Estimated: `3499`
		// Minimum execution time: 41_000_000 picoseconds.
		Weight::from_parts(41_000_000, 3499)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: Registry IdentityOf (r:1 w:1)
	/// Proof Skipped: Registry IdentityOf (max_values: None, max_size: None, mode: Measured)
	fn clear_identity() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1385`
		//  Estimated: `3860`
		// Minimum execution time: 36_000_000 picoseconds.
		Weight::from_parts(36_000_000, 3860)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: Registry IdentityOf (r:1 w:1)
	/// Proof Skipped: Registry IdentityOf (max_values: None, max_size: None, mode: Measured)
	fn set_identity() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1024`
		//  Estimated: `3499`
		// Minimum execution time: 41_000_000 picoseconds.
		Weight::from_parts(41_000_000, 3499)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: Registry IdentityOf (r:1 w:1)
	/// Proof Skipped: Registry IdentityOf (max_values: None, max_size: None, mode: Measured)
	fn clear_identity() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1385`
		//  Estimated: `3860`
		// Minimum execution time: 36_000_000 picoseconds.
		Weight::from_parts(36_000_000, 3860)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}