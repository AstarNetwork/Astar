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

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok, weights::Weight};
use parity_scale_codec::Encode;
use sp_core::H160;
use sp_runtime::MultiAddress;

#[test]
fn calling_into_same_vm_is_not_allowed() {
    ExtBuilder::default().build().execute_with(|| {
        // Calling EVM from EVM
        let evm_context = Context {
            source_vm_id: VmId::Evm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let evm_vm_id = VmId::Evm;
        let evm_target = H160::repeat_byte(1).encode();
        let input = vec![1, 2, 3];
        let value = 1_000_000u128;
        let evm_used_weight: Weight = weights::SubstrateWeight::<TestRuntime>::evm_call_overheads();
        assert_noop!(
            Xvm::call(
                evm_context,
                evm_vm_id,
                ALICE,
                evm_target,
                input.clone(),
                value,
                None
            ),
            CallFailure::error(SameVmCallDenied, evm_used_weight,),
        );

        // Calling WASM from WASM
        let wasm_context = Context {
            source_vm_id: VmId::Wasm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let wasm_vm_id = VmId::Wasm;
        let wasm_target = ALICE.encode();
        let wasm_used_weight: Weight =
            weights::SubstrateWeight::<TestRuntime>::wasm_call_overheads();
        assert_noop!(
            Xvm::call(
                wasm_context,
                wasm_vm_id,
                ALICE,
                wasm_target,
                input,
                value,
                None
            ),
            CallFailure::error(SameVmCallDenied, wasm_used_weight,),
        );
    });
}

#[test]
fn evm_call_fails_if_target_not_h160() {
    ExtBuilder::default().build().execute_with(|| {
        let context = Context {
            source_vm_id: VmId::Wasm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let vm_id = VmId::Evm;
        let input = vec![1; 65_536];
        let value = 1_000_000u128;
        let used_weight: Weight = weights::SubstrateWeight::<TestRuntime>::evm_call_overheads();

        assert_noop!(
            Xvm::call(
                context.clone(),
                vm_id,
                ALICE,
                ALICE.encode(),
                input.clone(),
                value,
                None
            ),
            CallFailure::revert(InvalidTarget, used_weight,),
        );

        assert_noop!(
            Xvm::call(context, vm_id, ALICE, vec![1, 2, 3], input, value, None),
            CallFailure::revert(InvalidTarget, used_weight,),
        );
    });
}

#[test]
fn evm_call_fails_if_input_too_large() {
    ExtBuilder::default().build().execute_with(|| {
        let context = Context {
            source_vm_id: VmId::Wasm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let vm_id = VmId::Evm;
        let target = H160::repeat_byte(0xFF);
        let value = 1_000_000u128;
        let used_weight: Weight = weights::SubstrateWeight::<TestRuntime>::evm_call_overheads();

        assert_noop!(
            Xvm::call(
                context,
                vm_id,
                ALICE,
                target.encode(),
                vec![1; 65_537],
                value,
                None
            ),
            CallFailure::revert(InputTooLarge, used_weight,),
        );
    });
}

#[test]
fn evm_call_works() {
    ExtBuilder::default().build().execute_with(|| {
        let context = Context {
            source_vm_id: VmId::Wasm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let vm_id = VmId::Evm;
        let target = H160::repeat_byte(0xFF);
        let input = vec![1; 65_536];
        let value = 1_000_000u128;

        assert_ok!(Xvm::call(
            context,
            vm_id,
            ALICE,
            target.encode(),
            input.clone(),
            value,
            None
        ));
        let source = Decode::decode(
            &mut hex::decode("f0bd9ffde7f9f4394d8cc1d86bf24d87e5d5a9a9")
                .expect("invalid source hex")
                .as_ref(),
        )
        .expect("invalid source");
        MockEthereumTransact::assert_transacted(
            source,
            CheckedEthereumTx {
                gas_limit: U256::from(246000),
                target: H160::repeat_byte(0xFF),
                value: U256::from(value),
                input: EthereumTxInput::try_from(input).expect("input too large"),
                maybe_access_list: None,
            },
        );
    });
}

#[test]
fn wasm_call_fails_if_invalid_target() {
    ExtBuilder::default().build().execute_with(|| {
        let context = Context {
            source_vm_id: VmId::Evm,
            weight_limit: Weight::from_parts(1_000_000, 1_000_000),
        };
        let vm_id = VmId::Wasm;
        let target = vec![1, 2, 3];
        let input = vec![1, 2, 3];
        let value = 1_000_000u128;
        let used_weight: Weight = weights::SubstrateWeight::<TestRuntime>::wasm_call_overheads();

        assert_noop!(
            Xvm::call(context, vm_id, ALICE, target.encode(), input, value, None),
            CallFailure::revert(InvalidTarget, used_weight,),
        );
    });
}
