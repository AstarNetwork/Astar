use crate::mock::{
    advance_to_era, default_context, evm_call, exit_error, initialize_first_block,
    precompile_address, Call, DappsStaking, EraIndex, ExternalityBuilder, Origin, Precompiles,
    TestAccount, AST, BLOCK_REWARD, UNBONDING_PERIOD, *,
};
use codec::Encode;
use fp_evm::PrecompileOutput;
use frame_support::{assert_ok, dispatch::Dispatchable};
use pallet_evm::{ExitSucceed, PrecompileSet};
use sha3::{Digest, Keccak256};
use sp_runtime::Perbill;
use std::collections::BTreeMap;

use crate::utils;
use codec::Decode;

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

        let expected = Some(Err(exit_error("No method at given selector")));

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

        let selector = &Keccak256::digest(b"read_current_era()")[0..4];
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
fn read_unbonding_period_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        let selector = &Keccak256::digest(b"read_unbonding_period()")[0..4];
        let mut expected_unbonding_period = vec![0u8; 32];
        expected_unbonding_period[31] = UNBONDING_PERIOD as u8;

        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_unbonding_period,
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
fn read_era_reward_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // build input for the call
        let selector = &Keccak256::digest(b"read_era_reward(uint32)")[0..4];
        let mut input_data = Vec::<u8>::from([0u8; 36]);
        input_data[0..4].copy_from_slice(&selector);
        let era = [0u8; 32];
        input_data[4..36].copy_from_slice(&era);

        // build expected outcome
        let reward = BLOCK_REWARD;
        let expected_output = utils::argument_from_u128(reward);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_output,
            cost: Default::default(),
            logs: Default::default(),
        }));

        // verify that argument check is done in read_era_reward()
        assert_eq!(
            Precompiles::execute(precompile_address(), &selector, None, &default_context()),
            Some(Err(exit_error("Too few arguments")))
        );

        // execute and verify read_era_reward() query
        assert_eq!(
            Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
            expected
        );
    });
}

#[test]
fn read_era_staked_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // build input for the call
        let selector = &Keccak256::digest(b"read_era_staked(uint32)")[0..4];
        let mut input_data = Vec::<u8>::from([0u8; 36]);
        input_data[0..4].copy_from_slice(&selector);
        let era = [0u8; 32];
        input_data[4..36].copy_from_slice(&era);

        // build expected outcome
        let staked = 0;
        let expected_output = utils::argument_from_u128(staked);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_output,
            cost: Default::default(),
            logs: Default::default(),
        }));

        // verify that argument check is done in read_era_staked()
        assert_eq!(
            Precompiles::execute(precompile_address(), &selector, None, &default_context()),
            Some(Err(exit_error("Too few arguments")))
        );

        // execute and verify read_era_staked() query
        assert_eq!(
            Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
            expected
        );
    });
}

#[test]
fn read_era_reward_too_many_arguments_nok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // build input for the call
        let selector = &Keccak256::digest(b"read_era_reward(uint32)")[0..4];
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
fn error_mapping_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![(TestAccount::Alex, 200 * AST)])
        .build()
        .execute_with(|| {
            initialize_first_block();
            let developer = TestAccount::Alex;
            register_and_verify(developer.clone(), TEST_CONTRACT);

            // attempt to register the same contract
            let selector = &Keccak256::digest(b"register(address)")[0..4];
            let mut input_data = Vec::<u8>::from([0u8; 36]);
            input_data[0..4].copy_from_slice(&selector);
            input_data[16..36].copy_from_slice(&TEST_CONTRACT);
            let expected = Some(Err(exit_error("AlreadyRegisteredContract")));

            assert_eq!(
                Precompiles::execute(precompile_address(), &input_data, None, &default_context(),),
                expected
            );
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
            register_and_verify(developer, TEST_CONTRACT);
        });
}

