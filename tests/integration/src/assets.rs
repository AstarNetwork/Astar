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

use astar_primitives::evm::EVM_REVERT_CODE;
use pallet_evm_precompile_assets_erc20::AddressToAssetId;

#[test]
fn asset_create_and_destroy_work_for_evm_revert_code() {
    new_test_ext().execute_with(|| {
        let asset_id = 19;
        let precompile_address = Runtime::asset_id_to_address(asset_id);

        // Asset creation results in insertion of the revert opt code at the precompile address
        assert!(
            !pallet_evm::AccountCodes::<Runtime>::contains_key(&precompile_address),
            "Precompile address should be empty."
        );
        assert_ok!(Assets::create(
            RuntimeOrigin::signed(ALICE),
            asset_id.into(),
            ALICE.into(),
            1,
        ));
        assert_eq!(
            pallet_evm::AccountCodes::<Runtime>::get(&precompile_address),
            EVM_REVERT_CODE.to_vec(),
            "Precompile address should contain the revert code."
        );

        // Asset destroy results in removal of the revert opt code from the precompile address
        assert_ok!(Assets::start_destroy(
            RuntimeOrigin::signed(ALICE),
            asset_id.into(),
        ));
        assert_ok!(Assets::finish_destroy(
            RuntimeOrigin::signed(ALICE),
            asset_id.into(),
        ));
        assert!(
            !pallet_evm::AccountCodes::<Runtime>::contains_key(&precompile_address),
            "After asset is destroyed, precompile address should be empty."
        );
    });
}

#[test]
fn asset_create_fails_if_account_code_is_non_empty() {
    new_test_ext().execute_with(|| {
        let asset_id = 19;
        let precompile_address = Runtime::asset_id_to_address(asset_id);

        // Asset registration must fail if the precompile address is not empty
        pallet_evm::AccountCodes::<Runtime>::insert(&precompile_address, EVM_REVERT_CODE.to_vec());
        assert_noop!(
            Assets::create(
                RuntimeOrigin::signed(ALICE),
                asset_id.into(),
                ALICE.into(),
                1,
            ),
            pallet_assets::Error::<Runtime>::CallbackFailed
        );
    });
}
