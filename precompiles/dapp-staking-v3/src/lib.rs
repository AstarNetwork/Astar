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

//! Astar dApp staking interface.

#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use parity_scale_codec::MaxEncodedLen;

use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    ensure,
    traits::ConstU32,
};

use pallet_evm::AddressMapping;
use precompile_utils::{
    prelude::*,
    solidity::{
        codec::{Reader, Writer},
        Codec,
    },
};
use sp_core::{Get, H160, U256};
use sp_runtime::traits::Zero;
use sp_std::{marker::PhantomData, prelude::*};
extern crate alloc;

use astar_primitives::{dapp_staking::SmartContractHandle, AccountId, Balance};
use pallet_dapp_staking_v3::{
    AccountLedgerFor, ActiveProtocolState, ContractStake, ContractStakeAmount, CurrentEraInfo,
    DAppInfoFor, EraInfo, EraRewardSpanFor, EraRewards, IntegratedDApps, Ledger,
    Pallet as DAppStaking, ProtocolState, SingularStakingInfo, StakerInfo, Subperiod,
};

pub const STAKER_BYTES_LIMIT: u32 = 32;
type GetStakerBytesLimit = ConstU32<STAKER_BYTES_LIMIT>;

pub type DynamicAddress = BoundedBytes<GetStakerBytesLimit>;

#[cfg(test)]
mod test;

/// Helper struct used to encode protocol state.
#[derive(Debug, Clone, solidity::Codec)]
pub(crate) struct PrecompileProtocolState {
    era: U256,
    period: U256,
    subperiod: u8,
}

/// Helper struct used to encode different smart contract types for the v2 interface.
#[derive(Debug, Clone, solidity::Codec)]
pub struct SmartContractV2 {
    contract_type: SmartContractTypes,
    address: DynamicAddress,
}

/// Convenience type for smart contract type handling.
#[derive(Clone, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub(crate) enum SmartContractTypes {
    Evm,
    Wasm,
}

impl Codec for SmartContractTypes {
    fn read(reader: &mut Reader) -> MayRevert<SmartContractTypes> {
        let value256: U256 = reader
            .read()
            .map_err(|_| RevertReason::read_out_of_bounds(Self::signature()))?;

        let value_as_u8: u8 = value256
            .try_into()
            .map_err(|_| RevertReason::value_is_too_large(Self::signature()))?;

        value_as_u8
            .try_into()
            .map_err(|_| RevertReason::custom("Unknown smart contract type").into())
    }

    fn write(writer: &mut Writer, value: Self) {
        let value_as_u8: u8 = value.into();
        U256::write(writer, value_as_u8.into());
    }

    fn has_static_size() -> bool {
        true
    }

    fn signature() -> String {
        "uint8".into()
    }
}

