//! Astar collator staking interface.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Decode;
use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use sp_std::{marker::PhantomData, vec::Vec};

pub struct Staking<R>(PhantomData<R>);

impl<R> Staking<R>
where
    R: pallet_session::Config + pallet_collator_selection::Config,
    R::Call: From<pallet_session::Call<R>> + From<pallet_collator_selection::Call<R>>,
{
    fn set_keys(keys: Vec<u8>) -> Result<R::Call, ExitError> {
        let keys = <R as pallet_session::Config>::Keys::decode(&mut &keys[..])
            .map_err(|_| ExitError::Other("Unable to decode session keys".into()))?;
        Ok(pallet_session::Call::<R>::set_keys {
            keys,
            proof: Default::default(),
        }
        .into())
    }

    fn purge_keys() -> R::Call {
        pallet_session::Call::<R>::purge_keys {}.into()
    }

    fn register_as_candidate() -> R::Call {
        pallet_collator_selection::Call::<R>::register_as_candidate {}.into()
    }
}

impl<R> Precompile for Staking<R>
where
    R: pallet_evm::Config + pallet_session::Config + pallet_collator_selection::Config,
    R::Call: From<pallet_session::Call<R>>
        + From<pallet_collator_selection::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
{
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
    ) -> Result<PrecompileOutput, ExitError> {
        const SELECTOR_SIZE_BYTES: usize = 4;

        if input.len() < SELECTOR_SIZE_BYTES {
            return Err(ExitError::Other("input length less than 4 bytes".into()));
        }

        // ======= Staking.sol:Staking =======
        // Function signatures:
        // bcb24ddc: set_keys(bytes)
        // 321c9b7a: purge_keys()
        // d09b6ba5: register_as_candidate()
        let call = match input[0..SELECTOR_SIZE_BYTES] {
            [0xbc, 0xb2, 0x4d, 0xdc] => {
                if input.len() < SELECTOR_SIZE_BYTES + 32 * 2 {
                    return Err(ExitError::Other("input length less than 36 bytes".into()));
                }
                // Low level argument parsing
                let len_offset = SELECTOR_SIZE_BYTES + 32;
                let keys_offset = len_offset + 32;
                let keys_len = sp_core::U256::from_big_endian(&input[len_offset..keys_offset]);
                let keys = input[keys_offset..(keys_offset + keys_len.as_usize())].to_vec();
                Self::set_keys(keys)?
            }
            [0x32, 0x1c, 0x9b, 0x7a] => Self::purge_keys(),
            [0xd0, 0x9b, 0x6b, 0xa5] => Self::register_as_candidate(),
            _ => {
                return Err(ExitError::Other(
                    "No method at selector given selector".into(),
                ))
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
        let post_info = call
            .dispatch(Some(origin).into())
            .map_err(|_| ExitError::Other("Method call via EVM failed".into()))?;

        let gas_used =
            R::GasWeightMapping::weight_to_gas(post_info.actual_weight.unwrap_or(info.weight));
        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Stopped,
            cost: gas_used,
            output: Default::default(),
            logs: Default::default(),
        })
    }
}
