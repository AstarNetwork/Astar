#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_dapp_staking_v3.
pub trait WeightInfo {
	fn execute_call() -> Weight;

}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn execute_call() -> Weight {
		Weight::from_parts(1_000_000, 0)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn execute_call() -> Weight {
		Weight::from_parts(1_000_000, 0)
	}
}
