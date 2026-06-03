// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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
use sp_runtime::DispatchError;

/// New WASM code uploads are frozen.
#[test]
fn upload_code_is_disabled() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Contracts::upload_code(
                RuntimeOrigin::signed(ALICE),
                vec![],
                None,
                pallet_contracts::Determinism::Enforced,
            ),
            DispatchError::BadOrigin,
        );
    });
}

/// Deploying a new contract with inline code is frozen.
#[test]
fn instantiate_with_code_is_disabled() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Contracts::instantiate_with_code(
                RuntimeOrigin::signed(ALICE),
                0u128,
                Weight::zero(),
                None,
                vec![],
                vec![],
                vec![],
            ),
            DispatchError::BadOrigin,
        );
    });
}

/// Instantiating from an existing code hash is frozen.
#[test]
fn instantiate_is_disabled() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Contracts::instantiate(
                RuntimeOrigin::signed(ALICE),
                0u128,
                Weight::zero(),
                None,
                Default::default(),
                vec![],
                vec![],
            ),
            DispatchError::BadOrigin,
        );
    });
}
