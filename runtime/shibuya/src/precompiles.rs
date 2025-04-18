// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

//! The Astar Network EVM precompiles. This can be compiled with ``#[no_std]`, ready for Wasm.

use crate::{RuntimeCall, UnifiedAccounts};
use astar_primitives::precompiles::DispatchFilterValidate;
use frame_support::traits::ConstU32;
use frame_support::{parameter_types, traits::Contains};
use pallet_evm_precompile_assets_erc20::Erc20AssetsPrecompileSet;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_dapp_staking::DappStakingV3Precompile;
use pallet_evm_precompile_dispatch::Dispatch;
use pallet_evm_precompile_dispatch_lockdrop::DispatchLockdrop;
use pallet_evm_precompile_ed25519::Ed25519Verify;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompile_sr25519::Sr25519Precompile;
use pallet_evm_precompile_substrate_ecdsa::SubstrateEcdsaPrecompile;
use pallet_evm_precompile_unified_accounts::UnifiedAccountsPrecompile;
use pallet_evm_precompile_xcm::XcmPrecompile;
use precompile_utils::precompile_set::*;
use sp_std::fmt::Debug;

/// The asset precompile address prefix. Addresses that match against this prefix will be routed
/// to Erc20AssetsPrecompileSet
pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];
parameter_types! {
    pub AssetPrefix: &'static [u8] = ASSET_PRECOMPILE_ADDRESS_PREFIX;
}

/// Precompile checks for ethereum spec precompiles
/// We allow DELEGATECALL to stay compliant with Ethereum behavior.
type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);

/// Filter that only allows whitelisted runtime call to pass through dispatch precompile
pub struct WhitelistedCalls;

impl Contains<RuntimeCall> for WhitelistedCalls {
    fn contains(t: &RuntimeCall) -> bool {
        match t {
            RuntimeCall::Utility(pallet_utility::Call::batch { calls })
            | RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) => {
                calls.iter().all(|call| WhitelistedCalls::contains(call))
            }
            RuntimeCall::DappStaking(_) => true,
            RuntimeCall::Assets(pallet_assets::Call::transfer { .. }) => true,
            RuntimeCall::XTokens(orml_xtokens::Call::transfer_multiasset_with_fee { .. }) => true,
            RuntimeCall::XTokens(orml_xtokens::Call::transfer_multiasset { .. }) => true,
            // Governance related calls
            RuntimeCall::Democracy(_)
            | RuntimeCall::Treasury(_)
            | RuntimeCall::CommunityTreasury(_)
            | RuntimeCall::Preimage(_) => true,
            _ => false,
        }
    }
}

/// Filter that only allows whitelisted runtime call to pass through dispatch-lockdrop precompile
pub struct WhitelistedLockdropCalls;

impl Contains<RuntimeCall> for WhitelistedLockdropCalls {
    fn contains(t: &RuntimeCall) -> bool {
        match t {
            RuntimeCall::Utility(pallet_utility::Call::batch { calls })
            | RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) => calls
                .iter()
                .all(|call| WhitelistedLockdropCalls::contains(call)),
            RuntimeCall::DappStaking(pallet_dapp_staking::Call::unbond_and_unstake { .. }) => true,
            RuntimeCall::DappStaking(pallet_dapp_staking::Call::withdraw_unbonded { .. }) => true,
            RuntimeCall::Balances(pallet_balances::Call::transfer_all { .. }) => true,
            RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive { .. }) => true,
            RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death { .. }) => true,
            RuntimeCall::Assets(pallet_assets::Call::transfer { .. }) => true,
            _ => false,
        }
    }
}

/// The PrecompileSet installed in the Shibuya runtime.
#[precompile_utils::precompile_name_from_address]
pub type ShibuyaPrecompilesSetAt<R, C> = (
    // Ethereum precompiles:
    // We allow DELEGATECALL to stay compliant with Ethereum behavior.
    PrecompileAt<AddressU64<1>, ECRecover, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<2>, Sha256, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<3>, Ripemd160, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<4>, Identity, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<5>, Modexp, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<6>, Bn128Add, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<7>, Bn128Mul, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<8>, Bn128Pairing, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<9>, Blake2F, EthereumPrecompilesChecks>,
    // Non-Astar specific nor Ethereum precompiles :
    PrecompileAt<AddressU64<1024>, Sha3FIPS256, (CallableByContract, CallableByPrecompile)>,
    PrecompileAt<
        AddressU64<1025>,
        Dispatch<R, DispatchFilterValidate<RuntimeCall, WhitelistedCalls>>,
        // Not callable from smart contract nor precompiles, only EOA accounts
        (),
    >,
    PrecompileAt<AddressU64<1026>, ECRecoverPublicKey, (CallableByContract, CallableByPrecompile)>,
    PrecompileAt<AddressU64<1027>, Ed25519Verify, (CallableByContract, CallableByPrecompile)>,
    // Astar specific precompiles:
    PrecompileAt<
        AddressU64<20481>,
        DappStakingV3Precompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<20482>,
        Sr25519Precompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<20483>,
        SubstrateEcdsaPrecompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<20484>,
        XcmPrecompile<R, C>,
        (
            SubcallWithMaxNesting<1>,
            CallableByContract,
            CallableByPrecompile,
        ),
    >,
    // Skipping 20485 to make sure all network have consistent precompiles address
    PrecompileAt<
        AddressU64<20486>,
        UnifiedAccountsPrecompile<R, UnifiedAccounts>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<20487>,
        DispatchLockdrop<
            R,
            DispatchFilterValidate<RuntimeCall, WhitelistedLockdropCalls>,
            ConstU32<8>,
        >,
        // Not callable from smart contract nor precompiled, only EOA accounts
        (),
    >,
);

pub type ShibuyaPrecompiles<R, C> = PrecompileSetBuilder<
    R,
    (
        // Skip precompiles if out of range.
        PrecompilesInRangeInclusive<
            // TODO: what is the range for precompiles sets 1 - ?
            (AddressU64<1>, AddressU64<40951>),
            ShibuyaPrecompilesSetAt<R, C>,
        >,
        // Prefixed precompile sets (XC20)
        PrecompileSetStartingWith<AssetPrefix, Erc20AssetsPrecompileSet<R>, CallableByContract>,
    ),
>;
