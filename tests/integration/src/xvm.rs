// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

use crate::setup::*;

use sha3::{Digest, Keccak256};

use astar_primitives::{
    ethereum_checked::{CheckedEthereumTransact, CheckedEthereumTx, EthereumTxInput},
    xvm::{CallFailure, Context, FailureError, FailureReason, FailureRevert, VmId, XvmCall},
};
use fp_evm::{ExecutionInfoV2, ExitReason, ExitRevert};
use frame_support::{dispatch::PostDispatchInfo, traits::Currency, weights::Weight};
use pallet_contracts::{CollectEvents, DebugInfo, Determinism};
use pallet_contracts_primitives::{ExecReturnValue, ReturnFlags};
use parity_scale_codec::Encode;
use precompile_utils::{Bytes, EvmDataWriter};
use sp_runtime::MultiAddress;

// Build EVM revert message error data.
fn evm_revert_message_error(msg: &str) -> Vec<u8> {
    let hash = &Keccak256::digest(b"Error(string)")[..4];
    let selector = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);

    EvmDataWriter::new_with_selector(selector)
        .write(Bytes(msg.to_owned().into_bytes()))
        .build()
}

/*

pragma solidity >=0.8.2 <0.9.0;

contract Payable {
    address payable public owner;

    constructor() payable {
        owner = payable(msg.sender);
    }

    // 0xd0e30db0
    function deposit() public payable {}

    // 0x3ccfd60b
    function withdraw() public {
        uint amount = address(this).balance;
        (bool success, ) = owner.call{value: amount}("");
        require(success, "Failed to withdraw Ether");
    }
}

 */
const EVM_PAYABLE: &str = "6080604052336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506102d6806100536000396000f3fe6080604052600436106100345760003560e01c80633ccfd60b146100395780638da5cb5b14610050578063d0e30db01461007b575b600080fd5b34801561004557600080fd5b5061004e610085565b005b34801561005c57600080fd5b5061006561015b565b60405161007291906101c2565b60405180910390f35b61008361017f565b005b600047905060008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16826040516100d19061020e565b60006040518083038185875af1925050503d806000811461010e576040519150601f19603f3d011682016040523d82523d6000602084013e610113565b606091505b5050905080610157576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161014e90610280565b60405180910390fd5b5050565b60008054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b565b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b60006101ac82610181565b9050919050565b6101bc816101a1565b82525050565b60006020820190506101d760008301846101b3565b92915050565b600081905092915050565b50565b60006101f86000836101dd565b9150610203826101e8565b600082019050919050565b6000610219826101eb565b9150819050919050565b600082825260208201905092915050565b7f4661696c656420746f2077697468647261772045746865720000000000000000600082015250565b600061026a601883610223565b915061027582610234565b602082019050919050565b600060208201905081810360008301526102998161025d565b905091905056fea2646970667358221220bd8883b6a524d12ac9c29f105fdd1a0221a0436a79002f2a04e69d252596a62a64736f6c63430008120033";

/* WASM payable:

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod payable {
    #[ink(storage)]
    pub struct Payable {}

    impl Payable {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message, payable, selector = 42)]
        pub fn deposit(&self) {}
    }
}

*/
const WASM_PAYABLE_NAME: &'static str = "payable";

/* Call WASM payable:

pragma solidity >=0.8.2 <0.9.0;

interface XVM {
    function xvm_call(
        uint8 vm_id,
        bytes calldata to,
        bytes calldata input,
        uint256 value,
        uint256 storage_deposit_limit
    ) external payable returns (bool success, bytes memory data);
}

contract CallXVMPayble {
    function call_xvm_payable(bytes calldata to, bytes calldata input, uint256 value) external payable returns (bool success, bytes memory data)  {
        // Call with unlimited storage deposit limit.
        return XVM(0x0000000000000000000000000000000000005005).xvm_call(0x1F, to, input, value, 0);
    }
}

 */
