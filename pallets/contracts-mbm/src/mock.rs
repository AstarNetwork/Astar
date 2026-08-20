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

#![cfg(test)]

use frame_support::{
    construct_runtime, derive_impl,
    migrations::MultiStepMigrator,
    parameter_types,
    traits::{ConstU64, OnFinalize, OnInitialize, VariantCount},
    weights::Weight,
};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        Balances: pallet_balances,
        MultiBlockMigrations: pallet_migrations,
        ContractsMBM: crate,
    }
);

impl crate::Config for Runtime {
    type BenchmarkHoldReason = StorageDeposit;
}

/// Stands in for the runtime's `RuntimeHoldReason`. Written by hand rather than composed by
/// `construct_runtime!` so the mock does not have to pull in `pallet-contracts` - the migration is
/// generic over the reason, and the runtimes' real wiring is covered in `tests/integration`.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
)]
pub enum HoldReason {
    /// Stands in for `pallet_contracts::HoldReason::CodeUploadDepositReserve`.
    CodeUploadDepositReserve,
    /// Stands in for `pallet_contracts::HoldReason::StorageDepositReserve`.
    StorageDepositReserve,
    /// A hold owned by some other pallet, which must be left alone.
    Unrelated,
}

impl VariantCount for HoldReason {
    const VARIANT_COUNT: u32 = 3;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type AccountData = pallet_balances::AccountData<u64>;
    type Block = Block;
    type MultiBlockMigrator = MultiBlockMigrations;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type AccountStore = System;
    type RuntimeHoldReason = HoldReason;
    type ExistentialDeposit = ConstU64<1>;
}

#[derive_impl(pallet_migrations::config_preludes::TestDefaultConfig)]
impl pallet_migrations::Config for Runtime {
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations = (
        crate::ReleaseContractsDeposits<
            Runtime,
            CodeDeposit,
            StorageDeposit,
            EscrowAccount,
            crate::weights::SubstrateWeight<Runtime>,
        >,
    );
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
    type MigrationStatusHandler = ();
    type MaxServiceWeight = MaxServiceWeight;
}

pub const ESCROW: u64 = 9_999;

parameter_types! {
    pub const CodeDeposit: HoldReason = HoldReason::CodeUploadDepositReserve;
    pub const StorageDeposit: HoldReason = HoldReason::StorageDepositReserve;
    pub const EscrowAccount: u64 = ESCROW;
    /// Mutable so a test can squeeze the budget and force the migration across several blocks.
    pub static MaxServiceWeight: Weight = Weight::from_parts(2_000_000_000, 1_000_000);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Advances to block `n`, servicing multi block migrations exactly as `Executive` does.
#[allow(dead_code)]
pub fn run_to_block(n: u64) {
    assert!(System::block_number() < n);
    while System::block_number() < n {
        let b = System::block_number();
        AllPalletsWithSystem::on_finalize(b);
        // Done by Executive:
        <Runtime as frame_system::Config>::MultiBlockMigrator::step();
        System::set_block_number(b + 1);
        AllPalletsWithSystem::on_initialize(b + 1);
    }
}

/// Whether `pallet-migrations` still has work queued.
#[allow(dead_code)]
pub fn migrations_in_progress() -> bool {
    pallet_migrations::Cursor::<Runtime>::get().is_some()
}
