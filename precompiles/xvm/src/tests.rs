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

use precompile_utils::testing::*;
use precompile_utils::EvmDataWriter;
use sp_core::U256;

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
                    .write(U256::one())
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
                    .write(
                        hex::decode("0000000000000000000000000000000000000000")
                            .expect("invalid hex"),
                    )
                    .write(U256::one())
                    .build(),
            )
            .expect_no_logs()
            .execute_some();
    })
}

#[test]
fn weight_limit_is_min_of_remaining_and_user_limit() {
    ExtBuilder::default().build().execute_with(|| {
        // The caller didn't set a limit.
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(0x1Fu8)
                    .write(Bytes(
                        hex::decode("0000000000000000000000000000000000000000")
                            .expect("invalid hex"),
                    ))
                    .write(Bytes(b"".to_vec()))
                    .write(U256::one())
                    .build(),
            )
            .expect_no_logs()
            .execute_some();
        assert_eq!(
            WeightLimitCalledWith::get(),
            <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(u64::MAX, true)
        );

        // The caller set a limit.
        let gas_limit = 1_000;
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(0x1Fu8)
                    .write(Bytes(
                        hex::decode("0000000000000000000000000000000000000000")
                            .expect("invalid hex"),
                    ))
                    .write(Bytes(b"".to_vec()))
                    .write(U256::one())
                    .build(),
            )
            .with_gas_limit(gas_limit)
            .expect_no_logs()
            .execute_some();
        assert_eq!(
            WeightLimitCalledWith::get(),
            <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(gas_limit, true)
        );
    });
}
