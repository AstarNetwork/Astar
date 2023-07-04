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
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{PrecompileHandle, PrecompileOutput};
use parity_scale_codec::{Decode, Encode};

use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::{Currency, Get},
};
use pallet_dapps_staking::RewardDestination;
use pallet_evm::{AddressMapping, Precompile};
use precompile_utils::{
    error, revert, succeed, Address, Bytes, EvmData, EvmDataWriter, EvmResult, FunctionModifier,
    PrecompileHandleExt, RuntimeHelper,
};
use sp_core::H160;
use sp_runtime::traits::{Saturating, Zero};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
extern crate alloc;

type BalanceOf<Runtime> = <<Runtime as pallet_dapps_staking::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

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

impl<R> DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    BalanceOf<R>: EvmData,
    <R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
    R::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    R::RuntimeCall: From<pallet_dapps_staking::Call<R>>,
    R::AccountId: From<[u8; 32]>,
{
    /// Fetch current era from CurrentEra storage map
    fn read_current_era(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        handle.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();

        Ok(succeed(EvmDataWriter::new().write(current_era).build()))
    }

    /// Fetch unbonding period
    fn read_unbonding_period(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        handle.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        let unbonding_period = R::UnbondingPeriod::get();

        Ok(succeed(
            EvmDataWriter::new().write(unbonding_period).build(),
        ))
    }

    /// Fetch reward from EraRewardsAndStakes storage map
    fn read_era_reward(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        handle.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // parse input parameters for pallet-dapps-staking call
        let era: u32 = input.read::<u32>()?;

        // call pallet-dapps-staking
        let read_reward = pallet_dapps_staking::GeneralEraInfo::<R>::get(era);
        let reward = read_reward.map_or(Zero::zero(), |r| {
            r.rewards.stakers.saturating_add(r.rewards.dapps)
        });

        Ok(succeed(EvmDataWriter::new().write(reward).build()))
    }

    /// Fetch total staked amount from EraRewardsAndStakes storage map
    fn read_era_staked(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        handle.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // parse input parameters for pallet-dapps-staking call
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let era: u32 = input.read::<u32>()?;

        // call pallet-dapps-staking
        let reward_and_stake = pallet_dapps_staking::GeneralEraInfo::<R>::get(era);
        // compose output
        let staked = reward_and_stake.map_or(Zero::zero(), |r| r.staked);
        let staked = TryInto::<u128>::try_into(staked).unwrap_or(0);

        Ok(succeed(EvmDataWriter::new().write(staked).build()))
    }

    /// Fetch Ledger storage map for an account
    fn read_staked_amount(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        handle.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // parse input parameters for pallet-dapps-staking call
        let staker_vec: Vec<u8> = input.read::<Bytes>()?.into();
        let staker = Self::parse_input_address(staker_vec)?;

        // call pallet-dapps-staking
        let ledger = pallet_dapps_staking::Ledger::<R>::get(&staker);
        log::trace!(target: "ds-precompile", "read_staked_amount for account:{:?}, ledger.locked:{:?}", staker, ledger.locked);

        Ok(succeed(EvmDataWriter::new().write(ledger.locked).build()))
    }

    /// Read GeneralStakerInfo for account/contract
    fn read_staked_amount_on_contract(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        handle.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // parse contract address
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parse input parameters for pallet-dapps-staking call
        let staker_vec: Vec<u8> = input.read::<Bytes>()?.into();
        let staker = Self::parse_input_address(staker_vec)?;

        // call pallet-dapps-staking
        let staking_info = pallet_dapps_staking::GeneralStakerInfo::<R>::get(&staker, &contract_id);
        let staked_amount = staking_info.latest_staked_value();
        log::trace!(target: "ds-precompile", "read_staked_amount_on_contract for account:{:?}, contract: {:?} => staked_amount:{:?}", staker, contract_id, staked_amount);

        Ok(succeed(EvmDataWriter::new().write(staked_amount).build()))
    }

    /// Read the amount staked on contract in the given era
    fn read_contract_stake(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        handle.record_cost(2 * RuntimeHelper::<R>::db_read_gas_cost())?;

        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // parse input parameters for pallet-dapps-staking call
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;
        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();

        // call pallet-dapps-staking
        let staking_info =
            pallet_dapps_staking::Pallet::<R>::contract_stake_info(&contract_id, current_era)
                .unwrap_or_default();

        // encode output with total
        let total = TryInto::<u128>::try_into(staking_info.total).unwrap_or(0);

        Ok(succeed(EvmDataWriter::new().write(total).build()))
    }

    /// Register contract with the dapp-staking pallet
    /// Register is root origin only. This should always fail when called via evm precompile.
    fn register(_: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // register is root-origin call. it should always fail when called via evm precompiles.
        Err(error("register via evm precompile is not allowed"))
    }

    /// Lock up and stake balance of the origin account.
    fn bond_and_stake(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // parse contract's address
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parse balance to be staked
        let value: BalanceOf<R> = input.read()?;

        log::trace!(target: "ds-precompile", "bond_and_stake {:?}, {:?}", contract_id, value);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::bond_and_stake { contract_id, value };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Start unbonding process and unstake balance from the contract.
    fn unbond_and_unstake(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // parse contract's address
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parse balance to be unstaked
        let value: BalanceOf<R> = input.read()?;
        log::trace!(target: "ds-precompile", "unbond_and_unstake {:?}, {:?}", contract_id, value);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::unbond_and_unstake { contract_id, value };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Start unbonding process and unstake balance from the contract.
    fn withdraw_unbonded(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::withdraw_unbonded {};

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    fn claim_dapp(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // parse contract's address
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parse era
        let era: u32 = input.read::<u32>()?;
        log::trace!(target: "ds-precompile", "claim_dapp {:?}, era {:?}", contract_id, era);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::claim_dapp { contract_id, era };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    fn claim_staker(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // parse contract's address
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;
        log::trace!(target: "ds-precompile", "claim_staker {:?}", contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::claim_staker { contract_id };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Set claim reward destination for the caller
    fn set_reward_destination(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // raw solidity representation of enum
        let reward_destination_raw = input.read::<u8>()?;

        // Transform raw value into dapps staking enum
        let reward_destination = if reward_destination_raw == 0 {
            RewardDestination::FreeBalance
        } else if reward_destination_raw == 1 {
            RewardDestination::StakeBalance
        } else {
            return Err(error("Unexpected reward destination value."));
        };

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        log::trace!(target: "ds-precompile", "set_reward_destination {:?} {:?}", origin, reward_destination);

        let call = pallet_dapps_staking::Call::<R>::set_reward_destination { reward_destination };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }
    /// Withdraw staked funds from the unregistered contract
    fn withdraw_from_unregistered(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // parse contract's address
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;
        log::trace!(target: "ds-precompile", "withdraw_from_unregistered {:?}", contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::withdraw_from_unregistered { contract_id };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    fn nomination_transfer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(3)?;

        // parse origin contract's address
        let origin_contract_h160 = input.read::<Address>()?.0;
        let origin_contract_id = Self::decode_smart_contract(origin_contract_h160)?;

        // parse balance to be transferred
        let value = input.read::<BalanceOf<R>>()?;

        // parse target contract's address
        let target_contract_h160 = input.read::<Address>()?.0;
        let target_contract_id = Self::decode_smart_contract(target_contract_h160)?;

        log::trace!(target: "ds-precompile", "nomination_transfer {:?} {:?} {:?}", origin_contract_id, value, target_contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapps_staking::Call::<R>::nomination_transfer {
            origin_contract_id,
            value,
            target_contract_id,
        };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
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

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    ReadCurrentEra = "read_current_era()",
    ReadUnbondingPeriod = "read_unbonding_period()",
    ReadEraReward = "read_era_reward(uint32)",
    ReadEraStaked = "read_era_staked(uint32)",
    ReadStakedAmount = "read_staked_amount(bytes)",
    ReadStakedAmountOnContract = "read_staked_amount_on_contract(address,bytes)",
    ReadContractStake = "read_contract_stake(address)",
    Register = "register(address)",
    BondAndStake = "bond_and_stake(address,uint128)",
    UnbondAndUnstake = "unbond_and_unstake(address,uint128)",
    WithdrawUnbounded = "withdraw_unbonded()",
    ClaimDapp = "claim_dapp(address,uint128)",
    ClaimStaker = "claim_staker(address)",
    SetRewardDestination = "set_reward_destination(uint8)",
    WithdrawFromUnregistered = "withdraw_from_unregistered(address)",
    NominationTransfer = "nomination_transfer(address,uint128,address)",
}

impl<R> Precompile for DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    R::RuntimeCall: From<pallet_dapps_staking::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
    BalanceOf<R>: EvmData,
    R::AccountId: From<[u8; 32]>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "ds-precompile", "Execute input = {:?}", handle.input());

        let selector = handle.read_selector()?;

        handle.check_function_modifier(match selector {
            Action::ReadCurrentEra
            | Action::ReadUnbondingPeriod
            | Action::ReadEraReward
            | Action::ReadEraStaked
            | Action::ReadStakedAmount
            | Action::ReadStakedAmountOnContract
            | Action::ReadContractStake => FunctionModifier::View,
            _ => FunctionModifier::NonPayable,
        })?;

        match selector {
            // read storage
            Action::ReadCurrentEra => Self::read_current_era(handle),
            Action::ReadUnbondingPeriod => Self::read_unbonding_period(handle),
            Action::ReadEraReward => Self::read_era_reward(handle),
            Action::ReadEraStaked => Self::read_era_staked(handle),
            Action::ReadStakedAmount => Self::read_staked_amount(handle),
            Action::ReadStakedAmountOnContract => Self::read_staked_amount_on_contract(handle),
            Action::ReadContractStake => Self::read_contract_stake(handle),
            // Dispatchables
            Action::Register => Self::register(handle),
            Action::BondAndStake => Self::bond_and_stake(handle),
            Action::UnbondAndUnstake => Self::unbond_and_unstake(handle),
            Action::WithdrawUnbounded => Self::withdraw_unbonded(handle),
            Action::ClaimDapp => Self::claim_dapp(handle),
            Action::ClaimStaker => Self::claim_staker(handle),
            Action::SetRewardDestination => Self::set_reward_destination(handle),
            Action::WithdrawFromUnregistered => Self::withdraw_from_unregistered(handle),
            Action::NominationTransfer => Self::nomination_transfer(handle),
        }
    }
}
