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

use crate::mock::*;
use crate::*;

use astar_primitives::xvm::CallError;
use parity_scale_codec::Encode;
use precompile_utils::testing::*;
use precompile_utils::EvmDataWriter;

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

#[test]
fn wrong_argument_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(42u64)
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"input doesn't match expected length");

        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(0u8)
                    .write(Bytes(b"".to_vec()))
                    .write(Bytes(b"".to_vec()))
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"invalid vm id");
    })
}

#[test]
fn correct_arguments_works() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(0x1Fu8)
                    .write(Bytes(b"".to_vec()))
                    .write(Bytes(b"".to_vec()))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(
                EvmDataWriter::new()
                    .write(false) // the XVM call should succeed but the internal should fail
                    .write(Bytes(CallError::InvalidTarget.encode()))
                    .build(),
            );
    })
}
