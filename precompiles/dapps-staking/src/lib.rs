//! Astar dApps staking interface.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use fp_evm::{Context, ExitError, ExitSucceed, PrecompileOutput};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::Get,
};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use sp_core::H160;
use sp_runtime::traits::{SaturatedConversion, Zero};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};
extern crate alloc;

mod utils;
pub use utils::*;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub struct DappsStakingWrapper<R>(PhantomData<R>);

impl<R> DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    R::Call: From<pallet_dapps_staking::Call<R>>,
{
    /// Fetch current era from CurrentEra storage map
    fn read_current_era() -> Result<PrecompileOutput, ExitError> {
        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);

        let output = utils::argument_from_u32(current_era);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }
    /// Fetch unbonding period
    fn read_unbonding_period() -> Result<PrecompileOutput, ExitError> {
        let unbonding_period = R::UnbondingPeriod::get();
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);

        let output = utils::argument_from_u32(unbonding_period);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }

    /// Fetch reward from EraRewardsAndStakes storage map
    fn read_era_reward(input: EvmInArg) -> Result<PrecompileOutput, ExitError> {
        input.expecting_arguments(1).map_err(|e| exit_error(e))?;
        let era = input.to_u256(1).low_u32();
        let read_reward = pallet_dapps_staking::EraRewardsAndStakes::<R>::get(era);
        let reward = read_reward.map_or(Zero::zero(), |r| r.rewards);
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);

        let reward = TryInto::<u128>::try_into(reward).unwrap_or(0);
        let output = utils::argument_from_u128(reward);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }
    /// Fetch total staked amount from EraRewardsAndStakes storage map
    fn read_era_staked(input: EvmInArg) -> Result<PrecompileOutput, ExitError> {
        input.expecting_arguments(1).map_err(|e| exit_error(e))?;
        let era = input.to_u256(1).low_u32();
        let reward_and_stake = pallet_dapps_staking::EraRewardsAndStakes::<R>::get(era);
        let staked = reward_and_stake.map_or(Zero::zero(), |r| r.staked);
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);

        let staked = TryInto::<u128>::try_into(staked).unwrap_or(0);
        let output = utils::argument_from_u128(staked);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }

    /// Fetch Ledger storage map
    fn read_staked_amount(input: EvmInArg) -> Result<PrecompileOutput, ExitError> {
        input.expecting_arguments(1).map_err(|e| exit_error(e))?;
        let staker_h160 = input.to_h160(1);
        let staker = R::AddressMapping::into_account_id(staker_h160);

        // call pallet-dapps-staking
        let ledger = pallet_dapps_staking::Ledger::<R>::get(&staker);
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);

        // compose output
        let output =
            argument_from_u128(TryInto::<u128>::try_into(ledger.locked).unwrap_or_default());

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }

    /// Read the amount staked on contract in the given era
    fn read_contract_era_stake(input: EvmInArg) -> Result<PrecompileOutput, ExitError> {
        input.expecting_arguments(2).map_err(|e| exit_error(e))?;

        // parse input parameters for pallet-dapps-staking call
        let contract_h160 = input.to_h160(1);
        let contract_id = Self::decode_smart_contract(contract_h160)?;
        let era = input.to_u256(2).low_u32();

        // call pallet-dapps-staking
        let staking_info = pallet_dapps_staking::Pallet::<R>::staking_info(&contract_id, era);
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);
        // encode output with total
        let total = TryInto::<u128>::try_into(staking_info.total).unwrap_or(0);
        let output = utils::argument_from_u128(total);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }

    /// Register contract with the dapp-staking pallet
    fn register(input: EvmInArg) -> Result<R::Call, ExitError> {
        input.expecting_arguments(1).map_err(|e| exit_error(e))?;

        // parse contract's address
        let contract_h160 = input.to_h160(1);

        let contract_id = Self::decode_smart_contract(contract_h160)?;

        Ok(pallet_dapps_staking::Call::<R>::register { contract_id }.into())
    }

    /// Lock up and stake balance of the origin account.
    fn bond_and_stake(input: EvmInArg) -> Result<R::Call, ExitError> {
        input.expecting_arguments(2).map_err(|e| exit_error(e))?;

        // parse contract's address
        let contract_h160 = input.to_h160(1);
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parse balance to be staked
        let value = input.to_u256(2).low_u128().saturated_into();

        Ok(pallet_dapps_staking::Call::<R>::bond_and_stake { contract_id, value }.into())
    }

    /// Start unbonding process and unstake balance from the contract.
    fn unbond_and_unstake(input: EvmInArg) -> Result<R::Call, ExitError> {
        input.expecting_arguments(2).map_err(|e| exit_error(e))?;

        // parse contract's address
        let contract_h160 = input.to_h160(1);
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parse balance to be staked
        let value = input.to_u256(2).low_u128().saturated_into();

        Ok(pallet_dapps_staking::Call::<R>::unbond_and_unstake { contract_id, value }.into())
    }

    /// Start unbonding process and unstake balance from the contract.
    fn withdraw_unbonded() -> Result<R::Call, ExitError> {
        Ok(pallet_dapps_staking::Call::<R>::withdraw_unbonded {}.into())
    }

    /// Claim rewards for the contract in the dapp-staking pallet
    fn claim(input: EvmInArg) -> Result<R::Call, ExitError> {
        input.expecting_arguments(2).map_err(|e| exit_error(e))?;

        // parse contract's address
        let contract_h160 = input.to_h160(1);
        let contract_id = Self::decode_smart_contract(contract_h160)?;

        // parse era
        let era = input.to_u256(2).low_u128().saturated_into();

        Ok(pallet_dapps_staking::Call::<R>::claim { contract_id, era }.into())
    }

    /// Helper method to decode type SmartContract enum
    pub fn decode_smart_contract(
        contract_h160: H160,
    ) -> Result<<R as pallet_dapps_staking::Config>::SmartContract, ExitError> {
        // Encode contract address to fit SmartContract enum.
        // Since the SmartContract enum type can't be accessed from this pecompile,
        // use locally defined enum clone (see Contract enum)
        let contract_enum_encoded = Contract::<H160>::Evm(contract_h160).encode();

        // encoded enum will add one byte before the contract's address
        // therefore we need to decode len(H160) + 1 byte = 21
        let smart_contract = <R as pallet_dapps_staking::Config>::SmartContract::decode(
            &mut &contract_enum_encoded[..21],
        )
        .map_err(|_| exit_error("Error while decoding SmartContract"))?;

        Ok(smart_contract)
    }
}

