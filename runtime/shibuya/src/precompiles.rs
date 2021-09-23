//! The Shibuya Network EVM precompiles. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Decode;
use evm::{executor::PrecompileOutput, Context, ExitError};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{Precompile, PrecompileSet};
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_dispatch::Dispatch;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_precompile_staking::Staking;
use sp_core::H160;
use sp_std::fmt::Debug;
use sp_std::marker::PhantomData;

/// The PrecompileSet installed in the Shiden runtime.
#[derive(Debug, Clone, Copy)]
pub struct ShibuyaNetworkPrecompiles<R>(PhantomData<R>);

impl<R> ShibuyaNetworkPrecompiles<R> {
    /// Return all addresses that contain precompiles. This can be used to populate dummy code
    /// under the precompile.
    pub fn used_addresses<AccountId: From<H160>>() -> impl Iterator<Item = AccountId> {
        sp_std::vec![1, 2, 3, 4, 5, 6, 7, 8, 1024, 1025, 1026, 20480]
            .into_iter()
            .map(|x| hash(x).into())
    }
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet
impl<R> PrecompileSet for ShibuyaNetworkPrecompiles<R>
where
    R: pallet_evm::Config + pallet_session::Config + pallet_collator_selection::Config,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    R::Call: From<pallet_session::Call<R>>
        + From<pallet_collator_selection::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo
        + Decode,
{
    fn execute(
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
    ) -> Option<Result<PrecompileOutput, ExitError>> {
        match address {
            // Ethereum precompiles :
            a if a == hash(1) => Some(ECRecover::execute(input, target_gas, context)),
            a if a == hash(2) => Some(Sha256::execute(input, target_gas, context)),
            a if a == hash(3) => Some(Ripemd160::execute(input, target_gas, context)),
            a if a == hash(4) => Some(Identity::execute(input, target_gas, context)),
            a if a == hash(5) => Some(Modexp::execute(input, target_gas, context)),
            a if a == hash(6) => Some(Bn128Add::execute(input, target_gas, context)),
            a if a == hash(7) => Some(Bn128Mul::execute(input, target_gas, context)),
            a if a == hash(8) => Some(Bn128Pairing::execute(input, target_gas, context)),
            // nor Ethereum precompiles :
            a if a == hash(1024) => Some(Sha3FIPS256::execute(input, target_gas, context)),
            a if a == hash(1025) => Some(Dispatch::<R>::execute(input, target_gas, context)),
            a if a == hash(1026) => Some(ECRecoverPublicKey::execute(input, target_gas, context)),
            // Astar precompiles (starts from 0x5000):
            a if a == hash(20480) => Some(Staking::<R>::execute(input, target_gas, context)),
            // Default
            _ => None,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
