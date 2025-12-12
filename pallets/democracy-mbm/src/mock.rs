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

use frame_support::traits::{
    ConstBool, ConstU64, EqualPrivilegeOnly, LockIdentifier, StorageVersion,
};
use frame_support::{
    construct_runtime, derive_impl,
    migrations::MultiStepMigrator,
    pallet_prelude::*,
    parameter_types,
    traits::{OnFinalize, OnInitialize},
};
use frame_system::{limits::BlockWeights, EnsureRoot, EnsureSigned};
use sp_runtime::{BuildStorage, Perbill};

type Block = frame_system::mocking::MockBlock<Runtime>;

pub const DEMOCRACY_ID: LockIdentifier = *b"democrac";

construct_runtime!(
    pub struct Runtime {
        System: frame_system,
        Balances: pallet_balances,
        Democracy: pallet_democracy,
        MultiBlockMigrations: pallet_migrations,
        Scheduler: pallet_scheduler,
        Preimage: pallet_preimage,
        Pallet: crate,
    }
);

impl crate::Config for Runtime {}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type AccountData = pallet_balances::AccountData<u64>;
    type Block = Block;
    type MultiBlockMigrator = MultiBlockMigrations;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Runtime {
    type AccountStore = System;
}

parameter_types! {
    pub const MaxServiceWeight: Weight = Weight::from_parts(1_000_000_000, 1_000_000);
}

#[derive_impl(pallet_migrations::config_preludes::TestDefaultConfig)]
impl pallet_migrations::Config for Runtime {
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations =
        (crate::DemocracyMigrationV1ToV2<Runtime, crate::weights::SubstrateWeight<Runtime>>,);
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
    type MaxServiceWeight = MaxServiceWeight;
}

impl pallet_preimage::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<u64>;
    type Consideration = ();
}

parameter_types! {
    pub MaximumWeight: Weight = Perbill::from_percent(80) * <<Runtime as frame_system::Config>::BlockWeights as Get<BlockWeights>>::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumWeight;
    type ScheduleOrigin = EnsureRoot<u64>;
    type MaxScheduledPerBlock = ConstU32<100>;
    type WeightInfo = ();
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = ();
    type BlockNumberProvider = System;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = ConstU64<2>;
    type LaunchPeriod = ConstU64<2>;
    type VotingPeriod = ConstU64<20>;
    type VoteLockingPeriod = ConstU64<20>;
    type MinimumDeposit = ConstU64<1>;
    type FastTrackVotingPeriod = ConstU64<20>;
    type CooloffPeriod = ConstU64<20>;
    type MaxVotes = ConstU32<128>;
    type MaxProposals = ConstU32<128>;
    type MaxDeposits = ConstU32<128>;
    type MaxBlacklisted = ConstU32<128>;
    type ExternalOrigin = EnsureSigned<Self::AccountId>;
    type ExternalMajorityOrigin = EnsureSigned<Self::AccountId>;
    type ExternalDefaultOrigin = EnsureSigned<Self::AccountId>;
    type FastTrackOrigin = EnsureSigned<Self::AccountId>;
    type InstantOrigin = EnsureSigned<Self::AccountId>;
    type InstantAllowed = ConstBool<true>;
    type CancellationOrigin = EnsureSigned<Self::AccountId>;
    type BlacklistOrigin = EnsureSigned<Self::AccountId>;
    type CancelProposalOrigin = EnsureSigned<Self::AccountId>;
    type VetoOrigin = EnsureSigned<Self::AccountId>;
    type SubmitOrigin = EnsureSigned<Self::AccountId>;
    type PalletsOrigin = OriginCaller;
    type Preimages = Preimage;
    type Scheduler = Scheduler;
    type Slash = ();
    type WeightInfo = ();
}

#[derive(Default)]
pub struct ExtBuilder;

pub(crate) const ALICE: u64 = 1;
pub(crate) const BOB: u64 = 2;
pub(crate) const CHARLIE: u64 = 3;
pub(crate) const DAVE: u64 = 4;
pub(crate) const YUKI: u64 = 5;

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap();
        pallet_balances::GenesisConfig::<Runtime> {
            balances: vec![
                (ALICE, 10000),
                (BOB, 10000),
                (CHARLIE, 10000),
                (DAVE, 10000),
                (YUKI, 10000),
            ],
            ..Default::default()
        }
        .assimilate_storage(&mut t)
        .unwrap();
        pallet_democracy::GenesisConfig::<Runtime>::default()
            .assimilate_storage(&mut t)
            .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            System::set_block_number(1);
            // Set storage version to 1 for Democracy pallet
            let storage_version = StorageVersion::new(1);
            storage_version.put::<pallet_democracy::Pallet<Runtime>>();
        });
        ext
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    ExtBuilder.build()
}

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
