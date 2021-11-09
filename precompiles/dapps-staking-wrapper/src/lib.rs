//! Astar dApps staking interface.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};

use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::Get,
};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use sp_core::{H160, U256};
use sp_runtime::traits::Zero;
use sp_std::convert::TryInto;
use sp_std::marker::PhantomData;

const SELECTOR_SIZE_BYTES: usize = 4;
const ARG_SIZE_BYTES: usize = 32;
const OFFSET_H160: usize = 12;
// use utils::*;

/// Multi-VM pointer to smart contract instance.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum Contract<A> {
    /// EVM smart contract instance.
    Evm(H160),
    /// Wasm smart contract instance. Not used in this precompile
    Wasm(A),
}

/// The balance type of this pallet.
pub struct DappsStakingWrapper<R>(PhantomData<R>);

impl<R> DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config + frame_system::Config,
    R::Call: From<pallet_dapps_staking::Call<R>>,
{
    /// Fetch current era from CurrentEra storage map
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

    /// Fetch reward and stake from EraRewardsAndStakes storage map
    fn era_reward_and_stake(input: &[u8]) -> Result<PrecompileOutput, ExitError> {
        let era = Self::parse_argument_u256(input, 1).low_u32();
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

    /// Fetch registered contract from RegisteredDevelopers storage map
    fn registered_contract(input: &[u8]) -> Result<PrecompileOutput, ExitError> {
        let developer_h160 = Self::parse_argument_h160(input, 1);
        let developer = R::AddressMapping::into_account_id(developer_h160);
        println!("************ developer_h160 {:?}", developer_h160);
        println!("************ developer public key {:?}", developer);

        // let developer = Self::parse_argument_account_id(input, 1);
        let smart_contract = pallet_dapps_staking::RegisteredDevelopers::<R>::get(&developer);
        let gas_used = R::GasWeightMapping::weight_to_gas(R::DbWeight::get().read);

        println!(
            "************ developer {:?}, contract {:?}",
            developer, smart_contract
        );
        // let output = Self::compose_output(smart_contract.unwrap_or_default()); // TODO

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output: Default::default(),
            logs: Default::default(),
        })
    }

    /// Register contract with the dapp-staking pallet
    fn register(input: &[u8]) -> Result<R::Call, ExitError> {
        // parse contract's address
        let contract_h160 = Self::parse_argument_h160(input, 1);
        println!("contract_h160 {:?}", contract_h160);

        // Encode contract address to fit SmartContract enum.
        // Since the SmartContract enum type can't be accessed from this pecompile,
        // use locally defined enum clone (see Contract enum)
        let contract_enum_encoded = Contract::<H160>::Evm(contract_h160).encode();
        println!(
            "contract_enum_encoded({:?}) = {:?}",
            contract_enum_encoded.len(),
            contract_enum_encoded
        );

        // encoded enum will add one byte before the contract's address
        // therefore we need to decode len(H160) + 1 byte = 21
        let smart_contract = <R as pallet_dapps_staking::Config>::SmartContract::decode(
            &mut &contract_enum_encoded[..21],
        ).map_err(|_| ExitError::DesignatedInvalid)?;
        println!("smart_contract {:?}", smart_contract);

        Ok(pallet_dapps_staking::Call::<R>::register(smart_contract).into())
    }

    fn parse_argument_u256(input: &[u8], position: usize) -> U256 {
        let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
        let end = offset + ARG_SIZE_BYTES;
        sp_core::U256::from_big_endian(&input[offset..end])
    }

    // fn parse_argument_account_id(input: &[u8], position: usize) -> R::AccountId{
    //     let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
    //     let end = offset + ARG_SIZE_BYTES;
    //     R::AccountId::decode(&mut &input[offset..end]).unwrap_or_default()
    // }

    fn parse_argument_h160(input: &[u8], position: usize) -> H160 {
        let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
        let end = offset + ARG_SIZE_BYTES;
        // H160 has 20 bytes. The first 12 bytes in u256 have no meaning
        sp_core::H160::from_slice(&input[(offset + OFFSET_H160)..end]).into()
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
        target_gas: Option<u64>,
        context: &Context,
    ) -> Result<PrecompileOutput, ExitError> {
        println!(
            "**************** DappsStakingWrapper execute selector={:x?}, len={:?} \ncontext.caller={:?}",
            &input[0..4],
            input.len(),
            context.caller
        );
        if input.len() < SELECTOR_SIZE_BYTES {
            return Err(ExitError::Other("input length less than 4 bytes".into()));
        }

        let call = match input[0..SELECTOR_SIZE_BYTES] {
            // storage getters
            // current_era = [215, 190, 56, 150]
            [0xd7, 0xbe, 0x38, 0x96] => return Self::current_era(),
            // era_reward_and_stake [185, 183, 14, 142]
            [0xb9, 0xb7, 0x0e, 0x8e] => return Self::era_reward_and_stake(input),
            // registered_contract [0x19, 0x2f, 0xb2, 0x56] 'address Developer'
            // registered_contract [0x60, 0x57, 0x36, 0x1d] 'uint256 Developer'
            [0x19, 0x2f, 0xb2, 0x56] => return Self::registered_contract(input),
            // register [0x44, 0x20, 0xe4, 0x86]
            [0x44, 0x20, 0xe4, 0x86] => Self::register(input)?,
            _ => {
                println!("!!!!!!!!!!! ERROR selector, selector={:x?}", &input[0..4]);
                return Err(ExitError::Other(
                    "No method at selector given selector".into(),
                ));
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
        println!("---------------- gas_used ={:?}", gas_used);

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gas_used,
            output: Default::default(),
            logs: Default::default(),
        })
    }
}
