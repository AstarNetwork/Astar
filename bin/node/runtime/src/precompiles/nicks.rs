
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use evm::{Context, ExitError, ExitSucceed};
use sp_std::{marker::PhantomData, vec::Vec};

pub struct Nicks<R>(PhantomData<R>);

impl<R: pallet_nicks::Config> Nicks<R> {
    fn set_name(name: Vec<u8>) -> pallet_nicks::Call<R> {
        pallet_nicks::Call::<R>::set_name(name)
    }
}

impl<R> Precompile for Nicks<R>
where
    R: pallet_evm::Config + pallet_nicks::Config,
    R::Call: From<pallet_nicks::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
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

        // ======= Nicks.sol:Nicks =======
        // Function signatures:
        // 6b701e08: set_name(string)
        let inner_call = match input[0..SELECTOR_SIZE_BYTES] {
            [0x6b, 0x70, 0x1e, 0x08] => Self::set_name(input[SELECTOR_SIZE_BYTES..].to_vec()),
            _ => return Err(ExitError::Other("No method at selector given selector".into())),
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
        let post_info = outer_call.dispatch(Some(origin).into())
            .map_err(|_| ExitError::Other("Method call via EVM failed".into()))?;

        let gas_used = R::GasWeightMapping::weight_to_gas(
            post_info.actual_weight.unwrap_or(info.weight),
        );
        Ok((ExitSucceed::Stopped, Default::default(), gas_used))
    }
}
