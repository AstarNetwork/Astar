use evm::{Context, ExitError, ExitSucceed};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::traits::StaticLookup;
use sp_std::{marker::PhantomData, vec::Vec};

pub struct Contracts<R>(PhantomData<R>);

impl<R: pallet_contracts::Config> Contracts<R>
where
    R::AccountId: AsRef<[u8]> + UncheckedFrom<R::Hash>,
{
    fn call(dest: R::AccountId, param: Vec<u8>) -> pallet_contracts::Call<R> {
        let source = R::Lookup::unlookup(dest);
        pallet_contracts::Call::<R>::call(source, Default::default(), 1_000_000_000, param)
    }
}

impl<R> Precompile for Contracts<R>
where
    R: pallet_evm::Config + pallet_contracts::Config,
    R::Call: From<pallet_contracts::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    R::AccountId: AsRef<[u8]> + UncheckedFrom<R::Hash>,
    R::Hash: From<sp_core::H256>,
{
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
    ) -> Result<(ExitSucceed, Vec<u8>, u64), ExitError> {
        const SELECTOR_SIZE_BYTES: usize = 4;

        if input.len() < SELECTOR_SIZE_BYTES {
            return Err(ExitError::Other("input length less than 4 bytes".into()));
        }

        // ======= Contracts.sol:Contracts =======
        //    Function signatures:
        //    d22be3ba: call(bytes32,string)
        let inner_call = match input[0..SELECTOR_SIZE_BYTES] {
            [0xd2, 0x2b, 0xe3, 0xba] => {
                if input.len() < SELECTOR_SIZE_BYTES + 32 * 3 {
                    return Err(ExitError::Other("input length less than 36 bytes".into()));
                }
                // Low level argument parsing
                let dest = sp_core::H256::from_slice(
                    &input[SELECTOR_SIZE_BYTES..(SELECTOR_SIZE_BYTES + 32)],
                );
                let len_offset = SELECTOR_SIZE_BYTES + 32 * 2;
                let param_offset = len_offset + 32;
                let param_len = sp_core::U256::from_big_endian(&input[len_offset..param_offset]);
                let param = input[param_offset..(param_offset + param_len.as_usize())].to_vec();
                Self::call(R::AccountId::unchecked_from(dest.into()), param)
            }
            _ => {
                return Err(ExitError::Other(
                    "No method at selector given selector".into(),
                ))
            }
        };
        let outer_call: R::Call = inner_call.into();
        let info = outer_call.get_dispatch_info();

        if let Some(gas_limit) = target_gas {
            let required_gas = R::GasWeightMapping::weight_to_gas(info.weight);
            if required_gas > gas_limit {
                return Err(ExitError::OutOfGas);
            }
        }

        let origin = R::AddressMapping::into_account_id(context.caller);
        let post_info = outer_call
            .dispatch(Some(origin).into())
            .map_err(|_| ExitError::Other("Method call via EVM failed".into()))?;

        let gas_used =
            R::GasWeightMapping::weight_to_gas(post_info.actual_weight.unwrap_or(info.weight));
        Ok((ExitSucceed::Stopped, Default::default(), gas_used))
    }
}