impl<R> Precompile for DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config + frame_system::Config,
    <R as frame_system::Config>::Call: From<pallet_dapps_staking::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin:
        From<Option<<R as frame_system::Config>::AccountId>>,
{
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
    ) -> Result<PrecompileOutput, ExitError> {
        let input = EvmInArg::new(&input);
        let selector = input.selector().map_err(|e| exit_error(e))?;
        let call = match selector {
            // storage getters
            [0xe6, 0x08, 0xd8, 0x0b] => return Self::read_current_era(),
            [0xdb, 0x62, 0xb2, 0x01] => return Self::read_unbonding_period(),
            [0xd9, 0x42, 0x4b, 0x16] => return Self::read_era_reward(input),
            [0x18, 0x38, 0x66, 0x93] => return Self::read_era_staked(input),
            [0x32, 0xbc, 0x5c, 0xa2] | [0xbd, 0x2b, 0x1d, 0x4c] => {
                return Self::read_staked_amount(input)
            }
            [0x2e, 0x7e, 0x8f, 0x15] => return Self::read_contract_era_stake(input),

            // extrinsic calls
            [0x44, 0x20, 0xe4, 0x86] => Self::register(input)?,
            [0x52, 0xb7, 0x3e, 0x41] => Self::bond_and_stake(input)?,
            [0xc7, 0x84, 0x1d, 0xd2] => Self::unbond_and_unstake(input)?,
            [0x77, 0xa0, 0xfe, 0x02] => Self::withdraw_unbonded()?,
            [0xc1, 0x3f, 0x4a, 0xf7] => Self::claim(input)?,
            _ => {
                return Err(ExitError::Other("No method at given selector".into()));
            }
        };

        let info = call.get_dispatch_info();
        if let Some(gas_limit) = target_gas {
            let required_gas = R::GasWeightMapping::weight_to_gas(info.weight);

            if required_gas > gas_limit {
                return Err(ExitError::OutOfGas);
            }
        }

        let origin = R::AddressMapping::into_account_id(context.caller);
        let post_info = call.dispatch(Some(origin).into()).map_err(|e| {
            let error_text = match e.error {
                sp_runtime::DispatchError::Module { message, .. } => message,
                _ => Some("No error Info"),
            };
            exit_error(error_text.unwrap_or_default())
        })?;

        let gas_used =
            R::GasWeightMapping::weight_to_gas(post_info.actual_weight.unwrap_or(info.weight));

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output: Default::default(),
            logs: Default::default(),
        })
    }
}