const CALL_WASM_PAYBLE: &str = "608060405234801561001057600080fd5b50610632806100206000396000f3fe60806040526004361061001e5760003560e01c80634012b91414610023575b600080fd5b61003d600480360381019061003891906101a6565b610054565b60405161004b9291906102e6565b60405180910390f35b6000606061500573ffffffffffffffffffffffffffffffffffffffff1663acc57c7e601f898989898960006040518863ffffffff1660e01b81526004016100a197969594939291906103ee565b6000604051808303816000875af11580156100c0573d6000803e3d6000fd5b505050506040513d6000823e3d601f19601f820116820180604052508101906100e991906105a0565b915091509550959350505050565b6000604051905090565b600080fd5b600080fd5b600080fd5b600080fd5b600080fd5b60008083601f8401126101305761012f61010b565b5b8235905067ffffffffffffffff81111561014d5761014c610110565b5b60208301915083600182028301111561016957610168610115565b5b9250929050565b6000819050919050565b61018381610170565b811461018e57600080fd5b50565b6000813590506101a08161017a565b92915050565b6000806000806000606086880312156101c2576101c1610101565b5b600086013567ffffffffffffffff8111156101e0576101df610106565b5b6101ec8882890161011a565b9550955050602086013567ffffffffffffffff81111561020f5761020e610106565b5b61021b8882890161011a565b9350935050604061022e88828901610191565b9150509295509295909350565b60008115159050919050565b6102508161023b565b82525050565b600081519050919050565b600082825260208201905092915050565b60005b83811015610290578082015181840152602081019050610275565b60008484015250505050565b6000601f19601f8301169050919050565b60006102b882610256565b6102c28185610261565b93506102d2818560208601610272565b6102db8161029c565b840191505092915050565b60006040820190506102fb6000830185610247565b818103602083015261030d81846102ad565b90509392505050565b6000819050919050565b600060ff82169050919050565b6000819050919050565b600061035261034d61034884610316565b61032d565b610320565b9050919050565b61036281610337565b82525050565b82818337600083830152505050565b60006103838385610261565b9350610390838584610368565b6103998361029c565b840190509392505050565b6103ad81610170565b82525050565b6000819050919050565b60006103d86103d36103ce846103b3565b61032d565b610170565b9050919050565b6103e8816103bd565b82525050565b600060a082019050610403600083018a610359565b818103602083015261041681888a610377565b9050818103604083015261042b818688610377565b905061043a60608301856103a4565b61044760808301846103df565b98975050505050505050565b61045c8161023b565b811461046757600080fd5b50565b60008151905061047981610453565b92915050565b600080fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b6104bc8261029c565b810181811067ffffffffffffffff821117156104db576104da610484565b5b80604052505050565b60006104ee6100f7565b90506104fa82826104b3565b919050565b600067ffffffffffffffff82111561051a57610519610484565b5b6105238261029c565b9050602081019050919050565b600061054361053e846104ff565b6104e4565b90508281526020810184848401111561055f5761055e61047f565b5b61056a848285610272565b509392505050565b600082601f8301126105875761058661010b565b5b8151610597848260208601610530565b91505092915050565b600080604083850312156105b7576105b6610101565b5b60006105c58582860161046a565b925050602083015167ffffffffffffffff8111156105e6576105e5610106565b5b6105f285828601610572565b915050925092905056fea2646970667358221220f15cb6d78fafea8cb36c2871d64ba6fab808db64581058721f11e89d77c0af3064736f6c63430008120033";

/* Call EVM Payable:

#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::env::{DefaultEnvironment, Environment};
use ink::prelude::vec::Vec;

#[ink::contract(env = CustomEnvironment)]
mod call_xvm_payable {
    use super::*;

    #[ink(storage)]
    pub struct CallXvmPayable {}

    impl CallXvmPayable {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message, payable, selector = 42)]
        pub fn call_xvm_payable(
            &self,
            target: Vec<u8>,
            input: Vec<u8>,
        ) -> CallResult {
            let value = Self::env().transferred_value();
            // Calling EVM
            Self::env().extension().call(0x0F, target, input, value)
        }
    }
}

pub type CallResult = u32;

#[ink::chain_extension]
pub trait XvmCall {
    type ErrorCode = u32;

    #[ink(extension = 0x00010001, handle_status = false)]
    fn call(vm_id: u8, target: Vec<u8>, input: Vec<u8>, value: u128) -> CallResult;
}

pub enum CustomEnvironment {}
impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize = <DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <DefaultEnvironment as Environment>::AccountId;
    type Balance = <DefaultEnvironment as Environment>::Balance;
    type Hash = <DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = XvmCall;
}

 */
