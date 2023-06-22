#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;

pub mod pallet;
pub mod weights;

pub use pallet::pallet::*;
pub use weights::WeightInfo;

/// Storage value representing the current Dapps staking pallet storage version.
/// Used by `on_runtime_upgrade` to determine whether a storage migration is needed or not.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Version {
    V1_0_0,
    V2_0_0,
    V3_0_0,
    V4_0_0,
}
