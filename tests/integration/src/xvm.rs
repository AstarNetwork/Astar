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

use astar_primitives::xvm::{Context, VmId, XvmCall};
use frame_support::{traits::Currency, weights::Weight};
use parity_scale_codec::Encode;
use sp_runtime::MultiAddress;

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

/* Call WASM payable:

// SPDX-License-Identifier: GPL-3.0

pragma solidity >=0.8.2 <0.9.0;

interface XVM {
    function xvm_call(
        uint8 vm_id,
        bytes calldata to,
        bytes calldata input,
        uint256 value
    ) external payable returns (bool success, bytes memory data);
}

contract CallXVMPayble {
    function call_xvm_payable(bytes calldata to, bytes calldata input, uint256 value) external payable returns (bool success, bytes memory data)  {
        return XVM(0x0000000000000000000000000000000000005005).xvm_call(0x1F, to, input, value);
    }
}

 */
const CALL_WASM_PAYBLE: &str = "608060405234801561001057600080fd5b506105e6806100206000396000f3fe60806040526004361061001e5760003560e01c80634012b91414610023575b600080fd5b61003d600480360381019061003891906101a3565b610054565b60405161004b9291906102e3565b60405180910390f35b6000606061500573ffffffffffffffffffffffffffffffffffffffff1663e5d9bac0601f89898989896040518763ffffffff1660e01b815260040161009e969594939291906103b0565b6000604051808303816000875af11580156100bd573d6000803e3d6000fd5b505050506040513d6000823e3d601f19601f820116820180604052508101906100e69190610554565b915091509550959350505050565b6000604051905090565b600080fd5b600080fd5b600080fd5b600080fd5b600080fd5b60008083601f84011261012d5761012c610108565b5b8235905067ffffffffffffffff81111561014a5761014961010d565b5b60208301915083600182028301111561016657610165610112565b5b9250929050565b6000819050919050565b6101808161016d565b811461018b57600080fd5b50565b60008135905061019d81610177565b92915050565b6000806000806000606086880312156101bf576101be6100fe565b5b600086013567ffffffffffffffff8111156101dd576101dc610103565b5b6101e988828901610117565b9550955050602086013567ffffffffffffffff81111561020c5761020b610103565b5b61021888828901610117565b9350935050604061022b8882890161018e565b9150509295509295909350565b60008115159050919050565b61024d81610238565b82525050565b600081519050919050565b600082825260208201905092915050565b60005b8381101561028d578082015181840152602081019050610272565b60008484015250505050565b6000601f19601f8301169050919050565b60006102b582610253565b6102bf818561025e565b93506102cf81856020860161026f565b6102d881610299565b840191505092915050565b60006040820190506102f86000830185610244565b818103602083015261030a81846102aa565b90509392505050565b6000819050919050565b600060ff82169050919050565b6000819050919050565b600061034f61034a61034584610313565b61032a565b61031d565b9050919050565b61035f81610334565b82525050565b82818337600083830152505050565b6000610380838561025e565b935061038d838584610365565b61039683610299565b840190509392505050565b6103aa8161016d565b82525050565b60006080820190506103c56000830189610356565b81810360208301526103d8818789610374565b905081810360408301526103ed818587610374565b90506103fc60608301846103a1565b979650505050505050565b61041081610238565b811461041b57600080fd5b50565b60008151905061042d81610407565b92915050565b600080fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b61047082610299565b810181811067ffffffffffffffff8211171561048f5761048e610438565b5b80604052505050565b60006104a26100f4565b90506104ae8282610467565b919050565b600067ffffffffffffffff8211156104ce576104cd610438565b5b6104d782610299565b9050602081019050919050565b60006104f76104f2846104b3565b610498565b90508281526020810184848401111561051357610512610433565b5b61051e84828561026f565b509392505050565b600082601f83011261053b5761053a610108565b5b815161054b8482602086016104e4565b91505092915050565b6000806040838503121561056b5761056a6100fe565b5b60006105798582860161041e565b925050602083015167ffffffffffffffff81111561059a57610599610103565b5b6105a685828601610526565b915050925092905056fea264697066735822122047908cecfa9ace275a4ba96e787bb0d1541ec599c370983693c6cd9c1f5b7dbe64736f6c63430008120033";

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

