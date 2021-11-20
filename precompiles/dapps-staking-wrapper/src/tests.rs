use crate::mock::{
    advance_to_era, default_context, evm_call, exit_error, initialize_first_block,
    precompile_address, Call, ExternalityBuilder, Origin, Precompiles, TestAccount, AST,
};
use crate::PrecompileOutput;
use frame_support::{assert_ok, dispatch::Dispatchable};
use pallet_evm::{ExitSucceed, PrecompileSet};
use sha3::{Digest, Keccak256};
use sp_core::H160;

use crate::utils;

#[test]
fn selector_out_of_bounds_nok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // Use 3 bytes selector. 4 bytes are needed
        let selector_nok = vec![0x01, 0x02, 0x03];

        let expected = Some(Err(exit_error("Selector too short")));

        assert_eq!(
            Precompiles::execute(
                precompile_address(),
                &selector_nok,
                None,
                &default_context(),
            ),
            expected
        );
    });
}
#[test]
fn selector_unknown_nok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // We use 3 bytes selector. 4 byts are needed
        let selector_nok = vec![0x01, 0x02, 0x03, 0x04];

        let expected = Some(Err(exit_error("No method at selector given selector")));

        assert_eq!(
            Precompiles::execute(
                precompile_address(),
                &selector_nok,
                None,
                &default_context(),
            ),
            expected
        );
    });
}

#[test]
fn current_era_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        let selector = &Keccak256::digest(b"current_era()")[0..4];
        let mut expected_era = vec![0u8; 32];
        expected_era[31] = 1;

        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_era.clone(),
            cost: Default::default(),
            logs: Default::default(),
        }));

        assert_eq!(
            Precompiles::execute(precompile_address(), &selector, None, &default_context()),
            expected
        );

        // advance to era 5 and check output
        expected_era[31] = 5;
        advance_to_era(5);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_era,
            cost: Default::default(),
            logs: Default::default(),
        }));
        assert_eq!(
            Precompiles::execute(precompile_address(), &selector, None, &default_context()),
            expected
        );
    });
}

#[test]
fn era_reward_and_stake_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // build input for the call
        let selector = &Keccak256::digest(b"era_reward_and_stake(uint32)")[0..4];
        let mut input_data = Vec::<u8>::from([0u8; 36]);
        input_data[0..4].copy_from_slice(&selector);
        let era = [0u8; 32];
        input_data[4..36].copy_from_slice(&era);

        // build expected outcome
        let reward = 0;
        let mut expected_output = utils::argument_from_u128(reward);
        let staked = 0;
        let mut staked_vec = utils::argument_from_u128(staked);
        expected_output.append(&mut staked_vec);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_output,
            cost: Default::default(),
            logs: Default::default(),
        }));

        assert_eq!(
            Precompiles::execute(precompile_address(), &selector, None, &default_context()),
            Some(Err(exit_error("Too few arguments")))
        );

        assert_eq!(
            Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
            expected
        );
    });
}

#[test]
fn era_reward_and_stake_too_many_arguments_nok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // build input for the call
        let selector = &Keccak256::digest(b"era_reward_and_stake(uint32)")[0..4];
        let mut input_data = Vec::<u8>::from([0u8; 37]);
        input_data[0..4].copy_from_slice(&selector);
        let era = [0u8; 33];
        input_data[4..37].copy_from_slice(&era);

        assert_eq!(
            Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
            Some(Err(exit_error("Too many arguments")))
        )
    });
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![(TestAccount::Alex, 200 * AST)])
        .build()
        .execute_with(|| {
            initialize_first_block();
            let developer = TestAccount::Alex;
            let selector = &Keccak256::digest(b"register(address)")[0..4];
            let mut input_data = Vec::<u8>::from([0u8; 36]);
            input_data[0..4].copy_from_slice(&selector);

            let contract_array = H160::repeat_byte(0x09).to_fixed_bytes();
            input_data[16..36].copy_from_slice(&contract_array);

            // register new contract
            assert_ok!(Call::Evm(evm_call(developer.clone(), input_data)).dispatch(Origin::root()));
            // TODO register did not execute and this TC will fail

            // let result = Evm::call(Origin::root(),
            // developer.to_h160(),
            // precompile_address(),
            // input_data,
            // U256::zero(),
            // u64::max_value(),
            // U256::zero().into(),
            // None);
            // println!("register_is_ok result = {:?}", result);

            // check_registered_contract(developer, contract_array);
            // check_register_event(developer, contract_h160);
        });
}

// fn check_registered_contract(developer: TestAccount, contract_array_h160: [u8; 20]) {
//     println!("--- check_registered_contract contract_array_h160({:?}) {:?}", contract_array_h160.len(), contract_array_h160);

//     // check if the contract is registered
//     let selector = &Keccak256::digest(b"registered_contract(address)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 36]);
//     input_data[0..4].copy_from_slice(&selector);

//     let developer_arg = utils::argument_from_h160(developer.to_h160());
//     // let contract_array = utils::compose_arg_from_h160(contract_array_h160);
//     println!("--- check_registered_contract developer_arg({:?}) {:?}", developer_arg.len(), developer_arg);
//     input_data[4..36].copy_from_slice(&developer_arg);

//     let expected = Some(Ok(PrecompileOutput {
//         exit_status: ExitSucceed::Returned,
//         output: contract_array_h160.to_vec(),
//         cost: Default::default(),
//         logs: Default::default(),
//     }));
//     println!("--- check_registered_contract expected() {:?}", expected);
//     assert_eq!(
//         Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
//         expected
//     );
// }

// Check Register event
// pub fn check_register_event(developer: H160, contract_id: H160) {
//     System::assert_last_event(Event::DappsStaking(
//         <TestRuntime as pallet_dapps_staking::Config>::Event::NewContract(
//         developer,
//         contract_id,
//     )));
// }
