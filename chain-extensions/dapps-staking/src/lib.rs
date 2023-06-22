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

#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::{
    traits::{Saturating, Zero},
    DispatchError,
};

use dapps_staking_chain_extension_types::{
    DSError, DappsStakingAccountInput, DappsStakingEraInput, DappsStakingNominationInput,
    DappsStakingValueInput,
};
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use pallet_dapps_staking::{RewardDestination, WeightInfo};
use parity_scale_codec::Encode;
use sp_std::marker::PhantomData;

type BalanceOf<T> = <<T as pallet_dapps_staking::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

enum DappsStakingFunc {
    CurrentEra,
    UnbondingPeriod,
    EraRewards,
    EraStaked,
    StakedAmount,
    StakedAmountOnContract,
    ReadContractStake,
    BondAndStake,
    UnbondAndUnstake,
    WithdrawUnbonded,
    ClaimStaker,
    ClaimDapp,
    SetRewardDestination,
    NominationTransfer,
}

impl TryFrom<u16> for DappsStakingFunc {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DappsStakingFunc::CurrentEra),
            2 => Ok(DappsStakingFunc::UnbondingPeriod),
            3 => Ok(DappsStakingFunc::EraRewards),
            4 => Ok(DappsStakingFunc::EraStaked),
            5 => Ok(DappsStakingFunc::StakedAmount),
            6 => Ok(DappsStakingFunc::StakedAmountOnContract),
            7 => Ok(DappsStakingFunc::ReadContractStake),
            8 => Ok(DappsStakingFunc::BondAndStake),
            9 => Ok(DappsStakingFunc::UnbondAndUnstake),
            10 => Ok(DappsStakingFunc::WithdrawUnbonded),
            11 => Ok(DappsStakingFunc::ClaimStaker),
            12 => Ok(DappsStakingFunc::ClaimDapp),
            13 => Ok(DappsStakingFunc::SetRewardDestination),
            14 => Ok(DappsStakingFunc::NominationTransfer),
            _ => Err(DispatchError::Other(
                "DappsStakingExtension: Unimplemented func_id",
            )),
        }
    }
}

/// Dapps Staking chain extension.
pub struct DappsStakingExtension<T>(PhantomData<T>);

impl<T> Default for DappsStakingExtension<T> {
    fn default() -> Self {
        DappsStakingExtension(PhantomData)
    }
}

impl<T> ChainExtension<T> for DappsStakingExtension<T>
where
    T: pallet_dapps_staking::Config + pallet_contracts::Config,
    <T as pallet_dapps_staking::Config>::SmartContract: From<[u8; 32]>,
    <T as SysConfig>::AccountId: From<[u8; 32]>,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            DappsStakingFunc::CurrentEra => {
                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let era_index = pallet_dapps_staking::CurrentEra::<T>::get();
                env.write(&era_index.encode(), false, None)?;
            }

            DappsStakingFunc::UnbondingPeriod => {
                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let unbonding_period = T::UnbondingPeriod::get();
                env.write(&unbonding_period.encode(), false, None)?;
            }

            DappsStakingFunc::EraRewards => {
                let arg: u32 = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let era_info = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg);
                let reward = era_info.map_or(Zero::zero(), |r| {
                    r.rewards.stakers.saturating_add(r.rewards.dapps)
                });
                env.write(&reward.encode(), false, None)?;
            }

            DappsStakingFunc::EraStaked => {
                let arg: u32 = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let era_info = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg);
                let staked_amount = era_info.map_or(Zero::zero(), |r| r.staked);
                env.write(&staked_amount.encode(), false, None)?;
            }

            DappsStakingFunc::StakedAmount => {
                let staker: T::AccountId = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let ledger = pallet_dapps_staking::Ledger::<T>::get(&staker);
                env.write(&ledger.locked.encode(), false, None)?;
            }

            DappsStakingFunc::StakedAmountOnContract => {
                let args: DappsStakingAccountInput = env.read_as()?;
                let staker: T::AccountId = args.staker.into();
                let contract: <T as pallet_dapps_staking::Config>::SmartContract =
                    args.contract.into();

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let staking_info =
                    pallet_dapps_staking::GeneralStakerInfo::<T>::get(&staker, &contract);
                let staked_amount = staking_info.latest_staked_value();
                env.write(&staked_amount.encode(), false, None)?;
            }

            DappsStakingFunc::ReadContractStake => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract: <T as pallet_dapps_staking::Config>::SmartContract =
                    contract_bytes.into();

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight.saturating_add(base_weight))?;

                let current_era = pallet_dapps_staking::CurrentEra::<T>::get();
                let staking_info =
                    pallet_dapps_staking::Pallet::<T>::contract_stake_info(&contract, current_era)
                        .unwrap_or_default();
                let total = TryInto::<u128>::try_into(staking_info.total).unwrap_or(0);
                env.write(&total.encode(), false, None)?;
            }

            DappsStakingFunc::BondAndStake => {
                let args: DappsStakingValueInput<BalanceOf<T>> = env.read_as()?;
                let contract = args.contract.into();
                let value: BalanceOf<T> = args.value;

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::bond_and_stake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::bond_and_stake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::UnbondAndUnstake => {
                let args: DappsStakingValueInput<BalanceOf<T>> = env.read_as()?;
                let contract = args.contract.into();
                let value: BalanceOf<T> = args.value;

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::unbond_and_unstake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::unbond_and_unstake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::WithdrawUnbonded => {
                let caller = env.ext().address().clone();

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::withdraw_unbonded();
                env.charge_weight(base_weight)?;

                let call_result = pallet_dapps_staking::Pallet::<T>::withdraw_unbonded(
                    RawOrigin::Signed(caller).into(),
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::ClaimStaker => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = contract_bytes.into();

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::claim_staker_with_restake()
                    .max(<T as pallet_dapps_staking::Config>::WeightInfo::claim_staker_without_restake());
                let charged_weight = env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::claim_staker(
                    RawOrigin::Signed(caller).into(),
                    contract,
                );

                let actual_weight = match call_result {
                    Ok(e) => e.actual_weight,
                    Err(e) => e.post_info.actual_weight,
                };
                if let Some(actual_weight) = actual_weight {
                    env.adjust_weight(charged_weight, actual_weight);
                }

                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::ClaimDapp => {
                let args: DappsStakingEraInput = env.read_as()?;
                let contract = args.contract.into();
                let era: u32 = args.era;

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::claim_dapp();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::claim_dapp(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    era,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::SetRewardDestination => {
                let reward_destination_raw: u8 = env.read_as()?;

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::set_reward_destination();
                env.charge_weight(base_weight)?;

                // Transform raw value into dapps staking enum
                let reward_destination = if reward_destination_raw == 0 {
                    RewardDestination::FreeBalance
                } else if reward_destination_raw == 1 {
                    RewardDestination::StakeBalance
                } else {
                    let error = DSError::RewardDestinationValueOutOfBounds;
                    return Ok(RetVal::Converging(error as u32));
                };

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::set_reward_destination(
                    RawOrigin::Signed(caller).into(),
                    reward_destination,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::NominationTransfer => {
                let args: DappsStakingNominationInput<BalanceOf<T>> = env.read_as()?;
                let origin_smart_contract = args.origin_contract.into();
                let target_smart_contract = args.target_contract.into();
                let value: BalanceOf<T> = args.value;

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::nomination_transfer();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::nomination_transfer(
                    RawOrigin::Signed(caller).into(),
                    origin_smart_contract,
                    value,
                    target_smart_contract,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }
        }

        Ok(RetVal::Converging(DSError::Success as u32))
    }
}