#[test]
fn evm_payable_call_via_xvm_works() {
    new_test_ext().execute_with(|| {
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
        let contract_addr = deploy_wasm_contract("payable");

        let prev_balance = Balances::free_balance(&contract_addr);
        let value = UNIT;
        assert_ok!(Xvm::call(
            Context {
                source_vm_id: VmId::Evm,
                weight_limit: Weight::from_parts(10_000_000_000, 1024 * 1024),
            },
            VmId::Wasm,
            ALICE,
            MultiAddress::<AccountId32, ()>::Id(contract_addr.clone()).encode(),
            // Calling `deposit`
            hex::decode("0000002a").expect("invalid selector hex"),
            value
        ));
        assert_eq!(
            Balances::free_balance(contract_addr.clone()),
            value + prev_balance
        );
    });
}

#[test]
fn calling_wasm_payable_from_evm_fails_if_caller_contract_balance_below_ed() {
    new_test_ext().execute_with(|| {
        let wasm_payable_addr = deploy_wasm_contract("payable");
        let call_wasm_payable_addr = deploy_evm_contract(CALL_WASM_PAYBLE);

        let value = 1_000_000_000;
        assert_ok!(EVM::call(
            RuntimeOrigin::root(),
            alith(),
            call_wasm_payable_addr.clone(),
            // to: 0x00a8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c
            // input: 0x0000002a (deposit)
            // value: 1000000000
            hex::decode("4012b914000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000003b9aca00000000000000000000000000000000000000000000000000000000000000002100a8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000002a00000000000000000000000000000000000000000000000000000000").expect("invalid call input hex"),
            U256::from(value),
            1_000_000,
            U256::from(DefaultBaseFeePerGas::get()),
            None,
            None,
            vec![],
        ));

        // TODO: after XVM error propagation finished, assert `pallet-evm` execution error
        // and update balance assertions.

        // Transfer to EVM contract ok.
        assert_eq!(
            Balances::free_balance(&account_id_from(call_wasm_payable_addr)),
            value,
        );
        // Transfer from EVM contract to wasm Contract err.
        assert_eq!(
            Balances::free_balance(&wasm_payable_addr),
            ExistentialDeposit::get(),
        );
    });
}

#[test]
fn calling_wasm_payable_from_evm_works() {
    new_test_ext().execute_with(|| {
        let wasm_payable_addr = deploy_wasm_contract("payable");
        let call_wasm_payable_addr = deploy_evm_contract(CALL_WASM_PAYBLE);

        let _ = Balances::deposit_creating(&account_id_from(call_wasm_payable_addr.clone()), ExistentialDeposit::get());

        let prev_wasm_payable_balance = Balances::free_balance(&wasm_payable_addr);
        let value = 1_000_000_000;
        assert_ok!(EVM::call(
            RuntimeOrigin::root(),
            alith(),
            call_wasm_payable_addr.clone(),
            // to: 0x00a8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c
            // input: 0x0000002a (deposit)
            // value: 1000000000
            hex::decode("4012b914000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000003b9aca00000000000000000000000000000000000000000000000000000000000000002100a8f69d59df362b69a8d4acdb9001eb3e1b8d067b8fdaa70081aed945bde5c48c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000002a00000000000000000000000000000000000000000000000000000000").expect("invalid call input hex"),
            U256::from(value),
            1_000_000,
            U256::from(DefaultBaseFeePerGas::get()),
            None,
            None,
            vec![],
        ));
        let recieved = Balances::free_balance(&wasm_payable_addr) - prev_wasm_payable_balance;
        assert_eq!(recieved, value);
    });
}

#[test]
fn calling_evm_payable_from_wasm_works() {
    new_test_ext().execute_with(|| {
        let evm_payable_addr = deploy_evm_contract(EVM_PAYABLE);
        let wasm_address = deploy_wasm_contract("call_xvm_payable");

        let value = UNIT;

        // TODO: after Account Unification finished, remove this mock account.
        // It is needed now because currently the `AccountMapping` and `AddressMapping` are
        // both one way mapping.
        let mock_unified_wasm_account = account_id_from(h160_from(wasm_address.clone()));
        let _ = Balances::deposit_creating(&mock_unified_wasm_account, value);

        let evm_payable = evm_payable_addr.as_ref().to_vec();
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
            MultiAddress::Id(wasm_address.clone()),
            value,
            Weight::from_parts(10_000_000_000, 10 * 1024 * 1024),
            None,
            input,
        ));

        assert_eq!(
            Balances::free_balance(account_id_from(evm_payable_addr)),
            value
        );

        // TODO: after Account Unification finished, enable the wasm address balance check
        // and remove the mock account balance check.
        // assert_eq!(Balances::free_balance(&wasm_address), ExistentialDeposit::get());
        assert_eq!(Balances::free_balance(&mock_unified_wasm_account), 0);
    });
}
