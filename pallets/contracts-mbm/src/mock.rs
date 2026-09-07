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
    traits::{OnFinalize, OnInitialize},
    weights::Weight,
};
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        MultiBlockMigrations: pallet_migrations,
        ContractsMBM: crate,
    }
);

impl crate::Config for Runtime {}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = Block;
    type MultiBlockMigrator = MultiBlockMigrations;
}

#[cfg(not(feature = "runtime-benchmarks"))]
mod migrations {
    use super::{ContractsPalletName, Runtime};

    pub type Weights = crate::weights::SubstrateWeight<Runtime>;
    pub type Purge = crate::PurgeContractsChildTries<Runtime, ContractsPalletName, Weights>;
    pub type Remove = crate::RemovePalletStepped<ContractsPalletName, Weights>;
}
#[cfg(not(feature = "runtime-benchmarks"))]
pub use migrations::{Purge, Remove};

#[derive_impl(pallet_migrations::config_preludes::TestDefaultConfig)]
impl pallet_migrations::Config for Runtime {
    /// Mirrors the runtimes: the child tries must be purged before the prefix holding their
    /// `trie_id`s is wiped.
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations = (migrations::Purge, migrations::Remove);
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
    type MigrationStatusHandler = ();
    type MaxServiceWeight = MaxServiceWeight;
}

parameter_types! {
    /// Name of the pallet these migrations retire, matching the runtimes.
    pub const ContractsPalletName: &'static str = "Contracts";
    /// Mutable so a test can squeeze the budget and force a migration across several blocks.
    pub static MaxServiceWeight: Weight = Weight::from_parts(50_000_000_000, 1_000_000);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    // Makes the migrations' own logs visible with
    // `RUST_LOG=mbm::contracts=debug cargo test -p contracts-mbm -- --nocapture`.
    sp_tracing::try_init_simple();

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