const CALL_EVM_PAYBLE_NAME: &'static str = "call_xvm_payable";

#[test]
fn evm_payable_call_via_xvm_works() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let evm_payable_addr = deploy_evm_contract(EVM_PAYABLE);

        let value = UNIT;
        assert_ok!(Xvm::call(
            Context {
                source_vm_id: VmId::Wasm,
                weight_limit: Weight::from_parts(1_000_000_000, 1024 * 1024),
            },
            VmId::Evm,
            ALICE,
            evm_payable_addr.as_ref().to_vec(),
            // Calling `deposit`
            hex::decode("d0e30db0").expect("invalid selector hex"),
            value,
            None,
        ));
        assert_eq!(
            Balances::free_balance(account_id_from(evm_payable_addr)),
            value
        );

        assert_ok!(Xvm::call(
            Context {
                source_vm_id: VmId::Wasm,
                weight_limit: Weight::from_parts(10_000_000_000, 1024 * 1024),
            },
            VmId::Evm,
            ALICE,
            evm_payable_addr.as_ref().to_vec(),
            // `Calling withdraw`
            hex::decode("3ccfd60b").expect("invalid selector hex"),
            0,
            None,
        ));
        assert_eq!(
            Balances::free_balance(account_id_from(evm_payable_addr)),
            ExistentialDeposit::get(),
        );
    });
}

#[test]
fn wasm_payable_call_via_xvm_works() {
    new_test_ext().execute_with(|| {
        let wasm_payable_addr = deploy_wasm_contract(WASM_PAYABLE_NAME);

        let prev_balance = Balances::free_balance(&wasm_payable_addr);
        let value = UNIT;
        assert_ok!(Xvm::call(
            Context {
                source_vm_id: VmId::Evm,
                weight_limit: Weight::from_parts(10_000_000_000, 1024 * 1024),
            },
            VmId::Wasm,
            ALICE,
            wasm_payable_addr.clone().encode(),
            // Calling `deposit`
            hex::decode("0000002a").expect("invalid selector hex"),
            value,
            None,
        ));
        assert_eq!(
            Balances::free_balance(wasm_payable_addr.clone()),
            value + prev_balance
        );
    });
}

#[test]
fn calling_wasm_payable_from_evm_fails_if_caller_contract_balance_below_ed() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let _ = deploy_wasm_contract(WASM_PAYABLE_NAME);
        let evm_caller_addr = deploy_evm_contract(CALL_WASM_PAYBLE);

        let value = 1_000_000_000;
        assert_ok!(EVM::call(
            RuntimeOrigin::root(),
            alith(),
            evm_caller_addr.clone(),
            // to: 0xa8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c
            // input: 0x0000002a (deposit)
            // value: 1000000000
            hex::decode("4012b914000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000003b9aca000000000000000000000000000000000000000000000000000000000000000020a8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c00000000000000000000000000000000000000000000000000000000000000040000002a00000000000000000000000000000000000000000000000000000000").expect("invalid call input hex"),
            U256::from(value),
            1_000_000,
            U256::from(DefaultBaseFeePerGas::get()),
            None,
            None,
            vec![],
        ));

        assert_eq!(
            System::events().iter().last().expect("no event found").event,
            RuntimeEvent::EVM(
                pallet_evm::Event::ExecutedFailed { address: evm_caller_addr },
            ),
        );
        // EVM caller contract balance should be unchanged.
        assert_eq!(
            Balances::free_balance(&account_id_from(evm_caller_addr)),
            0,
        );
    });
}

