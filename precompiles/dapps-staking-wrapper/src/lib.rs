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


#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub struct DappsStakingWrapper<R>(PhantomData<R>);

// impl<R> DappsStakingWrapper<R>
// where
//     R: pallet_evm::Config + pallet_dapps_staking::Config,
//     R::Call: From<pallet_dapps_staking::Call<R>>,
// {
//     fn current_era() -> Result<PrecompileOutput, ExitError> {
//         let current_era = pallet_dapps_staking::CurrentEra::<R>::get();
//         let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);
//         println!(
//             "DappsStaking response current_era era={:?} gas_used={:?}",
//             current_era, gas_used
//         );

//         let mut buffer = [0u8; 32];
//         buffer[32 - core::mem::size_of::<u32>()..].copy_from_slice(&current_era.to_be_bytes());
//         let output = buffer.to_vec();
//         println!("DappsStakingWrapper current_era in bytes {:?}", output);

//         Ok(PrecompileOutput {
//             exit_status: ExitSucceed::Returned,
//             cost: gas_used,
//             output,
//             logs: Default::default(),
//         })
//     }
// }

impl<R> DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config + frame_system::Config,
    <R as frame_system::Config>::Call: From<pallet_dapps_staking::Call<R>>,
{
    fn current_era() -> Result<PrecompileOutput, ExitError> {
        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();
        let gas_used = R::GasWeightMapping::weight_to_gas(<R as frame_system::Config>::DbWeight::get().read);
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
}

impl<R> Precompile for DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config + frame_system::Config,
    <R as frame_system::Config>::Call: From<pallet_dapps_staking::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin: From<Option<<R as frame_system::Config>::AccountId>>,
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
            _ => {
                println!("!!!!!!!!!!! ERROR selector");
                return Err(ExitError::Other(
                    "No method at selector given selector".into(),
                ))
            }
        };
    }
}