pub struct DappStakingV3Precompile<R>(PhantomData<R>);
#[precompile_utils::precompile]
impl<R> DappStakingV3Precompile<R>
where
    R: pallet_evm::Config
        + pallet_dapp_staking_v3::Config
        + frame_system::Config<AccountId = AccountId>,
    <R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
    R::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    R::RuntimeCall: From<pallet_dapp_staking_v3::Call<R>>,
{
    // v1 functions

    /// Read the ongoing `era` number.
    #[precompile::public("read_current_era()")]
    #[precompile::view]
    fn read_current_era(handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        handle.record_db_read::<R>(8 + ProtocolState::max_encoded_len())?;

        let current_era = ActiveProtocolState::<R>::get().era;

        Ok(current_era.into())
    }

    /// Read the `unbonding period` or `unlocking period` expressed in the number of eras.
    #[precompile::public("read_unbonding_period()")]
    #[precompile::view]
    fn read_unbonding_period(_: &mut impl PrecompileHandle) -> EvmResult<U256> {
        // constant, no DB read
        Ok(<R as pallet_dapp_staking_v3::Config>::UnlockingPeriod::get().into())
    }

    /// Read the total assigned reward pool for the given era.
    ///
    /// Total amount is sum of staker & dApp rewards.
    #[precompile::public("read_era_reward(uint32)")]
    #[precompile::view]
    fn read_era_reward(handle: &mut impl PrecompileHandle, era: u32) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: EraRewards:
        // Twox64Concat(8) + EraIndex(4) + EraRewardSpanFor::max_encoded_len
        handle.record_db_read::<R>(12 + EraRewardSpanFor::<R>::max_encoded_len())?;

        // Get the appropriate era reward span
        let era_span_index = DAppStaking::<R>::era_reward_span_index(era);
        let reward_span =
            EraRewards::<R>::get(&era_span_index).unwrap_or(EraRewardSpanFor::<R>::new());

        // Sum up staker & dApp reward pools for the era
        let reward = reward_span.get(era).map_or(Zero::zero(), |r| {
            r.staker_reward_pool.saturating_add(r.dapp_reward_pool)
        });

        Ok(reward)
    }

    /// Read the total staked amount for the given era.
    ///
    /// In case era is very far away in history, it's possible that the information is not available.
    /// In that case, zero is returned.
    ///
    /// This is safe to use for current era and the next one.
    #[precompile::public("read_era_staked(uint32)")]
    #[precompile::view]
    fn read_era_staked(handle: &mut impl PrecompileHandle, era: u32) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        handle.record_db_read::<R>(8 + ProtocolState::max_encoded_len())?;

        let current_era = ActiveProtocolState::<R>::get().era;

        // There are few distinct scenenarios:
        // 1. Era is in the past so the value might exist.
        // 2. Era is current or the next one, in which case we definitely have that information.
        // 3. Era is from the future (more than the next era), in which case we don't have that information.
        if era < current_era {
            // TODO: benchmark this function so we can measure ref time & PoV correctly
            // Storage item: EraRewards:
            // Twox64Concat(8) + Twox64Concat(8 + EraIndex(4)) + EraRewardSpanFor::max_encoded_len
            handle.record_db_read::<R>(20 + EraRewardSpanFor::<R>::max_encoded_len())?;

            let era_span_index = DAppStaking::<R>::era_reward_span_index(era);
            let reward_span =
                EraRewards::<R>::get(&era_span_index).unwrap_or(EraRewardSpanFor::<R>::new());

            let staked = reward_span.get(era).map_or(Zero::zero(), |r| r.staked);

            Ok(staked.into())
        } else if era == current_era || era == current_era.saturating_add(1) {
            // TODO: benchmark this function so we can measure ref time & PoV correctly
            // Storage item: CurrentEraInfo:
            // Twox64Concat(8) + EraInfo::max_encoded_len
            handle.record_db_read::<R>(8 + EraInfo::max_encoded_len())?;

            let current_era_info = CurrentEraInfo::<R>::get();

            if era == current_era {
                Ok(current_era_info.current_stake_amount.total())
            } else {
                Ok(current_era_info.next_stake_amount.total())
            }
        } else {
            Err(RevertReason::custom("Era is in the future").into())
        }
    }

    /// Read the total staked amount by the given account.
    #[precompile::public("read_staked_amount(bytes)")]
    #[precompile::view]
    fn read_staked_amount(
        handle: &mut impl PrecompileHandle,
        staker: DynamicAddress,
    ) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        // Storage item: Ledger:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len) + Ledger::max_encoded_len
        handle.record_db_read::<R>(
            24 + AccountLedgerFor::<R>::max_encoded_len()
                + ProtocolState::max_encoded_len()
                + <R as pallet_dapp_staking_v3::Config>::SmartContract::max_encoded_len(),
        )?;

        let staker = Self::parse_input_address(staker.into())?;

        // read the account's ledger
        let ledger = Ledger::<R>::get(&staker);
        log::trace!(target: "ds-precompile", "read_staked_amount for account: {:?}, ledger: {:?}", staker, ledger);

        // Make sure to check staked amount against the ongoing period (past period stakes are reset to zero).
        let current_period_number = ActiveProtocolState::<R>::get().period_number();

        Ok(ledger.staked_amount(current_period_number))
    }

    /// Read the total staked amount by the given staker on the given contract.
    #[precompile::public("read_staked_amount_on_contract(address,bytes)")]
    #[precompile::view]
    fn read_staked_amount_on_contract(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        staker: DynamicAddress,
    ) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        // Storage item: StakerInfo:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len) + SingularStakingInfo::max_encoded_len
        handle.record_db_read::<R>(
            24 + ProtocolState::max_encoded_len()
                + <R as pallet_dapp_staking_v3::Config>::SmartContract::max_encoded_len()
                + SingularStakingInfo::max_encoded_len(),
        )?;

        let smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(contract_h160.into());

        // parse the staker account
        let staker = Self::parse_input_address(staker.into())?;

        // Get staking info for the staker/contract combination
        let staking_info = StakerInfo::<R>::get(&staker, &smart_contract).unwrap_or_default();
        log::trace!(target: "ds-precompile", "read_staked_amount_on_contract for account:{:?}, staking_info: {:?}", staker, staking_info);

        // Ensure that the staking info is checked against the current period (stakes from past periods are reset)
        let current_period_number = ActiveProtocolState::<R>::get().period_number();

        if staking_info.period_number() == current_period_number {
            Ok(staking_info.total_staked_amount())
        } else {
            Ok(0_u128)
        }
    }

    /// Read the total amount staked on the given contract right now.
    #[precompile::public("read_contract_stake(address)")]
    #[precompile::view]
    fn read_contract_stake(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
    ) -> EvmResult<u128> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        // Storage item: IntegratedDApps:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len) + DAppInfoFor::max_encoded_len
        // Storage item: ContractStake:
        // Twox64Concat(8) + EraIndex(4) + ContractStakeAmount::max_encoded_len
        handle.record_db_read::<R>(
            36 + ProtocolState::max_encoded_len()
                + <R as pallet_dapp_staking_v3::Config>::SmartContract::max_encoded_len()
                + DAppInfoFor::<R>::max_encoded_len()
                + ContractStakeAmount::max_encoded_len(),
        )?;

        let smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(contract_h160.into());

        let current_period_number = ActiveProtocolState::<R>::get().period_number();
        let dapp_info = match IntegratedDApps::<R>::get(&smart_contract) {
            Some(dapp_info) => dapp_info,
            None => {
                // If the contract is not registered, return 0 to keep the legacy behavior.
                return Ok(0_u128);
            }
        };

        // call pallet-dapps-staking
        let contract_stake = ContractStake::<R>::get(&dapp_info.id);

        Ok(contract_stake.total_staked_amount(current_period_number))
    }

    /// Register contract with the dapp-staking pallet
    /// Register is root origin only. This should always fail when called via evm precompile.
    #[precompile::public("register(address)")]
    fn register(_: &mut impl PrecompileHandle, _address: Address) -> EvmResult<bool> {
        // register is root-origin call. it should always fail when called via evm precompiles.
        Err(RevertReason::custom("register via evm precompile is not allowed").into())
    }

    /// Lock & stake some amount on the specified contract.
    ///
    /// In case existing `stakeable` is sufficient to cover the given `amount`, only the `stake` operation is performed.
    /// Otherwise, best effort is done to lock the additional amount so `stakeable` amount can cover the given `amount`.
    #[precompile::public("bond_and_stake(address,uint128)")]
    fn bond_and_stake(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        amount: u128,
    ) -> EvmResult<bool> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        // Storage item: Ledger:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len()) + Ledger::max_encoded_len
        handle.record_db_read::<R>(
            24 + AccountLedgerFor::<R>::max_encoded_len()
                + ProtocolState::max_encoded_len()
                + <R as pallet_dapp_staking_v3::Config>::SmartContract::max_encoded_len(),
        )?;

        let smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(contract_h160.into());
        log::trace!(target: "ds-precompile", "bond_and_stake {:?}, {:?}", smart_contract, amount);

        // Read total locked & staked amounts
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let protocol_state = ActiveProtocolState::<R>::get();
        let ledger = Ledger::<R>::get(&origin);

        // Check if stakeable amount is enough to cover the given `amount`
        let stakeable_amount = ledger.stakeable_amount(protocol_state.period_number());

        // If it isn't, we need to first lock the additional amount.
        if stakeable_amount < amount {
            let delta = amount.saturating_sub(stakeable_amount);

            let lock_call = pallet_dapp_staking_v3::Call::<R>::lock { amount: delta };
            RuntimeHelper::<R>::try_dispatch(handle, Some(origin.clone()).into(), lock_call)?;
        }

        // Now, with best effort, we can try & stake the given `value`.
        let stake_call = pallet_dapp_staking_v3::Call::<R>::stake {
            smart_contract,
            amount,
        };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), stake_call)?;

        Ok(true)
    }

    /// Start unbonding process and unstake balance from the contract.
    #[precompile::public("unbond_and_unstake(address,uint128)")]
    fn unbond_and_unstake(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        amount: u128,
    ) -> EvmResult<bool> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        // Storage item: StakerInfo:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len) + SingularStakingInfo::max_encoded_len
        handle.record_db_read::<R>(
            24 + ProtocolState::max_encoded_len()
                + <R as pallet_dapp_staking_v3::Config>::SmartContract::max_encoded_len()
                + SingularStakingInfo::max_encoded_len(),
        )?;

        let smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(contract_h160.into());
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        log::trace!(target: "ds-precompile", "unbond_and_unstake {:?}, {:?}", smart_contract, amount);

        // Find out if there is something staked on the contract
        let protocol_state = ActiveProtocolState::<R>::get();
        let staker_info = StakerInfo::<R>::get(&origin, &smart_contract).unwrap_or_default();

        // If there is, we need to unstake it before calling `unlock`
        if staker_info.period_number() == protocol_state.period_number() {
            let unstake_call = pallet_dapp_staking_v3::Call::<R>::unstake {
                smart_contract,
                amount,
            };
            RuntimeHelper::<R>::try_dispatch(handle, Some(origin.clone()).into(), unstake_call)?;
        }

        // Now we can try and `unlock` the given `amount`
        let unlock_call = pallet_dapp_staking_v3::Call::<R>::unlock { amount };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), unlock_call)?;

        Ok(true)
    }

    /// Claim back the unbonded (or unlocked) funds.
    #[precompile::public("withdraw_unbonded()")]
    fn withdraw_unbonded(handle: &mut impl PrecompileHandle) -> EvmResult<bool> {
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::claim_unlocked {};

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Claim dApp rewards for the given era
    #[precompile::public("claim_dapp(address,uint128)")]
    fn claim_dapp(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
        era: u128,
    ) -> EvmResult<bool> {
        let smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(contract_h160.into());

        // parse era
        let era = era
            .try_into()
            .map_err::<Revert, _>(|_| RevertReason::value_is_too_large("era type").into())
            .in_field("era")?;

        log::trace!(target: "ds-precompile", "claim_dapp {:?}, era {:?}", smart_contract, era);

        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::claim_dapp_reward {
            smart_contract,
            era,
        };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Claim staker rewards.
    ///
    /// Smart contract argument is legacy & is ignored in the new implementation.
    #[precompile::public("claim_staker(address)")]
    fn claim_staker(
        handle: &mut impl PrecompileHandle,
        _contract_h160: Address,
    ) -> EvmResult<bool> {
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::claim_staker_rewards {};

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Set claim reward destination for the caller.
    ///
    /// This call has been deprecated by dApp staking v3.
    #[precompile::public("set_reward_destination(uint8)")]
    fn set_reward_destination(_: &mut impl PrecompileHandle, _destination: u8) -> EvmResult<bool> {
        Err(RevertReason::custom("Setting reward destination is no longer supported.").into())
    }

    /// Withdraw staked funds from the unregistered contract
    #[precompile::public("withdraw_from_unregistered(address)")]
    fn withdraw_from_unregistered(
        handle: &mut impl PrecompileHandle,
        contract_h160: Address,
    ) -> EvmResult<bool> {
        let smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(contract_h160.into());
        log::trace!(target: "ds-precompile", "withdraw_from_unregistered {:?}", smart_contract);

        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::unstake_from_unregistered { smart_contract };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(true)
    }

    /// Transfers stake from one contract to another.
    /// This is a legacy functionality that is no longer supported via direct call to dApp staking v3.
    /// However, it can be achieved by chaining `unstake` and `stake` calls.
    #[precompile::public("nomination_transfer(address,uint128,address)")]
    fn nomination_transfer(
        handle: &mut impl PrecompileHandle,
        origin_contract_h160: Address,
        amount: u128,
        target_contract_h160: Address,
    ) -> EvmResult<bool> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: StakerInfo:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len) + SingularStakingInfo::max_encoded_len
        handle.record_db_read::<R>(
            16 + <R as pallet_dapp_staking_v3::Config>::SmartContract::max_encoded_len()
                + SingularStakingInfo::max_encoded_len(),
        )?;

        let origin_smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(origin_contract_h160.into());
        let target_smart_contract =
            <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(target_contract_h160.into());
        log::trace!(target: "ds-precompile", "nomination_transfer {:?} {:?} {:?}", origin_smart_contract, amount, target_smart_contract);

        // Find out how much staker has staked on the origin contract
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let staker_info = StakerInfo::<R>::get(&origin, &origin_smart_contract).unwrap_or_default();

        // We don't care from which period the staked amount is, the logic takes care of the situation
        // if value comes from the past period.
        let staked_amount = staker_info.total_staked_amount();
        let minimum_allowed_stake_amount =
            <R as pallet_dapp_staking_v3::Config>::MinimumStakeAmount::get();

        // In case the remaining staked amount on the origin contract is less than the minimum allowed stake amount,
        // everything will be unstaked. To keep in line with legacy `nomination_transfer` behavior, we should transfer
        // the entire amount from the origin to target contract.
        //
        // In case value comes from the past period, we don't care, since the `unstake` call will fall apart.
        let stake_amount = if staked_amount > 0
            && staked_amount.saturating_sub(amount) < minimum_allowed_stake_amount
        {
            staked_amount
        } else {
            amount
        };

        // First call unstake from the origin smart contract
        let unstake_call = pallet_dapp_staking_v3::Call::<R>::unstake {
            smart_contract: origin_smart_contract,
            amount,
        };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin.clone()).into(), unstake_call)?;

        // Then call stake on the target smart contract
        let stake_call = pallet_dapp_staking_v3::Call::<R>::stake {
            smart_contract: target_smart_contract,
            amount: stake_amount,
        };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), stake_call)?;

        Ok(true)
    }

    // v2 functions

    /// Read the current protocol state.
    #[precompile::public("protocol_state()")]
    #[precompile::view]
    fn protocol_state(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileProtocolState> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        handle.record_db_read::<R>(8 + ProtocolState::max_encoded_len())?;

        let protocol_state = ActiveProtocolState::<R>::get();

        Ok(PrecompileProtocolState {
            era: protocol_state.era.into(),
            period: protocol_state.period_number().into(),
            subperiod: subperiod_id(&protocol_state.subperiod()),
        })
    }

    /// Read the `unbonding period` or `unlocking period` expressed in the number of eras.
    #[precompile::public("unlocking_period()")]
    #[precompile::view]
    fn unlocking_period(_: &mut impl PrecompileHandle) -> EvmResult<U256> {
        // constant, no DB read
        Ok(DAppStaking::<R>::unlocking_period().into())
    }

    /// Attempt to lock the given amount into the dApp staking protocol.
    #[precompile::public("lock(uint128)")]
    fn lock(handle: &mut impl PrecompileHandle, amount: u128) -> EvmResult<bool> {
        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let lock_call = pallet_dapp_staking_v3::Call::<R>::lock { amount };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), lock_call)?;

        Ok(true)
    }

    /// Attempt to unlock the given amount from the dApp staking protocol.
    #[precompile::public("unlock(uint128)")]
    fn unlock(handle: &mut impl PrecompileHandle, amount: u128) -> EvmResult<bool> {
        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let unlock_call = pallet_dapp_staking_v3::Call::<R>::unlock { amount };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), unlock_call)?;

        Ok(true)
    }

    /// Attempts to claim unlocking chunks which have undergone the entire unlocking period.
    #[precompile::public("claim_unlocked()")]
    fn claim_unlocked(handle: &mut impl PrecompileHandle) -> EvmResult<bool> {
        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let claim_unlocked_call = pallet_dapp_staking_v3::Call::<R>::claim_unlocked {};
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), claim_unlocked_call)?;

        Ok(true)
    }

    /// Attempts to stake the given amount on the given smart contract.
    #[precompile::public("stake((uint8,bytes),uint128)")]
    fn stake(
        handle: &mut impl PrecompileHandle,
        smart_contract: SmartContractV2,
        amount: Balance,
    ) -> EvmResult<bool> {
        let smart_contract = Self::decode_smart_contract(smart_contract)?;

        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let stake_call = pallet_dapp_staking_v3::Call::<R>::stake {
            smart_contract,
            amount,
        };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), stake_call)?;

        Ok(true)
    }

    /// Attempts to unstake the given amount from the given smart contract.
    #[precompile::public("unstake((uint8,bytes),uint128)")]
    fn unstake(
        handle: &mut impl PrecompileHandle,
        smart_contract: SmartContractV2,
        amount: Balance,
    ) -> EvmResult<bool> {
        let smart_contract = Self::decode_smart_contract(smart_contract)?;

        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let unstake_call = pallet_dapp_staking_v3::Call::<R>::unstake {
            smart_contract,
            amount,
        };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), unstake_call)?;

        Ok(true)
    }

    /// Attempts to claim one or more pending staker rewards.
    #[precompile::public("claim_staker_rewards()")]
    fn claim_staker_rewards(handle: &mut impl PrecompileHandle) -> EvmResult<bool> {
        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let claim_staker_rewards_call = pallet_dapp_staking_v3::Call::<R>::claim_staker_rewards {};
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), claim_staker_rewards_call)?;

        Ok(true)
    }

    /// Attempts to claim bonus reward for being a loyal staker of the given dApp.
    #[precompile::public("claim_bonus_reward((uint8,bytes))")]
    fn claim_bonus_reward(
        handle: &mut impl PrecompileHandle,
        smart_contract: SmartContractV2,
    ) -> EvmResult<bool> {
        let smart_contract = Self::decode_smart_contract(smart_contract)?;

        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let claim_bonus_reward_call =
            pallet_dapp_staking_v3::Call::<R>::claim_bonus_reward { smart_contract };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), claim_bonus_reward_call)?;

        Ok(true)
    }

    /// Attempts to claim dApp reward for the given dApp in the given era.
    #[precompile::public("claim_bonus_reward((uint8,bytes),uint256)")]
    fn claim_dapp_reward(
        handle: &mut impl PrecompileHandle,
        smart_contract: SmartContractV2,
        era: U256,
    ) -> EvmResult<bool> {
        let smart_contract = Self::decode_smart_contract(smart_contract)?;
        let era = era
            .try_into()
            .map_err::<Revert, _>(|_| RevertReason::value_is_too_large("Era number.").into())
            .in_field("era")?;

        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let claim_dapp_reward_call = pallet_dapp_staking_v3::Call::<R>::claim_dapp_reward {
            smart_contract,
            era,
        };
        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), claim_dapp_reward_call)?;

        Ok(true)
    }

    /// Attempts to unstake everything from an unregistered contract.
    #[precompile::public("unstake_from_unregistered((uint8,bytes))")]
    fn unstake_from_unregistered(
        handle: &mut impl PrecompileHandle,
        smart_contract: SmartContractV2,
    ) -> EvmResult<bool> {
        let smart_contract = Self::decode_smart_contract(smart_contract)?;

        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let unstake_from_unregistered_call =
            pallet_dapp_staking_v3::Call::<R>::unstake_from_unregistered { smart_contract };
        RuntimeHelper::<R>::try_dispatch(
            handle,
            Some(origin).into(),
            unstake_from_unregistered_call,
        )?;

        Ok(true)
    }

    /// Attempts to cleanup expired entries for the staker.
    #[precompile::public("cleanup_expired_entries()")]
    fn cleanup_expired_entries(handle: &mut impl PrecompileHandle) -> EvmResult<bool> {
        // Prepare call & dispatch it
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let cleanup_expired_entries_call =
            pallet_dapp_staking_v3::Call::<R>::cleanup_expired_entries {};
        RuntimeHelper::<R>::try_dispatch(
            handle,
            Some(origin).into(),
            cleanup_expired_entries_call,
        )?;

        Ok(true)
    }

    // Utility functions

    /// Helper method to decode smart contract struct for v2 calls
    pub(crate) fn decode_smart_contract(
        smart_contract: SmartContractV2,
    ) -> EvmResult<<R as pallet_dapp_staking_v3::Config>::SmartContract> {
        let smart_contract = match smart_contract.contract_type {
            SmartContractTypes::Evm => {
                ensure!(
                    smart_contract.address.as_bytes().len() == 20,
                    revert("Invalid address length for Astar EVM smart contract.")
                );
                let h160_address = H160::from_slice(smart_contract.address.as_bytes());
                <R as pallet_dapp_staking_v3::Config>::SmartContract::evm(h160_address)
            }
            SmartContractTypes::Wasm => {
                ensure!(
                    smart_contract.address.as_bytes().len() == 32,
                    revert("Invalid address length for Astar WASM smart contract.")
                );
                let mut staker_bytes = [0_u8; 32];
                staker_bytes[..].clone_from_slice(&smart_contract.address.as_bytes());

                <R as pallet_dapp_staking_v3::Config>::SmartContract::wasm(staker_bytes.into())
            }
        };

        Ok(smart_contract)
    }

    /// Helper method to parse H160 or SS58 address
    pub(crate) fn parse_input_address(staker_vec: Vec<u8>) -> EvmResult<R::AccountId> {
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

/// Numeric Id of the subperiod enum value.
pub(crate) fn subperiod_id(subperiod: &Subperiod) -> u8 {
    match subperiod {
        Subperiod::Voting => 0,
        Subperiod::BuildAndEarn => 1,
    }
}