#[test]
fn calling_wasm_payable_from_evm_works() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let wasm_payable_callee_addr = deploy_wasm_contract(WASM_PAYABLE_NAME);
        let evm_caller_addr = deploy_evm_contract(CALL_WASM_PAYBLE);

        let _ = Balances::deposit_creating(&account_id_from(evm_caller_addr.clone()), ExistentialDeposit::get());

        let prev_wasm_payable_balance = Balances::free_balance(&wasm_payable_callee_addr);
        let value = 1_000_000_000;
        assert_ok!(EVM::call(
            RuntimeOrigin::root(),
            alith(),
            evm_caller_addr.clone(),
            // to: 0xa8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c
            // input: 0x0000002a (deposit)
            // value: 1000000000
            hex::decode("4012b914000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000003b9aca000000000000000000000000000000000000000000000000000000000000000020a8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c00000000000000000000000000000000000000000000000000000000000000040000002a00000000000000000000000000000000000000000000000000000000").expect("invalid call input hex"),
            U256::from(value),
            1_000_000,
            U256::from(DefaultBaseFeePerGas::get()),
            None,
            None,
            vec![],
        ));
        let recieved = Balances::free_balance(&wasm_payable_callee_addr) - prev_wasm_payable_balance;
        assert_eq!(recieved, value);
    });
}

#[test]
fn calling_evm_payable_from_wasm_works() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let evm_payable_callee_addr = deploy_evm_contract(EVM_PAYABLE);
        let wasm_caller_addr = deploy_wasm_contract(CALL_EVM_PAYBLE_NAME);

        let value = UNIT;

        // claim the default mappings for wasm contract
        claim_default_accounts(wasm_caller_addr.clone());

        let evm_payable = evm_payable_callee_addr.as_ref().to_vec();
        let deposit_func = hex::decode("d0e30db0").expect("invalid deposit function hex");
        let input = hex::decode("0000002a")
            .expect("invalid selector hex")
            .iter()
            .chain(evm_payable.encode().iter())
            .chain(deposit_func.encode().iter())
            .cloned()
            .collect::<Vec<_>>();
        assert_ok!(Contracts::call(
            RuntimeOrigin::signed(ALICE),
            MultiAddress::Id(wasm_caller_addr.clone()),
            value,
            Weight::from_parts(10_000_000_000, 1024 * 1024),
            None,
            input,
        ));

        assert_eq!(
            Balances::free_balance(account_id_from(evm_payable_callee_addr)),
            value
        );

        assert_eq!(
            Balances::free_balance(&wasm_caller_addr),
            ExistentialDeposit::get()
        );
    });
}

#[test]
fn reentrance_not_allowed() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        // Call path: WASM -> EVM -> WASM
        let wasm_caller_addr = deploy_wasm_contract(CALL_EVM_PAYBLE_NAME);
        let evm_caller_addr = deploy_evm_contract(CALL_WASM_PAYBLE);
        let _ = deploy_wasm_contract(WASM_PAYABLE_NAME);

        // to: 0xa8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c
        // input: 0x0000002a (deposit)
        // value: 1000000000
        let call_wasm_payable_input = hex::decode("4012b914000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000003b9aca000000000000000000000000000000000000000000000000000000000000000020a8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c00000000000000000000000000000000000000000000000000000000000000040000002a00000000000000000000000000000000000000000000000000000000").expect("invalid call input hex");
        let input = hex::decode("0000002a")
            .expect("invalid selector hex")
            .iter()
            .chain(evm_caller_addr.as_ref().to_vec().encode().iter())
            .chain(call_wasm_payable_input.encode().iter())
            .cloned()
            .collect::<Vec<_>>();

        // assert `ReentranceDenied` error
        let result = Contracts::bare_call(
            ALICE,
            wasm_caller_addr,
            0,
            Weight::from_parts(10_000_000_000, 1024 * 1024),
            None,
            input,
            DebugInfo::Skip,
            CollectEvents::Skip,
            Determinism::Enforced,
        );
        match result.result {
            Ok(ExecReturnValue { flags, data }) => {
                assert!(flags.contains(ReturnFlags::REVERT));

                let reentrance_msg_error = evm_revert_message_error(&format!("{:?}", FailureError::ReentranceDenied));
                let error_string = String::from_utf8(data).expect("invalid utf8");
                assert!(error_string.contains(&format!("{:?}", reentrance_msg_error)));
            }
            _ => panic!("unexpected wasm call result"),
        }
    });
}

/*

pragma solidity >=0.8.2 <0.9.0;

contract ShinyError {
    error TooShiny(uint256 a, uint256 star);

    function revert_with_err_msg() public pure {
        revert("too shiny");
    }

    function revert_with_err_type() public pure {
        revert TooShiny(1, 2);
    }
}

 */
