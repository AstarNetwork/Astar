#![cfg_attr(not(feature = "std"), no_std)]

use codec::Decode;
use evm::{Context, ExitError, ExitSucceed};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{Precompile, PrecompileSet};
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_dispatch::Dispatch;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use sp_core::H160;
use sp_std::{marker::PhantomData, vec::Vec};

mod nicks;
use nicks::Nicks;

mod contracts;
use contracts::Contracts;

#[derive(Debug, Clone, Copy)]
pub struct AstarPrecompiles<R>(PhantomData<R>);

impl<R> PrecompileSet for AstarPrecompiles<R>
where
    R: pallet_evm::Config + pallet_nicks::Config + pallet_contracts::Config,
    R::Call: Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo
        + Decode
        + From<pallet_nicks::Call<R>>
        + From<pallet_contracts::Call<R>>,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    R::AccountId: AsRef<[u8]> + sp_core::crypto::UncheckedFrom<R::Hash>,
    R::Hash: From<sp_core::H256>,
{
    fn execute(
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
    ) -> Option<Result<(ExitSucceed, Vec<u8>, u64), ExitError>> {
        match address {
            // Ethereum precompiles
            a if a == hash(1) => Some(ECRecover::execute(input, target_gas, context)),
            a if a == hash(2) => Some(Sha256::execute(input, target_gas, context)),
            a if a == hash(3) => Some(Ripemd160::execute(input, target_gas, context)),
            a if a == hash(4) => Some(Identity::execute(input, target_gas, context)),
            a if a == hash(5) => Some(Modexp::execute(input, target_gas, context)),
            a if a == hash(6) => Some(Bn128Add::execute(input, target_gas, context)),
            a if a == hash(7) => Some(Bn128Mul::execute(input, target_gas, context)),
            a if a == hash(8) => Some(Bn128Pairing::execute(input, target_gas, context)),
            // Non Ethereum precompiles
            a if a == hash(1024) => Some(Dispatch::<R>::execute(input, target_gas, context)),
            // Astar precompiles
            a if a == hash(4096) => Some(Contracts::<R>::execute(input, target_gas, context)),
            a if a == hash(4097) => Some(Nicks::<R>::execute(input, target_gas, context)),
            _ => None,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