#[test]
fn bond_and_stake_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex, 200 * AST),
            (TestAccount::Bobo, 200 * AST),
            (TestAccount::Dino, 100 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register new contract by Alex
            let developer = TestAccount::Alex;
            register_and_verify(developer, TEST_CONTRACT);

            let amount_staked_bobo = 100 * AST;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            let amount_staked_dino = 50 * AST;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            let mut stakers_map = BTreeMap::new();
            stakers_map.insert(TestAccount::Bobo, amount_staked_bobo);
            stakers_map.insert(TestAccount::Dino, amount_staked_dino);

            let era = 1;
            contract_era_stake_verify(TEST_CONTRACT, amount_staked_bobo + amount_staked_dino, era);
            contract_era_stakers_verify(TEST_CONTRACT, era, stakers_map);
        });
}

#[test]
fn unbond_and_unstake_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex, 200 * AST),
            (TestAccount::Bobo, 200 * AST),
            (TestAccount::Dino, 100 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register new contract by Alex
            let developer = TestAccount::Alex;
            register_and_verify(developer, TEST_CONTRACT);

            let amount_staked_bobo = 100 * AST;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);
            let amount_staked_dino = 50 * AST;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            // Bobo unstakes all
            let era = 2;
            advance_to_era(era);
            unbond_unstake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            let mut stakers_map = BTreeMap::new();
            stakers_map.insert(TestAccount::Dino, amount_staked_dino);
            // staking_info_verify(contract_array, amount_staked_dino, era, stakers_map);
            contract_era_stake_verify(TEST_CONTRACT, amount_staked_dino, era);
            contract_era_stakers_verify(TEST_CONTRACT, era, stakers_map);

            // withdraw unbonded funds
            advance_to_era(era + UNBONDING_PERIOD + 1);
            withdraw_unbonded_verify(TestAccount::Bobo);
        });
}

#[test]
fn claim_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex, 200 * AST),
            (TestAccount::Bobo, 200 * AST),
            (TestAccount::Dino, 200 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register new contract by Alex
            let developer = TestAccount::Alex;
            register_and_verify(developer, TEST_CONTRACT);

            let stake_amount_total = 300 * AST;
            let ratio_bobo = Perbill::from_rational(3u32, 5u32);
            let ratio_dino = Perbill::from_rational(2u32, 5u32);
            let amount_staked_bobo = ratio_bobo * stake_amount_total;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            let amount_staked_dino = ratio_dino * stake_amount_total;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            // advance era and claim reward
            let era = 5;
            advance_to_era(era);
            claim_and_verify(TEST_CONTRACT, era - 1);

            //check that the reward is payed out to the stakers and the developer
            let developer_reward = Perbill::from_percent(DEVELOPER_REWARD_PERCENTAGE)
                * BLOCK_REWARD
                * BLOCKS_PER_ERA as u128
                - REGISTER_DEPOSIT;
            let stakers_reward = Perbill::from_percent(100 - DEVELOPER_REWARD_PERCENTAGE)
                * BLOCK_REWARD
                * BLOCKS_PER_ERA as u128;
            let bobo_reward = ratio_bobo * stakers_reward;
            let dino_reward = ratio_dino * stakers_reward;
            assert_eq!(
                <TestRuntime as pallet_evm::Config>::Currency::free_balance(TestAccount::Alex),
                (200 * AST) + developer_reward
            );
            assert_eq!(
                <TestRuntime as pallet_evm::Config>::Currency::free_balance(TestAccount::Bobo),
                (200 * AST) + bobo_reward
            );
            assert_eq!(
                <TestRuntime as pallet_evm::Config>::Currency::free_balance(TestAccount::Dino),
                (200 * AST) + dino_reward
            );
        });
}

// ****************************************************************************************************
// Helper functions
// ****************************************************************************************************

