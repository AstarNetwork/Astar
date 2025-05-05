#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for vesting_mbm.
pub trait WeightInfo {
    fn step() -> Weight;
}

/// Weights for vesting_mbm using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    /// Storage: `Vesting::Vesting` (r:2 w:1)
    /// Proof: `Vesting::Vesting` (`max_values`: None, `max_size`: Some(1057), added: 3532, mode: `MaxEncodedLen`)
    /// The range of component `x` is `[1, 28]`.
    fn step() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `316 + x * (36 ±0)`
        //  Estimated: `8054`
        // Minimum execution time: 13_079_000 picoseconds.
        Weight::from_parts(13_513_131, 8054)
            // Standard Error: 723
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    /// Storage: `Vesting::Vesting` (r:2 w:1)
    /// Proof: `Vesting::Vesting` (`max_values`: None, `max_size`: Some(1057), added: 3532, mode: `MaxEncodedLen`)
    /// The range of component `x` is `[1, 28]`.
    fn step() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `316 + x * (36 ±0)`
        //  Estimated: `8054`
        // Minimum execution time: 13_079_000 picoseconds.
        Weight::from_parts(13_513_131, 8054)
            // Standard Error: 723
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
}
