//! The Shiden Network EVM precompiles. This can be compiled with ``#[no_std]`, ready for Wasm.

use pallet_evm::{
    ExitRevert, Precompile, PrecompileFailure, PrecompileHandle, PrecompileResult, PrecompileSet,
};
use pallet_evm_precompile_assets_erc20::Erc20AssetsPrecompileSet;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_dispatch::Dispatch;
use pallet_evm_precompile_ed25519::Ed25519Verify;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompile_sr25519::Sr25519Precompile;
use pallet_evm_precompile_substrate_ecdsa::SubstrateEcdsaPrecompile;
use pallet_precompile_dapps_staking::DappsStakingWrapper;
use sp_core::H160;
use sp_std::fmt::Debug;
use sp_std::marker::PhantomData;

/// The asset precompile address prefix. Addresses that match against this prefix will be routed
/// to Erc20AssetsPrecompileSet
pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];

/// The PrecompileSet installed in the Shiden runtime.
#[derive(Debug, Clone, Copy)]
pub struct ShidenNetworkPrecompiles<R>(PhantomData<R>);

impl<R> ShidenNetworkPrecompiles<R> {
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Return all addresses that contain precompiles. This can be used to populate dummy code
    /// under the precompile.
    pub fn used_addresses() -> impl Iterator<Item = H160> {
        sp_std::vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 1024, 1025, 1026, 1027, 20481, 20482, 20483]
            .into_iter()
            .map(|x| hash(x))
    }
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet
impl<R> PrecompileSet for ShidenNetworkPrecompiles<R>
where
    R: pallet_evm::Config,
    Dispatch<R>: Precompile,
    DappsStakingWrapper<R>: Precompile,
    Erc20AssetsPrecompileSet<R>: PrecompileSet,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        let address = handle.code_address();
        if self.is_precompile(address) && address > hash(9) && handle.context().address != address {
            return Some(Err(PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: b"cannot be called with DELEGATECALL or CALLCODE".to_vec(),
            }));
        }
        match address {
            // Ethereum precompiles :
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            a if a == hash(5) => Some(Modexp::execute(handle)),
            a if a == hash(6) => Some(Bn128Add::execute(handle)),
            a if a == hash(7) => Some(Bn128Mul::execute(handle)),
            a if a == hash(8) => Some(Bn128Pairing::execute(handle)),
            a if a == hash(9) => Some(Blake2F::execute(handle)),
            // nor Ethereum precompiles :
            a if a == hash(1024) => Some(Sha3FIPS256::execute(handle)),
            a if a == hash(1025) => Some(Dispatch::<R>::execute(handle)),
            a if a == hash(1026) => Some(ECRecoverPublicKey::execute(handle)),
            a if a == hash(1027) => Some(Ed25519Verify::execute(handle)),
            // Astar precompiles (starts from 0x5000):
            // DappStaking 0x5001
            a if a == hash(20481) => Some(DappsStakingWrapper::<R>::execute(handle)),
            // Sr25519     0x5002
            a if a == hash(20482) => Some(Sr25519Precompile::<R>::execute(handle)),
            // SubstrateEcdsa 0x5003
            a if a == hash(20483) => Some(SubstrateEcdsaPrecompile::<R>::execute(handle)),
            // If the address matches asset prefix, the we route through the asset precompile set
            a if &a.to_fixed_bytes()[0..4] == ASSET_PRECOMPILE_ADDRESS_PREFIX => {
                Erc20AssetsPrecompileSet::<R>::new().execute(handle)
            }
            // Default
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160) -> bool {
        Self::used_addresses().find(|x| x == &address).is_some()
            || Erc20AssetsPrecompileSet::<R>::new().is_precompile(address)
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