const EVM_DUMMY_ERROR: &'static str = "608060405234801561001057600080fd5b50610231806100206000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c806328fd58ae1461003b578063cb1c03b214610045575b600080fd5b61004361004f565b005b61004d61008a565b005b6040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161008190610128565b60405180910390fd5b600160026040517f2cdac97f0000000000000000000000000000000000000000000000000000000081526004016100c29291906101d2565b60405180910390fd5b600082825260208201905092915050565b7f746f6f207368696e790000000000000000000000000000000000000000000000600082015250565b60006101126009836100cb565b915061011d826100dc565b602082019050919050565b6000602082019050818103600083015261014181610105565b9050919050565b6000819050919050565b6000819050919050565b6000819050919050565b600061018161017c61017784610148565b61015c565b610152565b9050919050565b61019181610166565b82525050565b6000819050919050565b60006101bc6101b76101b284610197565b61015c565b610152565b9050919050565b6101cc816101a1565b82525050565b60006040820190506101e76000830185610188565b6101f460208301846101c3565b939250505056fea26469706673582212203b6d6f183650a1e330bb63d34c4d28865e8356715721534381292e37b07c8dd664736f6c63430008120033";

#[test]
fn evm_call_via_xvm_fails_if_revert() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let evm_callee_addr = deploy_evm_contract(EVM_DUMMY_ERROR);

        let result = Xvm::call(
            Context {
                source_vm_id: VmId::Wasm,
                weight_limit: Weight::from_parts(1_000_000_000, 1024 * 1024),
            },
            VmId::Evm,
            ALICE,
            evm_callee_addr.as_ref().to_vec(),
            // Calling `revert_with_err_msg`
            hex::decode("28fd58ae").expect("invalid selector hex"),
            0,
            None,
        );
        match result {
            Err(CallFailure {
                reason: FailureReason::Revert(FailureRevert::VmRevert(data)),
                ..
            }) => {
                assert_eq!(data, evm_revert_message_error("too shiny"));
            }
            _ => panic!("unexpected evm call result: {:?}", result),
        }

        let result1 = Xvm::call(
            Context {
                source_vm_id: VmId::Wasm,
                weight_limit: Weight::from_parts(1_000_000_000, 1024 * 1024),
            },
            VmId::Evm,
            ALICE,
            evm_callee_addr.as_ref().to_vec(),
            // Calling `revert_with_err_type`
            hex::decode("cb1c03b2").expect("invalid selector hex"),
            0,
            None,
        );
        match result1 {
            Err(CallFailure {
                reason: FailureReason::Revert(FailureRevert::VmRevert(data)),
                ..
            }) => {
                // data with error type `TooShiny(uint256,uint256)` on revert: selector(4) ++ payload(32) ++ paylaod(32)
                let mut encoded = [0u8; 4 + 32 + 32];
                encoded[..4].copy_from_slice(&Keccak256::digest(b"TooShiny(uint256,uint256)")[..4]);
                U256::from(1).to_big_endian(&mut encoded[4..36]);
                U256::from(2).to_big_endian(&mut encoded[36..]);
                assert_eq!(data, encoded);
            }
            _ => panic!("unexpected evm call result: {:?}", result1),
        }
    });
}

/* Dummy Error:

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod dummy_error {

    #[ink(storage)]
    pub struct DummyError {}

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        #[codec(index = 7)]
        DummyError,
    }

    impl DummyError {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message, selector = 42)]
        pub fn do_revert(&self) -> Result<(), Error> {
            Err(Error::DummyError)
        }
    }
}

 */
const WASM_DUMMY_ERROR_NAME: &'static str = "dummy_error";

#[test]
fn wasm_call_via_xvm_fails_if_revert() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let wasm_callee_addr = deploy_wasm_contract(WASM_DUMMY_ERROR_NAME);
        let input = hex::decode("0000002a").expect("invalid selector hex");
        let result = Xvm::call(
            Context {
                source_vm_id: VmId::Evm,
                weight_limit: Weight::from_parts(10_000_000_000, 1024 * 1024),
            },
            VmId::Wasm,
            ALICE,
            wasm_callee_addr.clone().encode(),
            input,
            0,
            None,
        );
        match result {
            Err(CallFailure {
                reason: FailureReason::Revert(FailureRevert::VmRevert(data)),
                ..
            }) => {
                // `DummyError` error index is set `7` in wasm contract.
                assert_eq!(data.last(), Some(&7u8));
            }
            _ => panic!("unexpected wasm call result: {:?}", result),
        }
    });
}

