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

//! Astar dApps staking interface.

#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};

use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::{Currency, Get},
};
use pallet_dapps_staking::RewardDestination;
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{ConstU32, H160, U256};
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
extern crate alloc;

type BalanceOf<Runtime> = <<Runtime as pallet_dapps_staking::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

pub const STAKER_BYTES_LIMIT: u32 = 32;
type GetStakerBytesLimit = ConstU32<STAKER_BYTES_LIMIT>;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// This is only used to encode SmartContract enum
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum Contract<A> {
    /// EVM smart contract instance.
    Evm(H160),
    /// Wasm smart contract instance. Not used in this precompile
    Wasm(A),
}

pub struct DappsStakingWrapper<R>(PhantomData<R>);

#[precompile_utils::precompile]
impl<R> DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    BalanceOf<R>: solidity::Codec,
    <R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
    R::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    R::RuntimeCall: From<pallet_dapps_staking::Call<R>>,
    R::AccountId: From<[u8; 32]>,
{
    /// Fetch current era from CurrentEra storage map
    #[precompile::public("read_current_era()")]
    #[precompile::view]
    fn read_current_era(handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: CurrentEra:
        // Twox64(8) + EraIndex(4)
        handle.record_db_read::<R>(12)?;

        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();
        Ok(current_era.into())
    }

    /// Fetch unbonding period
    #[precompile::public("read_unbonding_period()")]
    #[precompile::view]
    fn read_unbonding_period(_: &mut impl PrecompileHandle) -> EvmResult<U256> {
        // constant, no DB read
        let unbonding_period = R::UnbondingPeriod::get();

        Ok(unbonding_period.into())
    }

    /// Fetch reward from EraRewardsAndStakes storage map
    #[precompile::public("read_era_reward(uint32)")]
    #[precompile::view]
    fn read_era_reward(handle: &mut impl PrecompileHandle, era: u32) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: GeneralEraInfo:
        // Twox64Concat(8) + EraIndex(4) + EraInfo::max_encoded_len
        handle.record_db_read::<R>(12 + pallet_dapps_staking::EraInfo::max_encoded_len())?;

        // call pallet-dapps-staking
        let read_reward = pallet_dapps_staking::GeneralEraInfo::<R>::get(era);
        let reward = read_reward.map_or(Zero::zero(), |r| {
            r.rewards.stakers.saturating_add(r.rewards.dapps)
        });

        Ok(reward.into())
    }

    /// Fetch total staked amount from EraRewardsAndStakes storage map
    #[precompile::public("read_era_staked(uint32)")]
    #[precompile::view]
    fn read_era_staked(handle: &mut impl PrecompileHandle, era: u32) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: GeneralEraInfo:
        // Twox64Concat(8) + EraIndex(4) + EraInfo::max_encoded_len
        handle.record_db_read::<R>(12 + pallet_dapps_staking::EraInfo::max_encoded_len())?;

        // call pallet-dapps-staking
        let reward_and_stake = pallet_dapps_staking::GeneralEraInfo::<R>::get(era);
        // compose output
        let staked = reward_and_stake.map_or(Zero::zero(), |r| r.staked);
        let staked = TryInto::<u128>::try_into(staked).unwrap_or(0);

        Ok(staked.into())
    }

    /// Fetch Ledger storage map for an account
    #[precompile::public("read_staked_amount(bytes)")]
    #[precompile::view]
    fn read_staked_amount(
        handle: &mut impl PrecompileHandle,
        staker: BoundedBytes<GetStakerBytesLimit>,
    ) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: Ledger:
        // Blake2_128Concat(16 + 32) + Ledger::max_encoded_len
        handle.record_db_read::<R>(48 + pallet_dapps_staking::AccountLedger::max_encoded_len())?;

        // parse input parameters for pallet-dapps-staking call
        let staker = Self::parse_input_address(staker.into())?;

        // call pallet-dapps-staking
        let ledger = pallet_dapps_staking::Ledger::<R>::get(&staker);
        log::trace!(target: "ds-precompile", "read_staked_amount for account:{:?}, ledger.locked:{:?}", staker, ledger.locked);

        Ok(ledger.locked.into())
    }

    /// Read GeneralStakerInfo for account/contract
    #[precompile::public("read_staked_amount_on_contract(address,bytes)")]
    #[precompile::view]
    fn read_staked_amount_on_contract(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        staker: BoundedBytes<GetStakerBytesLimit>,
    ) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: GeneralStakerInfo:
        // Blake2_128Concat(16 + 32) + Blake2_128Concat(16 + SmartContract::max_encoded_len) + StakerInfo::max_encoded_len
        handle.record_db_read::<R>(
            64 + <R as pallet_dapps_staking::Config>::SmartContract::max_encoded_len()
                + pallet_dapps_staking::StakerInfo::max_encoded_len(),
        )?;

        // parse contract address
        let contract_id = Self::decode_smart_contract(contract_h160.into())?;

        // parse input parameters for pallet-dapps-staking call
        let staker = Self::parse_input_address(staker.into())?;

        // call pallet-dapps-staking
        let staking_info = pallet_dapps_staking::GeneralStakerInfo::<R>::get(&staker, &contract_id);
        let staked_amount = staking_info.latest_staked_value();
        log::trace!(target: "ds-precompile", "read_staked_amount_on_contract for account:{:?}, contract: {:?} => staked_amount:{:?}", staker, contract_id, staked_amount);

        Ok(staked_amount.into())
    }

    /// Read the amount staked on contract in the given era
    #[precompile::public("read_contract_stake(address)")]
    #[precompile::view]
    fn read_contract_stake(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
    ) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: CurrentEra:
        // Twox64(8) + EraIndex(4)
        handle.record_db_read::<R>(16)?;
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ContractEraStake:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len) + Twox64Concat(8 + 4) + ContractStakeInfo::max_encoded_len
        handle.record_db_read::<R>(
            28 + <R as pallet_dapps_staking::Config>::SmartContract::max_encoded_len()
                + pallet_dapps_staking::ContractStakeInfo::max_encoded_len(),
        )?;

        let contract_id = Self::decode_smart_contract(contract_h160.into())?;
        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();

        // call pallet-dapps-staking
        let staking_info =
            pallet_dapps_staking::Pallet::<R>::contract_stake_info(&contract_id, current_era)
                .unwrap_or_default();

        // encode output with total
        let total = TryInto::<u128>::try_into(staking_info.total).unwrap_or(0);
        log::trace!(target: "ds-precompile", "read_contract_stake for contract: {:?} => staked_amount:{:?}", contract_id, total);
        Ok(total.into())
    }

    /// Register contract with the dapp-staking pallet
    /// Register is root origin only. This should always fail when called via evm precompile.
    #[precompile::public("register(address)")]
    fn register(_: &mut impl PrecompileHandle, _address: Address) -> EvmResult<bool> {
        // register is root-origin call. it should always fail when called via evm precompiles.
        Err(RevertReason::custom("register via evm precompile is not allowed").into())
    }

    /// Lock up and stake balance of the origin account.
    #[precompile::public("bond_and_stake(address,uint128)")]
    fn bond_and_stake(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        value: u128,
    ) -> EvmResult<bool> {
        // parse contract's address
        let contract_id = Self::decode_smart_contract(contract_h160.into())?;

        log::trace!(target: "ds-precompile", "bond_and_stake {:?}, {:?}", contract_id, value);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::bond_and_stake { contract_id, value };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Start unbonding process and unstake balance from the contract.
    #[precompile::public("unbond_and_unstake(address,uint128)")]
    fn unbond_and_unstake(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        value: u128,
    ) -> EvmResult<bool> {
        // parse contract's address
        let contract_id = Self::decode_smart_contract(contract_h160.into())?;

        log::trace!(target: "ds-precompile", "unbond_and_unstake {:?}, {:?}", contract_id, value);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::unbond_and_unstake { contract_id, value };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Start unbonding process and unstake balance from the contract.
    #[precompile::public("withdraw_unbonded()")]
    fn withdraw_unbonded(handle: &mut impl PrecompileHandle) -> EvmResult<bool> {
        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::withdraw_unbonded {};

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    #[precompile::public("claim_dapp(address,uint128)")]
    fn claim_dapp(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        era: u128,
    ) -> EvmResult<bool> {
        // parse contract's address
        let contract_id = Self::decode_smart_contract(contract_h160.into())?;

        // parse era
        let era = era
            .try_into()
            .map_err::<Revert, _>(|_| RevertReason::value_is_too_large("era type").into())
            .in_field("era")?;

        log::trace!(target: "ds-precompile", "claim_dapp {:?}, era {:?}", contract_id, era);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::claim_dapp { contract_id, era };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    #[precompile::public("claim_staker(address)")]
    fn claim_staker(handle: &mut impl PrecompileHandle, contract_h160: Address) -> EvmResult<bool> {
        // parse contract's address
        let contract_id = Self::decode_smart_contract(contract_h160.into())?;
        log::trace!(target: "ds-precompile", "claim_staker {:?}", contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::claim_staker { contract_id };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Set claim reward destination for the caller
    #[precompile::public("set_reward_destination(uint8)")]
    fn set_reward_destination(
        handle: &mut impl PrecompileHandle,
        reward_destination_raw: u8,
    ) -> EvmResult<bool> {
        // Transform raw value into dapps staking enum
        let reward_destination = if reward_destination_raw == 0 {
            RewardDestination::FreeBalance
        } else if reward_destination_raw == 1 {
            RewardDestination::StakeBalance
        } else {
            return Err(RevertReason::custom("Unexpected reward destination value.").into());
        };

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        log::trace!(target: "ds-precompile", "set_reward_destination {:?} {:?}", origin, reward_destination);

        let call = pallet_dapps_staking::Call::<R>::set_reward_destination { reward_destination };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Withdraw staked funds from the unregistered contract
    #[precompile::public("withdraw_from_unregistered(address)")]
    fn withdraw_from_unregistered(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
    ) -> EvmResult<bool> {
        // parse contract's address
        let contract_id = Self::decode_smart_contract(contract_h160.into())?;
        log::trace!(target: "ds-precompile", "withdraw_from_unregistered {:?}", contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::withdraw_from_unregistered { contract_id };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    #[precompile::public("nomination_transfer(address,uint128,address)")]
    fn nomination_transfer(
        handle: &mut impl PrecompileHandle,
        origin_contract_h160: Address,
        value: u128,
        target_contract_h160: Address,
    ) -> EvmResult<bool> {
        // parse origin contract's address
        let origin_contract_id = Self::decode_smart_contract(origin_contract_h160.into())?;

        // parse target contract's address
        let target_contract_id = Self::decode_smart_contract(target_contract_h160.into())?;

        log::trace!(target: "ds-precompile", "nomination_transfer {:?} {:?} {:?}", origin_contract_id, value, target_contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::nomination_transfer {
            origin_contract_id,
            value,
            target_contract_id,
        };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Helper method to decode type SmartContract enum
    pub fn decode_smart_contract(
        contract_h160: H160,
    ) -> EvmResult<<R as pallet_dapps_staking::Config>::SmartContract> {
        // Encode contract address to fit SmartContract enum.
        // Since the SmartContract enum type can't be accessed from this pecompile,
        // use locally defined enum clone (see Contract enum)
        let contract_enum_encoded = Contract::<H160>::Evm(contract_h160).encode();

        // encoded enum will add one byte before the contract's address
        // therefore we need to decode len(H160) + 1 byte = 21
        let smart_contract = <R as pallet_dapps_staking::Config>::SmartContract::decode(
            &mut &contract_enum_encoded[..21],
        )
        .map_err(|_| revert("Error while decoding SmartContract"))?;

        Ok(smart_contract)
    }

    /// Helper method to parse H160 or SS58 address
    fn parse_input_address(staker_vec: Vec<u8>) -> EvmResult<R::AccountId> {
        let staker: R::AccountId = match staker_vec.len() {
            // public address of the ss58 account has 32 bytes
            32 => {
                let mut staker_bytes = [0_u8; 32];
                staker_bytes[..].clone_from_slice(&staker_vec[0..32]);

                staker_bytes.into()
            }
            // public address of the H160 account has 20 bytes
            20 => {
                let mut staker_bytes = [0_u8; 20];
                staker_bytes[..].clone_from_slice(&staker_vec[0..20]);

                R::AddressMapping::into_account_id(staker_bytes.into())
            }
            _ => {
                // Return err if account length is wrong
                return Err(revert("Error while parsing staker's address"));
            }
        };

        Ok(staker)
    }
}
