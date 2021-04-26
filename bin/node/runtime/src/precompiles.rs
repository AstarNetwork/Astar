#![cfg_attr(not(feature = "std"), no_std)]

use evm::{Context, ExitError, ExitSucceed};
use pallet_evm::{Config, Precompile, PrecompileSet};
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use sp_core::H160;
use sp_std::{marker::PhantomData, vec::Vec};

#[derive(Debug, Clone, Copy)]
pub struct PlasmPrecompiles<R>(PhantomData<R>);

impl<R> PrecompileSet for PlasmPrecompiles<R>
where
    R: Config,
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
            _ => None,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