/// helper function to register and verify if registration is valid
fn register_and_verify(developer: TestAccount, contract_array: [u8; 20]) {
    let selector = &Keccak256::digest(b"register(address)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 36]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);

    // verify that argument check is done in register()
    assert_ok!(Call::Evm(evm_call(developer.clone(), selector.to_vec())).dispatch(Origin::root()));

    // call register()
    assert_ok!(Call::Evm(evm_call(developer.clone(), input_data)).dispatch(Origin::root()));

    // check the storage after the register
    let smart_contract_bytes =
        (DappsStaking::registered_contract(developer).unwrap_or_default()).encode();
    assert_eq!(
        smart_contract_bytes,
        to_smart_contract_bytes(contract_array)
    );

    // check_register_event(developer, contract_h160);
}

/// transform 20 byte array (h160) to smart contract encoded 21 bytes
pub fn to_smart_contract_bytes(input: [u8; 20]) -> [u8; 21] {
    let mut smart_contract_bytes = [0u8; 21];
    // prepend enum byte to the H160
    // enum for SmartContract::H160 is 0
    smart_contract_bytes[0] = 0;
    smart_contract_bytes[1..21].copy_from_slice(&input[0..20]);

    smart_contract_bytes
}

/// helper function to read ledger storage item
fn read_staked_amount_verify(staker: TestAccount, amount: u128) {
    let selector = &Keccak256::digest(b"read_staked_amount(address)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 36]);
    input_data[0..4].copy_from_slice(&selector);

    let staker_arg = utils::argument_from_h160(staker.to_h160());

    input_data[4..36].copy_from_slice(&staker_arg);

    let expected = Some(Ok(PrecompileOutput {
        exit_status: ExitSucceed::Returned,
        output: utils::argument_from_u128(amount),
        cost: Default::default(),
        logs: Default::default(),
    }));

    // verify that argument check is done in registered_contract
    assert_eq!(
        Precompiles::execute(precompile_address(), &selector, None, &default_context()),
        Some(Err(exit_error("Too few arguments")))
    );

    assert_eq!(
        Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
        expected
    );
}

/// helper function to bond, stake and verify if resulet is OK
fn bond_stake_and_verify(staker: TestAccount, contract_array: [u8; 20], amount: u128) {
    let selector = &Keccak256::digest(b"bond_and_stake(address,uint128)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let staking_amount = amount.to_be_bytes();
    input_data[(68 - staking_amount.len())..68].copy_from_slice(&staking_amount);

    // verify that argument check is done in bond_and_stake()
    assert_ok!(Call::Evm(evm_call(staker.clone(), selector.to_vec())).dispatch(Origin::root()));

    // call bond_and_stake()
    assert_ok!(Call::Evm(evm_call(staker.clone(), input_data)).dispatch(Origin::root()));

    read_staked_amount_verify(staker.clone(), amount.clone());
    is_in_astarbase_is_ok(staker, amount);
}

/// helper function to unbond, unstake and verify if resulet is OK
fn unbond_unstake_and_verify(staker: TestAccount, contract_array: [u8; 20], amount: u128) {
    let selector = &Keccak256::digest(b"unbond_and_unstake(address,uint128)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let staking_amount = amount.to_be_bytes();
    input_data[(68 - staking_amount.len())..68].copy_from_slice(&staking_amount);

    // verify that argument check is done in unbond_unstake()
    assert_ok!(Call::Evm(evm_call(staker.clone(), selector.to_vec())).dispatch(Origin::root()));

    // call unbond_and_unstake()
    assert_ok!(Call::Evm(evm_call(staker.clone(), input_data.clone())).dispatch(Origin::root()));

    read_staked_amount_verify(staker.clone(), amount.clone());
    is_in_astarbase_is_ok(staker, amount);
}

/// helper function to withdraw unstaked funds and verify if resulet is OK
fn withdraw_unbonded_verify(staker: TestAccount) {
    let selector = &Keccak256::digest(b"withdraw_unbonded()")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 4]);
    input_data[0..4].copy_from_slice(&selector);

    // call unbond_and_unstake(). Check usable_balance before and after the call
    assert_ne!(
        <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker),
        <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker)
    );
    assert_ok!(Call::Evm(evm_call(staker.clone(), input_data)).dispatch(Origin::root()));
    assert_eq!(
        <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker),
        <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker)
    );
}

