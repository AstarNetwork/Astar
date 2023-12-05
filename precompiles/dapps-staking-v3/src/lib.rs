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

use fp_evm::{PrecompileHandle, PrecompileOutput};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};

use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};

use pallet_evm::{AddressMapping, Precompile};
use precompile_utils::{
    error, revert, succeed, Address, Bytes, EvmData, EvmDataWriter, EvmResult, FunctionModifier,
    PrecompileHandleExt, RuntimeHelper,
};
use sp_core::{Get, H160};
use sp_runtime::traits::Zero;
use sp_std::{marker::PhantomData, prelude::*};
extern crate alloc;

use astar_primitives::Balance;
use pallet_dapp_staking_v3::{
    AccountLedgerFor, ActiveProtocolState, ContractStake, ContractStakeAmount, CurrentEraInfo,
    DAppInfoFor, EraInfo, EraRewardSpanFor, EraRewards, IntegratedDApps, Ledger,
    Pallet as DAppStaking, ProtocolState, SingularStakingInfo, StakerInfo,
};

// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;

/// This is only used to encode SmartContract enum
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum Contract<A> {
    /// EVM smart contract instance.
    Evm(H160),
    /// Wasm smart contract instance. Not used in this precompile
    Wasm(A),
}

pub struct DappStakingV3Precompile<R>(PhantomData<R>);

