//! Astar dApps staking interface.

#![cfg_attr(not(feature = "std"), no_std)]

// use codec::Decode;
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::Get,
};
use pallet_evm::{GasWeightMapping, Precompile};
// use sp_core::H160;
use sp_std::{marker::PhantomData};

// use utils::*;
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

        let mut buffer = [0u8; 32];
        buffer[32 - core::mem::size_of::<u32>()..].copy_from_slice(&current_era.to_be_bytes());
        let output = buffer.to_vec();
        println!("DappsStakingWrapper current_era in bytes {:?}", output);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output,
            logs: Default::default(),
        })
    }

    // fn register(
    //     input: &[u8]
    // ) -> R::Call {
    //     let address = R::Sm
    //     ::Evm(H160::repeat_byte(0x01));
    //     let contract: R::SmartContract = address;
    //     pallet_dapps_staking::Call::<R>::register(contract).into()
    // }
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
        const SELECTOR_SIZE_BYTES: usize = 4;
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
            // [0x32, 0x1c, 0x9b, 0x7a] => Self::register(input),
            _ => {
                println!("!!!!!!!!!!! ERROR selector");
                return Err(ExitError::Other(
                    "No method at selector given selector".into(),
                ))
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