/// helper function to bond, stake and verify if resulet is OK
fn claim_and_verify(contract_array: [u8; 20], era: EraIndex) {
    let staker = TestAccount::Bobo;
    let selector = &Keccak256::digest(b"claim(address,uint128)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let era_array = era.to_be_bytes();
    input_data[(68 - era_array.len())..68].copy_from_slice(&era_array);

    // verify that argument check is done in claim()
    assert_ok!(Call::Evm(evm_call(staker.clone(), selector.to_vec())).dispatch(Origin::root()));

    // call bond_and_stake()
    assert_ok!(Call::Evm(evm_call(staker.clone(), input_data)).dispatch(Origin::root()));
}

fn contract_era_stake_verify(contract_array: [u8; 20], amount: u128, era: EraIndex) {
    // prepare input to read staked amount on the contract
    let selector = &Keccak256::digest(b"read_contract_era_stake(address,uint32)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let mut era_vec = Vec::<u8>::from([0u8; 32]);
    era_vec[31] = era as u8;
    input_data[(68 - era_vec.len())..68].copy_from_slice(&era_vec);

    // Compose expected outcome: add total stake on contract
    let total = amount;
    let expected_output = utils::argument_from_u128(total);
    let expected = Some(Ok(PrecompileOutput {
        exit_status: ExitSucceed::Returned,
        output: expected_output,
        cost: Default::default(),
        logs: Default::default(),
    }));

    // verify that argument check is done in read_contract_era_stake
    assert_eq!(
        Precompiles::execute(precompile_address(), &selector, None, &default_context()),
        Some(Err(exit_error("Too few arguments")))
    );

    // execute and verify read_contract_era_stake() query
    assert_eq!(
        Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
        expected
    );
}

/// helper function to check if (un)bonding was successful
fn contract_era_stakers_verify(
    contract_array: [u8; 20],
    era: EraIndex,
    expected_stakers_map: BTreeMap<TestAccount, u128>,
) {
    // check the storage
    let smart_contract = decode_smart_contract_from_array(contract_array).unwrap();
    let staking_info = DappsStaking::contract_era_stake(&smart_contract, era).unwrap_or_default();
    let stakers = staking_info.stakers;
    assert_eq!(expected_stakers_map, stakers);
}

/// Helper method to decode type SmartContract enum from [u8; 20]
fn decode_smart_contract_from_array(
    contract_array: [u8; 20],
) -> Result<<TestRuntime as pallet_dapps_staking::Config>::SmartContract, String> {
    // Encode contract address to fit SmartContract enum.
    let mut contract_enum_encoded: [u8; 21] = [0; 21];
    contract_enum_encoded[0] = 0; // enum for EVM H160 address is 0
    contract_enum_encoded[1..21].copy_from_slice(&contract_array);

    let smart_contract = <TestRuntime as pallet_dapps_staking::Config>::SmartContract::decode(
        &mut &contract_enum_encoded[..21],
    )
    .map_err(|_| "Error while decoding SmartContract")?;

    Ok(smart_contract)
}

/// test for Astarbase.sol
fn is_in_astarbase_is_ok(staker: TestAccount, amount: u128) {
    let selector = &Keccak256::digest(b"is_in_astarbase(address)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 36]);
    input_data[0..4].copy_from_slice(&selector);

    let staker_arg = utils::argument_from_h160(staker.to_h160());

    input_data[4..36].copy_from_slice(&staker_arg);

    let expected = Some(Ok(PrecompileOutput {
        exit_status: ExitSucceed::Returned,
        output: utils::argument_from_u128(amount),
        cost: Default::default(),
        logs: Default::default(),
    }));

    // verify that argument check is done
    assert_eq!(
        Precompiles::execute(precompile_address(), &selector, None, &default_context()),
        Some(Err(exit_error("Too few arguments")))
    );

    assert_eq!(
        Precompiles::execute(precompile_address(), &input_data, None, &default_context()),
        expected
    );
}