impl<R> DappStakingV3Precompile<R>
where
    R: pallet_evm::Config + pallet_dapp_staking_v3::Config,
    <R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
    R::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    R::RuntimeCall: From<pallet_dapp_staking_v3::Call<R>>,
    R::AccountId: From<[u8; 32]>,
{
    /// Read the ongoing `era` number.
    fn read_current_era(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        handle.record_db_read::<R>(4 + ProtocolState::max_encoded_len())?;

        let current_era = ActiveProtocolState::<R>::get().era;

        Ok(succeed(EvmDataWriter::new().write(current_era).build()))
    }

    /// Read the `unbonding period` or `unlocking period` expressed in the number of eras.
    fn read_unbonding_period(_: &impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // constant, no DB read
        Ok(succeed(
            EvmDataWriter::new()
                .write(<R as pallet_dapp_staking_v3::Config>::UnlockingPeriod::get())
                .build(),
        ))
    }

    /// Read the total assigned reward pool for the given era.
    ///
    /// Total amount is sum of staker & dApp rewards.
    fn read_era_reward(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: EraRewards:
        // Twox64Concat(8) + EraIndex(4) + EraRewardSpanFor::max_encoded_len
        handle.record_db_read::<R>(12 + EraRewardSpanFor::<R>::max_encoded_len())?;

        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // Parse era for which rewards are required
        let era: u32 = input.read::<u32>()?;

        // Get the appropriate era reward span
        let era_span_index = DAppStaking::<R>::era_reward_span_index(era);
        let reward_span =
            EraRewards::<R>::get(&era_span_index).unwrap_or(EraRewardSpanFor::<R>::new());

        // Sum up staker & dApp reward pools for the era
        let reward = reward_span.get(era).map_or(Zero::zero(), |r| {
            r.staker_reward_pool.saturating_add(r.dapp_reward_pool)
        });

        Ok(succeed(EvmDataWriter::new().write(reward).build()))
    }

    /// Read the total staked amount for the given era.
    ///
    /// In case era is very far away in history, it's possible that the information is not available.
    /// In that case, zero is returned.
    ///
    /// This is safe to use for current era and the next one.
    fn read_era_staked(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: ActiveProtocolState:
        // Twox64(8) + ProtocolState::max_encoded_len
        handle.record_db_read::<R>(4 + ProtocolState::max_encoded_len())?;

        let current_era = ActiveProtocolState::<R>::get().era;

        // Parse era from the input
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;
        let era: u32 = input.read::<u32>()?;

        // There are few distinct scenenarios:
        // 1. Era is in the past so the value might exist.
        // 2. Era is current or the next one, in which case we definitely have that information.
        // 3. Era is from the future (more than the next era), in which case we don't have that information.
        if era < current_era {
            // TODO: benchmark this function so we can measure ref time & PoV correctly
            // Storage item: EraRewards:
            // Twox64Concat(8) + EraIndex(4) + EraRewardSpanFor::max_encoded_len
            handle.record_db_read::<R>(12 + EraRewardSpanFor::<R>::max_encoded_len())?;

            let era_span_index = DAppStaking::<R>::era_reward_span_index(era);
            let reward_span =
                EraRewards::<R>::get(&era_span_index).unwrap_or(EraRewardSpanFor::<R>::new());

            let staked = reward_span.get(era).map_or(Zero::zero(), |r| r.staked);

            Ok(succeed(EvmDataWriter::new().write(staked).build()))
        } else if era == current_era || era == current_era.saturating_add(1) {
            // TODO: benchmark this function so we can measure ref time & PoV correctly
            // Storage item: CurrentEraInfo:
            // Twox64Concat(8) + EraInfo::max_encoded_len
            handle.record_db_read::<R>(8 + EraInfo::max_encoded_len())?;

            let current_era_info = CurrentEraInfo::<R>::get();

            if era == current_era {
                Ok(succeed(
                    EvmDataWriter::new()
                        .write(current_era_info.current_stake_amount.total())
                        .build(),
                ))
            } else {
                Ok(succeed(
                    EvmDataWriter::new()
                        .write(current_era_info.next_stake_amount.total())
                        .build(),
                ))
            }
        } else {
            Err(error("Era is in the future"))
        }
    }

    /// Read the total staked amount by the given account.
    fn read_staked_amount(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
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

        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // parse the staker account
        let staker_vec: Vec<u8> = input.read::<Bytes>()?.into();
        let staker = Self::parse_input_address(staker_vec)?;

        // read the account's ledger
        let ledger = Ledger::<R>::get(&staker);
        log::trace!(target: "ds-precompile", "read_staked_amount for account: {:?}, ledger: {:?}", staker, ledger);

        // Make sure to check staked amount against the ongoing period (past period stakes are reset to zero).
        let current_period_number = ActiveProtocolState::<R>::get().period_number();

        Ok(succeed(
            EvmDataWriter::new()
                .write(ledger.staked_amount(current_period_number))
                .build(),
        ))
    }

    /// Read the total staked amount by the given staker on the given contract.
    fn read_staked_amount_on_contract(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
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

        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // parse contract address
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parsae the staker account
        let staker_vec: Vec<u8> = input.read::<Bytes>()?.into();
        let staker = Self::parse_input_address(staker_vec)?;

        // Get staking info for the staker/contract combination
        let staking_info = StakerInfo::<R>::get(&staker, &contract_id).unwrap_or_default();
        log::trace!(target: "ds-precompile", "read_staked_amount_on_contract for account:{:?}, staking_info: {:?}", staker, staking_info);

        // Ensure that the staking info is checked against the current period (stakes from past periods are reset)
        let current_period_number = ActiveProtocolState::<R>::get().period_number();

        if staking_info.period_number() == current_period_number {
            Ok(succeed(
                EvmDataWriter::new()
                    .write(staking_info.total_staked_amount())
                    .build(),
            ))
        } else {
            Ok(succeed(EvmDataWriter::new().write(0_u128).build()))
        }
    }

    /// Read the amount staked on right now.
    fn read_contract_stake(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
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

        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // parse input parameters for pallet-dapps-staking call
        let contract_h160 = input.read::<Address>()?.0;
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        let current_period_number = ActiveProtocolState::<R>::get().period_number();
        let dapp_info = match IntegratedDApps::<R>::get(&contract_id) {
            Some(dapp_info) => dapp_info,
            None => {
                // If the contract is not registered, return 0 to keep the legacy behavior.
                return Ok(succeed(EvmDataWriter::new().write(0_u128).build()));
            }
        };

        // call pallet-dapps-staking
        let contract_stake = ContractStake::<R>::get(&dapp_info.id);

        Ok(succeed(
            EvmDataWriter::new()
                .write(contract_stake.total_staked_amount(current_period_number))
                .build(),
        ))
    }

    /// Register contract with the dapp-staking pallet
    /// Register is root origin only. This should always fail when called via evm precompile.
    fn register(_: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // register is root-origin call. it should always fail when called via evm precompiles.
        Err(error("register via evm precompile is not allowed"))
    }

    /// Lock & stake some amount on the specified contract.
    ///
    /// In case existing `stakeable` is sufficient to cover the given `amount`, only the `stake` operation is performed.
    /// Otherwise, best effort is done to lock the additional amount so `stakeable` amount can cover the given `amount`.
    fn bond_and_stake(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
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

        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // Parse smart contract & amount
        let contract_h160 = input.read::<Address>()?.0;
        let smart_contract = Self::decode_smart_contract(contract_h160)?;
        let amount: Balance = input.read()?;

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

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Start unbonding process and unstake balance from the contract.
    fn unbond_and_unstake(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
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

        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // Parse smart contract & amount
        let contract_h160 = input.read::<Address>()?.0;
        let smart_contract = Self::decode_smart_contract(contract_h160)?;
        let amount: Balance = input.read()?;
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

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Claim back the unbonded (or unlocked) funds.
    fn withdraw_unbonded(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::claim_unlocked {};

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Claim dApp rewards for the given era
    fn claim_dapp(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(2)?;

        // Parse the smart contract & era
        let contract_h160 = input.read::<Address>()?.0;
        let smart_contract = Self::decode_smart_contract(contract_h160)?;
        let era: u32 = input.read::<u32>()?;
        log::trace!(target: "ds-precompile", "claim_dapp {:?}, era {:?}", smart_contract, era);

        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::claim_dapp_reward {
            smart_contract,
            era,
        };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Claim staker rewards.
    ///
    /// Smart contract argument is legacy & is ignored in the new implementation.
    fn claim_staker(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // Parse smart contract to keep in line with the legacy behavior.
        let contract_h160 = input.read::<Address>()?.0;
        let _smart_contract = Self::decode_smart_contract(contract_h160)?;
        log::trace!(target: "ds-precompile", "claim_staker {:?}", _smart_contract);

        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::claim_staker_rewards {};

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Set claim reward destination for the caller.
    ///
    /// This call has been deprecated by dApp staking v3.
    fn set_reward_destination(_handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        Err(error(
            "set_reward_destination via evm precompile is no longer supported",
        ))
    }

    /// Withdraw staked funds from the unregistered contract
    fn withdraw_from_unregistered(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(1)?;

        // Parse smart contract
        let contract_h160 = input.read::<Address>()?.0;
        let smart_contract = Self::decode_smart_contract(contract_h160)?;
        log::trace!(target: "ds-precompile", "withdraw_from_unregistered {:?}", smart_contract);

        let origin = R::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_dapp_staking_v3::Call::<R>::unstake_from_unregistered { smart_contract };

        RuntimeHelper::<R>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Transfers stake from one contract to another.
    /// This is a legacy functionality that is no longer supported via direct call to dApp staking v3.
    /// However, it can be achieved by chaining `unstake` and `stake` calls.
    fn nomination_transfer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // TODO: benchmark this function so we can measure ref time & PoV correctly
        // Storage item: StakerInfo:
        // Blake2_128Concat(16 + SmartContract::max_encoded_len) + SingularStakingInfo::max_encoded_len
        handle.record_db_read::<R>(
            16 + <R as pallet_dapp_staking_v3::Config>::SmartContract::max_encoded_len()
                + SingularStakingInfo::max_encoded_len(),
        )?;

        let mut input = handle.read_input()?;
        input.expect_arguments(3)?;

        // Parse origin smart contract, transfer amount & the target smart contract
        let origin_contract_h160 = input.read::<Address>()?.0;
        let origin_smart_contract = Self::decode_smart_contract(origin_contract_h160)?;

        let amount = input.read::<Balance>()?;

        let target_contract_h160 = input.read::<Address>()?.0;
        let target_smart_contract = Self::decode_smart_contract(target_contract_h160)?;
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

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    /// Helper method to decode type SmartContract enum
    pub fn decode_smart_contract(
        contract_h160: H160,
    ) -> EvmResult<<R as pallet_dapp_staking_v3::Config>::SmartContract> {
        // Encode contract address to fit SmartContract enum.
        // Since the SmartContract enum type can't be accessed from this pecompile,
        // use locally defined enum clone (see Contract enum)
        let contract_enum_encoded = Contract::<H160>::Evm(contract_h160).encode();

        // encoded enum will add one byte before the contract's address
        // therefore we need to decode len(H160) + 1 byte = 21
        let smart_contract = <R as pallet_dapp_staking_v3::Config>::SmartContract::decode(
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

impl<R> Precompile for DappStakingV3Precompile<R>
where
    R: pallet_evm::Config + pallet_dapp_staking_v3::Config,
    R::RuntimeCall: From<pallet_dapp_staking_v3::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <R::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<R::AccountId>>,
    Balance: EvmData,
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