#[test]
fn evm_caller_reverts_if_wasm_callee_reverted() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let _ = deploy_wasm_contract(WASM_DUMMY_ERROR_NAME);
        let evm_caller_addr = deploy_evm_contract(CALL_WASM_PAYBLE);

        // to: 0xa0565d335eb7545deeb25563471219e6f0c9b9bb504a112a5f26fe61237c5a23
        // input: 0x0000002a (do_revert)
        // value: 0
        let input = hex::decode("4012b914000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020a0565d335eb7545deeb25563471219e6f0c9b9bb504a112a5f26fe61237c5a2300000000000000000000000000000000000000000000000000000000000000040000002a00000000000000000000000000000000000000000000000000000000").expect("invalid call input hex");
        let tx = CheckedEthereumTx {
            target: evm_caller_addr.clone(),
            input: EthereumTxInput::try_from(input).expect("input too large"),
            value: U256::zero(),
            gas_limit: U256::from(1_000_000),
            maybe_access_list: None,
        };

        // Note `EVM::call` won't log details of the revert error, so we need to
        // use `EthereumChecked` here for error checks.
        match EthereumChecked::xvm_transact(alith(), tx) {
            Ok((PostDispatchInfo { .. }, ExecutionInfoV2 { exit_reason, value, .. })) => {
                assert_eq!(exit_reason, ExitReason::Revert(ExitRevert::Reverted));

                // The last item `7` of `[0, 1, 7]` indicates the `DummyError` error index.
                let revert_msg_error = evm_revert_message_error("FailureRevert::VmRevert([0, 1, 7])");
                assert_eq!(value, revert_msg_error);
            },
            _ => panic!("unexpected evm call result"),
        }
    });
}

#[test]
fn wasm_caller_reverts_if_evm_callee_reverted() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let evm_callee_addr = deploy_evm_contract(EVM_DUMMY_ERROR);
        let wasm_caller_addr = deploy_wasm_contract(CALL_EVM_PAYBLE_NAME);

        // Calling `revert_with_err_msg`
        let revert_func = hex::decode("28fd58ae").expect("invalid selector hex");
        let input = hex::decode("0000002a")
            .expect("invalid selector hex")
            .iter()
            .chain(evm_callee_addr.as_ref().to_vec().encode().iter())
            .chain(revert_func.encode().iter())
            .cloned()
            .collect::<Vec<_>>();

        // assert `too shiny` error
        let result = Contracts::bare_call(
            ALICE,
            wasm_caller_addr,
            0,
            Weight::from_parts(10_000_000_000, 1024 * 1024),
            None,
            input,
            DebugInfo::Skip,
            CollectEvents::Skip,
            Determinism::Enforced,
        );
        match result.result {
            Ok(ExecReturnValue { flags, data }) => {
                assert!(flags.contains(ReturnFlags::REVERT));

                let revert_failure = FailureReason::Revert(FailureRevert::VmRevert(
                    evm_revert_message_error("too shiny"),
                ));
                let error_string = String::from_utf8(data).expect("invalid utf8");
                assert!(error_string.contains(&format!("{:?}", revert_failure)));
            }
            _ => panic!("unexpected wasm call result"),
        }
    });
}

/*

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod simple_storage {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct SimpleStorage {
        storages: Mapping<AccountId, u32>,
    }

    impl SimpleStorage {
        #[ink(constructor)]
        pub fn new() -> Self {
            let storages = Mapping::default();
            Self { storages }
        }

        #[ink(message, selector = 42)]
        pub fn store(&mut self) {
            let caller = self.env().caller();
            self.storages.insert(caller, &42);
        }

        #[ink(message, selector = 43)]
        pub fn get(&self) -> u32 {
            let caller = self.env().caller();
            self.storages.get(caller).unwrap_or(0)
        }
    }
}

 */
