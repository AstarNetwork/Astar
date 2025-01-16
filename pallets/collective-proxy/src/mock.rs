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

use crate::{self as pallet_collective_proxy};

use astar_primitives::{Balance, BlockNumber};
use frame_support::{
    construct_runtime, derive_impl, ord_parameter_types, parameter_types,
    traits::{ConstU128, ConstU32, InstanceFilter},
    weights::Weight,
};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

use frame_system::EnsureSignedBy;

type Block = frame_system::mocking::MockBlockU32<Test>;
type AccountId = u64;

pub(crate) const COMMUNITY_ACCOUNT: AccountId = 1337;
pub(crate) const PRIVILEGED_ACCOUNT: AccountId = 365;

construct_runtime!(
    pub struct Test {
        System: frame_system,
        Balances: pallet_balances,
        CollectiveProxy: pallet_collective_proxy,
    }
);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type RuntimeCall = RuntimeCall;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type RuntimeTask = RuntimeTask;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type RuntimeHoldReason = RuntimeHoldReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<1>;
    type WeightInfo = ();
}

parameter_types! {
    pub const ProxyAccountId: AccountId = COMMUNITY_ACCOUNT;
}
ord_parameter_types! {
    pub const CollectiveProxyManager: AccountId = PRIVILEGED_ACCOUNT;
}

#[derive(Default)]
pub struct MockCallFilter;
impl InstanceFilter<RuntimeCall> for MockCallFilter {
    fn filter(&self, c: &RuntimeCall) -> bool {
        matches!(
            c,
            RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death { .. })
                | RuntimeCall::System(frame_system::Call::remark { .. })
        )
    }
}

impl pallet_collective_proxy::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CollectiveProxy = EnsureSignedBy<CollectiveProxyManager, AccountId>;
    type ProxyAccountId = ProxyAccountId;
    type CallFilter = MockCallFilter;
    type WeightInfo = ();
}

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();

        let mut balances: Vec<_> = vec![1000; 9]
            .into_iter()
            .enumerate()
            .map(|(idx, amount)| (idx as AccountId + 1, amount as Balance))
            .collect();
        balances.push((COMMUNITY_ACCOUNT, 1000));

        pallet_balances::GenesisConfig::<Test> { balances: balances }
            .assimilate_storage(&mut storage)
            .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            System::set_block_number(1);
        });

        ext
    }
}
