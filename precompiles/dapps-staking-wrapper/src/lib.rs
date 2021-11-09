//! Astar dApps staking interface.

#![cfg_attr(not(feature = "std"), no_std)]

// use codec::Decode;
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::Get,
};
use pallet_evm::{GasWeightMapping, Precompile, AddressMapping};
use sp_core::{U256, H160};
use sp_runtime::traits::Zero;
use sp_std::convert::TryInto;
use sp_std::marker::PhantomData;

const SELECTOR_SIZE_BYTES: usize = 4;
const ARG_SIZE_BYTES: usize = 32;

// use utils::*;

// pub trait EvmDataTrait: Sized {
//     fn read(input: &mut EvmInput) -> Result<Self, Error>;
// }

// #[derive(Clone, Debug)]
// pub struct EvmInput<'a>{
//     pub data: &'a [u8]
// }

// impl EvmDataTrait for EvmInput {
//     fn read
// }

/// The balance type of this pallet.
pub struct DappsStakingWrapper<R>(PhantomData<R>);

impl<R> DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config + frame_system::Config,
    R::Call: From<pallet_dapps_staking::Call<R>>,
{
    fn current_era() -> Result<PrecompileOutput, ExitError> {
        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);
        println!(
            "DappsStaking response current_era era={:?} gas_used={:?}",
            current_era, gas_used
        );

        let output = Self::compose_output(current_era);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }

    fn era_reward_and_stake(input: &[u8]) -> Result<PrecompileOutput, ExitError> {
        let era = Self::get_argument(input, 1).low_u32();
        let reward_and_stake = pallet_dapps_staking::EraRewardsAndStakes::<R>::get(era);
        let (reward, staked) = if let Some(r) = reward_and_stake {
            (r.rewards, r.staked)
        } else {
            (Zero::zero(), Zero::zero())
        };
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);
        println!(
            "DappsStaking response for era={:?}, reward={:?} staked ={:?} gas_used={:?}",
            era, reward, staked, gas_used
        );

        let reward = TryInto::<u128>::try_into(reward).unwrap_or(0);
        let mut output = Self::compose_output_u128(reward);

        let staked = TryInto::<u128>::try_into(staked).unwrap_or(0);
        let mut staked_vec = Self::compose_output_u128(staked);
        output.append(&mut staked_vec);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }

    // Fetch registered contract from RegisteredDevelopers storage map
    fn registered_contract(input: &[u8]) -> Result<PrecompileOutput, ExitError> {
        let developer_h160 = Self::get_argument_h160(input, 1);
        let developer = R::AddressMapping::into_account_id(developer_h160);
        println!("************ developer_h160 {:?}", developer_h160);
        println!("************ developer public key {:?}", developer);

        // let developer = Self::get_argument_account_id(input, 1);
        let smart_contract = pallet_dapps_staking::RegisteredDevelopers::<R>::get(&developer);
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);

        println!(
            "************ developer {:?}, contract {:?}",
            developer, smart_contract
        );
        // let output = Self::compose_output(smart_contract.unwrap_or_default());

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output: Default::default(),
            logs: Default::default(),
        })
    }

    fn get_argument(input: &[u8], position: usize) -> U256 {
        let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
        let end = offset + ARG_SIZE_BYTES;
        sp_core::U256::from_big_endian(&input[offset..end])
    }

    // fn get_argument_account_id(input: &[u8], position: usize) -> R::AccountId{
    //     let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
    //     let end = offset + ARG_SIZE_BYTES;
    //     R::AccountId::decode(&mut &input[offset..end]).unwrap_or_default()
    // }

    fn get_argument_h160(input: &[u8], position: usize) -> H160 {
        let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
        let end = offset + ARG_SIZE_BYTES;
        // H160 has 20 bytes. The first 12 bytes in u256 have no meaning
        let offset_h160 = 12;
        sp_core::H160::from_slice(&input[(offset + offset_h160)..end]).into()
        // H160::from_slice(&data[12..32]).into()
    }

    fn compose_output(value: u32) -> Vec<u8> {
        let mut buffer = [0u8; ARG_SIZE_BYTES];
        buffer[32 - core::mem::size_of::<u32>()..].copy_from_slice(&value.to_be_bytes());
        buffer.to_vec()
    }

    fn compose_output_u128(value: u128) -> Vec<u8> {
        let mut buffer = [0u8; ARG_SIZE_BYTES];
        buffer[32 - core::mem::size_of::<u128>()..].copy_from_slice(&value.to_be_bytes());
        buffer.to_vec()
    }
}

impl<R> Precompile for DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    R::Call: From<pallet_dapps_staking::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
{
    fn execute(
        input: &[u8],
        _target_gas: Option<u64>,
        context: &Context,
    ) -> Result<PrecompileOutput, ExitError> {
        println!(
            "*** DappsStakingWrapper execute input={:?}, len={:?}, context.caller={:?}",
            input,
            input.len(),
            context.caller
        );
        if input.len() < SELECTOR_SIZE_BYTES {
            return Err(ExitError::Other("input length less than 4 bytes".into()));
        }

        match input[0..SELECTOR_SIZE_BYTES] {
            // storage getters
            // current_era = [215, 190, 56, 150]
            [0xd7, 0xbe, 0x38, 0x96] => return Self::current_era(),
            // era_reward_and_stake [185, 183, 14, 142]
            [0xb9, 0xb7, 0x0e, 0x8e] => return Self::era_reward_and_stake(input),
            // registered_contract [0x19, 0x2f, 0xb2, 0x56] 'address Developer'
            // registered_contract [0x60, 0x57, 0x36, 0x1d] 'uint256 Developer'
            [0x19, 0x2f, 0xb2, 0x56] => return Self::registered_contract(input),
            // [0x32, 0x1c, 0x9b, 0x7a] => Self::register(input),
            _ => {
                println!("!!!!!!!!!!! ERROR selector, input={:?}", input);
                return Err(ExitError::Other(
                    "No method at selector given selector".into(),
                ));
            }
        };

        // let info = call.get_dispatch_info();
        // if let Some(gas_limit) = target_gas {
        //     let required_gas = R::GasWeightMapping::weight_to_gas(info.weight);
        //     if required_gas > gas_limit {
        //         return Err(ExitError::OutOfGas);
        //     }
        // }

        // let origin = R::AddressMapping::into_account_id(context.caller);
        // let post_info = call
        //     .dispatch(Some(origin).into())
        //     .map_err(|_| ExitError::Other("Method call via EVM failed".into()))?;

        // let gas_used =
        //     R::GasWeightMapping::weight_to_gas(post_info.actual_weight.unwrap_or(info.weight));

        // Ok(PrecompileOutput {
        //     exit_status: ExitSucceed::Stopped,
        //     cost: target_gas.unwrap_or(0),
        //     output: Default::default(),
        //     logs: Default::default(),
        // })
    }
}