const WASM_SIMPLE_STORAGE_NAME: &'static str = "simple_storage";

#[test]
fn wasm_call_via_xvm_fails_if_storage_deposit_limit_exhausted() {
    new_test_ext().execute_with(|| {
        let wasm_callee_addr = deploy_wasm_contract(WASM_SIMPLE_STORAGE_NAME);
        let input = hex::decode("0000002a")
            .expect("invalid selector hex");
        let result = Xvm::call(
            Context {
                source_vm_id: VmId::Evm,
                weight_limit: Weight::from_parts(10_000_000_000, 1024 * 1024),
            },
            VmId::Wasm,
            ALICE,
            wasm_callee_addr.clone().encode(),
            input,
            0,
            Some(0)
        );
        match result {
            Err(CallFailure {
                reason: FailureReason::Error(FailureError::VmError(data)),
                ..
            }) => {
                let error_string = "WASM call error: Module(ModuleError { index: 70, error: [22, 0, 0, 0], message: Some(\"StorageDepositLimitExhausted\") })";
                assert_eq!(data, error_string.as_bytes());
            },
            _ => panic!("unexpected wasm call result"),
        }
    });
}

#[test]
fn wasm_call_via_xvm_call_works_if_sufficient_storage_deposit_limit() {
    new_test_ext().execute_with(|| {
        let wasm_callee_addr = deploy_wasm_contract(WASM_SIMPLE_STORAGE_NAME);
        let input = hex::decode("0000002a").expect("invalid selector hex");
        assert_ok!(Xvm::call(
            Context {
                source_vm_id: VmId::Evm,
                weight_limit: Weight::from_parts(10_000_000_000, 1024 * 1024),
            },
            VmId::Wasm,
            ALICE,
            wasm_callee_addr.clone().encode(),
            input,
            0,
            Some(UNIT)
        ));
    });
}

/*

// SPDX-License-Identifier: GPL-3.0

pragma solidity >=0.8.2 <0.9.0;

interface XVM {
    function xvm_call(
        uint8 vm_id,
        bytes calldata to,
        bytes calldata input,
        uint256 value,
        uint256 storage_deposit_limit
    ) external payable returns (bool success, bytes memory data);
}

contract CallXVMPayble {
    function call_xvm_payable(bytes calldata to, bytes calldata input, uint256 value, uint256 storage_deposit_limit) external payable returns (bool success, bytes memory data)  {
        return XVM(0x0000000000000000000000000000000000005005).xvm_call(0x1F, to, input, value, storage_deposit_limit);
    }
}

 */
