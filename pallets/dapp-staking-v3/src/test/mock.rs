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

use crate::{self as pallet_dapp_staking, *};

use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU128, ConstU16, ConstU32, ConstU64},
    weights::Weight,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

use sp_io::TestExternalities;

pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

pub(crate) const EXISTENTIAL_DEPOSIT: Balance = 2;
pub(crate) const MINIMUM_LOCK_AMOUNT: Balance = 10;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub struct Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        DappStaking: pallet_dapp_staking,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1024));
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u64;
    type RuntimeCall = RuntimeCall;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
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
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type AccountStore = System;
    type WeightInfo = ();
}

impl pallet_dapp_staking::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type SmartContract = MockSmartContract;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxNumberOfContracts = ConstU16<10>;
    type MaxLockedChunks = ConstU32<5>;
    type MaxUnlockingChunks = ConstU32<5>;
    type MinimumLockedAmount = ConstU128<MINIMUM_LOCK_AMOUNT>;
    type UnlockingPeriod = ConstU64<20>;
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug, TypeInfo, MaxEncodedLen, Hash)]
pub enum MockSmartContract {
    Wasm(AccountId),
    Other(AccountId),
}

impl Default for MockSmartContract {
    fn default() -> Self {
        MockSmartContract::Wasm(1)
    }
}

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let balances = vec![1000; 9]
            .into_iter()
            .enumerate()
            .map(|(idx, amount)| (idx as u64 + 1, amount))
            .collect();

        pallet_balances::GenesisConfig::<Test> { balances: balances }
            .assimilate_storage(&mut storage)
            .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            System::set_block_number(1);
            DappStaking::on_initialize(System::block_number());

            // TODO: remove this after proper on_init handling is implemented
            pallet_dapp_staking::ActiveProtocolState::<Test>::put(ProtocolState {
                era: 1,
                next_era_start: BlockNumber::from(101_u32),
                period: 1,
                period_type: PeriodType::Voting(16),
                maintenance: false,
            });
        });

        ext
    }
}

/// Run to the specified block number.
/// Function assumes first block has been initialized.
pub(crate) fn _run_to_block(n: u64) {
    while System::block_number() < n {
        DappStaking::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        // This is performed outside of dapps staking but we expect it before on_initialize
        DappStaking::on_initialize(System::block_number());
    }
}

/// Run for the specified number of blocks.
/// Function assumes first block has been initialized.
pub(crate) fn run_for_blocks(n: u64) {
    _run_to_block(System::block_number() + n);
}

/// Advance blocks until the specified era has been reached.
///
/// Function has no effect if era is already passed.
pub(crate) fn advance_to_era(era: EraNumber) {
    // TODO: Properly implement this later when additional logic has been implemented
    ActiveProtocolState::<Test>::mutate(|state| state.era = era);
}

/// Advance blocks until the specified period has been reached.
///
/// Function has no effect if period is already passed.
pub(crate) fn advance_to_period(period: PeriodNumber) {
    // TODO: Properly implement this later when additional logic has been implemented
    ActiveProtocolState::<Test>::mutate(|state| state.period = period);
}
