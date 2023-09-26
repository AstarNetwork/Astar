// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! The Local EVM precompiles. This can be compiled with ``#[no_std]`, ready for Wasm.

use crate::RuntimeCall;
use astar_primitives::precompiles::DispatchFilterValidate;
use frame_support::traits::Contains;
use pallet_evm::{
    ExitRevert, IsPrecompileResult, Precompile, PrecompileFailure, PrecompileHandle,
    PrecompileResult, PrecompileSet,
};
use pallet_evm_precompile_assets_erc20::{AddressToAssetId, Erc20AssetsPrecompileSet};
use pallet_evm_precompile_batch::BatchPrecompile;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_dapps_staking::DappsStakingWrapper;
use pallet_evm_precompile_dispatch::Dispatch;
use pallet_evm_precompile_ed25519::Ed25519Verify;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompile_sr25519::Sr25519Precompile;
use pallet_evm_precompile_substrate_ecdsa::SubstrateEcdsaPrecompile;
use pallet_evm_precompile_xvm::XvmPrecompile;
use sp_core::H160;
use sp_std::fmt::Debug;
use sp_std::marker::PhantomData;

/// The asset precompile address prefix. Addresses that match against this prefix will be routed
/// to Erc20AssetsPrecompileSet
pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];

/// Filter that only allows whitelisted runtime call to pass through dispatch precompile
pub struct WhitelistedCalls;

impl Contains<RuntimeCall> for WhitelistedCalls {
    fn contains(t: &RuntimeCall) -> bool {
        match t {
            RuntimeCall::Utility(pallet_utility::Call::batch { calls })
            | RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) => {
                calls.iter().all(|call| WhitelistedCalls::contains(call))
            }
            RuntimeCall::Assets(pallet_assets::Call::transfer { .. }) => true,
            RuntimeCall::DappsStaking(_) => true,
            _ => false,
        }
    }
}

/// The PrecompileSet installed in the Local runtime.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalNetworkPrecompiles<R>(PhantomData<R>);

impl<R> LocalNetworkPrecompiles<R> {
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Return all addresses that contain precompiles. This can be used to populate dummy code
    /// under the precompile.
    pub fn used_addresses() -> impl Iterator<Item = H160> {
        sp_std::vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 1024, 1025, 1026, 1027, 20481, 20482, 20483, 20485, 20846
        ]
        .into_iter()
        .map(hash)
    }
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet
impl<R> PrecompileSet for LocalNetworkPrecompiles<R>
where
    Erc20AssetsPrecompileSet<R>: PrecompileSet,
    DappsStakingWrapper<R>: Precompile,
    BatchPrecompile<R>: Precompile,
    XvmPrecompile<R, pallet_xvm::Pallet<R>>: Precompile,
    Dispatch<R, DispatchFilterValidate<RuntimeCall, WhitelistedCalls>>: Precompile,
    R: pallet_evm::Config
        + pallet_xvm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        let address = handle.code_address();
        if let IsPrecompileResult::Answer { is_precompile, .. } =
            self.is_precompile(address, u64::MAX)
        {
            if is_precompile && address > hash(9) && handle.context().address != address {
                return Some(Err(PrecompileFailure::Revert {
                    exit_status: ExitRevert::Reverted,
                    output: b"cannot be called with DELEGATECALL or CALLCODE".to_vec(),
                }));
            }
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
            a if a == hash(1025) => Some(Dispatch::<
                R,
                DispatchFilterValidate<RuntimeCall, WhitelistedCalls>,
            >::execute(handle)),
            a if a == hash(1026) => Some(ECRecoverPublicKey::execute(handle)),
            a if a == hash(1027) => Some(Ed25519Verify::execute(handle)),
            // Astar precompiles (starts from 0x5000):
            // DappStaking 0x5001
            a if a == hash(20481) => Some(DappsStakingWrapper::<R>::execute(handle)),
            // Sr25519 0x5002
            a if a == hash(20482) => Some(Sr25519Precompile::<R>::execute(handle)),
            // SubstrateEcdsa 0x5003
            a if a == hash(20483) => Some(SubstrateEcdsaPrecompile::<R>::execute(handle)),
            // Xvm 0x5005
            a if a == hash(20485) => {
                Some(XvmPrecompile::<R, pallet_xvm::Pallet<R>>::execute(handle))
            }
            // Batch 0x5006
            a if a == hash(20486) => Some(BatchPrecompile::<R>::execute(handle)),

            // If the address matches asset prefix, the we route through the asset precompile set
            a if &a.to_fixed_bytes()[0..4] == ASSET_PRECOMPILE_ADDRESS_PREFIX => {
                Erc20AssetsPrecompileSet::<R>::new().execute(handle)
            }
            // Default
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().any(|x| x == address),
            extra_cost: 0,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