const CALL_XVM_PAYABLE_WITH_SDL: &'static str = "608060405234801561001057600080fd5b50610609806100206000396000f3fe60806040526004361061001e5760003560e01c80632d9338da14610023575b600080fd5b61003d600480360381019061003891906101a6565b610054565b60405161004b9291906102f8565b60405180910390f35b6000606061500573ffffffffffffffffffffffffffffffffffffffff1663acc57c7e601f8a8a8a8a8a8a6040518863ffffffff1660e01b81526004016100a097969594939291906103c5565b6000604051808303816000875af11580156100bf573d6000803e3d6000fd5b505050506040513d6000823e3d601f19601f820116820180604052508101906100e89190610577565b91509150965096945050505050565b6000604051905090565b600080fd5b600080fd5b600080fd5b600080fd5b600080fd5b60008083601f8401126101305761012f61010b565b5b8235905067ffffffffffffffff81111561014d5761014c610110565b5b60208301915083600182028301111561016957610168610115565b5b9250929050565b6000819050919050565b61018381610170565b811461018e57600080fd5b50565b6000813590506101a08161017a565b92915050565b600080600080600080608087890312156101c3576101c2610101565b5b600087013567ffffffffffffffff8111156101e1576101e0610106565b5b6101ed89828a0161011a565b9650965050602087013567ffffffffffffffff8111156102105761020f610106565b5b61021c89828a0161011a565b9450945050604061022f89828a01610191565b925050606061024089828a01610191565b9150509295509295509295565b60008115159050919050565b6102628161024d565b82525050565b600081519050919050565b600082825260208201905092915050565b60005b838110156102a2578082015181840152602081019050610287565b60008484015250505050565b6000601f19601f8301169050919050565b60006102ca82610268565b6102d48185610273565b93506102e4818560208601610284565b6102ed816102ae565b840191505092915050565b600060408201905061030d6000830185610259565b818103602083015261031f81846102bf565b90509392505050565b6000819050919050565b600060ff82169050919050565b6000819050919050565b600061036461035f61035a84610328565b61033f565b610332565b9050919050565b61037481610349565b82525050565b82818337600083830152505050565b60006103958385610273565b93506103a283858461037a565b6103ab836102ae565b840190509392505050565b6103bf81610170565b82525050565b600060a0820190506103da600083018a61036b565b81810360208301526103ed81888a610389565b90508181036040830152610402818688610389565b905061041160608301856103b6565b61041e60808301846103b6565b98975050505050505050565b6104338161024d565b811461043e57600080fd5b50565b6000815190506104508161042a565b92915050565b600080fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b610493826102ae565b810181811067ffffffffffffffff821117156104b2576104b161045b565b5b80604052505050565b60006104c56100f7565b90506104d1828261048a565b919050565b600067ffffffffffffffff8211156104f1576104f061045b565b5b6104fa826102ae565b9050602081019050919050565b600061051a610515846104d6565b6104bb565b90508281526020810184848401111561053657610535610456565b5b610541848285610284565b509392505050565b600082601f83011261055e5761055d61010b565b5b815161056e848260208601610507565b91505092915050565b6000806040838503121561058e5761058d610101565b5b600061059c85828601610441565b925050602083015167ffffffffffffffff8111156105bd576105bc610106565b5b6105c985828601610549565b915050925092905056fea2646970667358221220e44af7386feb3ae682c95df11f7b3de851b515869a073d6fabe05758e94e351f64736f6c63430008120033";

#[test]
fn calling_wasm_from_evm_works_if_sufficient_storage_deposit_limit() {
    new_test_ext().execute_with(|| {
        // create account mappings
        connect_accounts(&ALICE, &alith_secret_key());

        let wasm_callee_addr = deploy_wasm_contract(WASM_SIMPLE_STORAGE_NAME);
        let evm_caller_addr = deploy_evm_contract(CALL_XVM_PAYABLE_WITH_SDL);

        // Fund the EVM contract to pay for storage deposit.
        let _ = Balances::deposit_creating(&account_id_from(evm_caller_addr.clone()), UNIT);

        assert_ok!(EVM::call(
            RuntimeOrigin::root(),
            alith(),
            evm_caller_addr.clone(),
            // to: 0x0e0ddb5a5f0b99d7be468a3051a94073ec6b1900178316401a52b93415026999
            // input: 0x0000002a (store)
            // value: 0
            // storage_deposit_limit: 1_000_000_000_000_000
            hex::decode("2d9338da000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000038d7ea4c6800000000000000000000000000000000000000000000000000000000000000000200e0ddb5a5f0b99d7be468a3051a94073ec6b1900178316401a52b9341502699900000000000000000000000000000000000000000000000000000000000000040000002a00000000000000000000000000000000000000000000000000000000").expect("invalid call input hex"),
            U256::zero(),
            1_000_000,
            U256::from(DefaultBaseFeePerGas::get()),
            None,
            None,
            vec![],
        ));
        assert_eq!(
            System::events().iter().last().expect("no event found").event,
            RuntimeEvent::EVM(
                pallet_evm::Event::Executed { address: evm_caller_addr },
            ),
        );

        let result = Contracts::bare_call(
            account_id_from(evm_caller_addr),
            wasm_callee_addr,
            0,
            Weight::from_parts(10_000_000_000, 1024 * 1024),
            None,
            // `get` selector
            hex::decode("0000002b").expect("invalid selector hex"),
            DebugInfo::Skip,
            CollectEvents::Skip,
            Determinism::Enforced,
        );
        match result.result {
            Ok(ExecReturnValue { flags, data }) => {
                assert!(!flags.contains(ReturnFlags::REVERT));
                // Retrive stored value `42`.
                assert_eq!(data[1], 42);
            },
            _ => panic!("unexpected wasm call result"),
        }
    });
}
