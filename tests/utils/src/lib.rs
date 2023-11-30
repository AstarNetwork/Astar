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

use frame_support::{traits::Currency, weights::Weight};
use pallet_contracts_primitives::{Code, ReturnFlags};
use parity_scale_codec::Decode;
use sp_runtime::traits::Hash;

type ContractBalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

/// Load a given wasm module from wasm binary contents along
/// with it's hash.
///
/// The fixture files are located under the `../ink-contracts/` directory.
pub fn load_wasm_module<T>(
    fixture_name: &str,
) -> std::io::Result<(Vec<u8>, <T::Hashing as Hash>::Output)>
where
    T: frame_system::Config,
{
    let fixture_path = ["../ink-contracts/", fixture_name, ".wasm"].concat();
    let wasm_binary = std::fs::read(fixture_path)?;
    let code_hash = T::Hashing::hash(&wasm_binary);
    Ok((wasm_binary, code_hash))
}

// Load and deploy the contract from wasm binary
/// and check for successful deploy
pub fn deploy_wasm_contract<T: pallet_contracts::Config>(
    contract_name: &str,
    origin: T::AccountId,
    value: ContractBalanceOf<T>,
    gas_limit: Weight,
    storage_deposit_limit: Option<ContractBalanceOf<T>>,
    data: Vec<u8>,
) -> (T::AccountId, <T::Hashing as Hash>::Output) {
    let (code, hash) = load_wasm_module::<T>(contract_name).unwrap();
    let outcome = pallet_contracts::Pallet::<T>::bare_instantiate(
        origin,
        value,
        gas_limit,
        storage_deposit_limit,
        Code::Upload(code),
        data,
        vec![],
        pallet_contracts::DebugInfo::Skip,
        pallet_contracts::CollectEvents::Skip,
    );

    // make sure it does not revert
    let result = outcome.result.unwrap();
    assert!(
        !result.result.did_revert(),
        "deploy_contract: reverted - {:?}",
        result
    );
    (result.account_id, hash)
}

/// Call the wasm contract method and returns the decoded return
/// values along with return flags and consumed weight
pub fn call_wasm_contract_method<T: pallet_contracts::Config, V: Decode>(
    origin: T::AccountId,
    dest: T::AccountId,
    value: ContractBalanceOf<T>,
    gas_limit: Weight,
    storage_deposit_limit: Option<ContractBalanceOf<T>>,
    data: Vec<u8>,
    debug: bool,
) -> (V, ReturnFlags, Weight) {
    let outcome = pallet_contracts::Pallet::<T>::bare_call(
        origin,
        dest,
        value,
        gas_limit,
        storage_deposit_limit,
        data,
        pallet_contracts::DebugInfo::Skip,
        pallet_contracts::CollectEvents::Skip,
        pallet_contracts::Determinism::Enforced,
    );

    if debug {
        println!(
            "Contract debug buffer - {:?}",
            String::from_utf8(outcome.debug_message.clone())
        );
        println!("Contract outcome - {outcome:?}");
    }

    let res = outcome.result.unwrap();
    // check for revert
    assert!(!res.did_revert(), "Contract reverted!");

    let value = Result::<V, ()>::decode(&mut res.data.as_ref())
        .expect("decoding failed")
        .expect("ink! lang error");

    (value, res.flags, outcome.gas_consumed)
}
