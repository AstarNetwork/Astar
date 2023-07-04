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

//! Runtime integration tests setup & imports.

pub use frame_support::{
    assert_ok,
    traits::{OnFinalize, OnIdle, OnInitialize},
    weights::Weight,
};
pub use sp_core::H160;
pub use sp_runtime::{AccountId32, MultiAddress};

#[cfg(feature = "shibuya")]
pub use shibuya::*;
#[cfg(feature = "shibuya")]
mod shibuya {
    pub use shibuya_runtime::*;

    /// 1 SBY.
    pub const UNIT: Balance = SBY;
}

#[cfg(feature = "shiden")]
pub use shiden::*;
#[cfg(feature = "shiden")]
mod shiden {
    pub use shiden_runtime::*;

    /// 1 SDN.
    pub const UNIT: Balance = SDN;
}

#[cfg(feature = "astar")]
pub use astar::*;
#[cfg(feature = "astar")]
mod astar {
    pub use astar_runtime::*;

    /// 1 ASTR.
    pub const UNIT: Balance = ASTR;
}

pub const ALICE: AccountId32 = AccountId32::new([1_u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2_u8; 32]);
pub const CAT: AccountId32 = AccountId32::new([3_u8; 32]);

pub const INITIAL_AMOUNT: u128 = 100_000 * UNIT;

pub type SystemError = frame_system::Error<Runtime>;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_dapps_staking as DappStakingCall;
pub use pallet_proxy::Event as ProxyEvent;
pub use pallet_utility::{Call as UtilityCall, Event as UtilityEvent};

pub struct ExtBuilder {
    balances: Vec<(AccountId32, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self { balances: vec![] }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(AccountId32, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime> {
            balances: self.balances,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    ExtBuilder::default()
        .balances(vec![
            (ALICE, INITIAL_AMOUNT),
            (BOB, INITIAL_AMOUNT),
            (CAT, INITIAL_AMOUNT),
        ])
        .build()
}

// Block time: 12 seconds.
pub const BLOCK_TIME: u64 = 12_000;

pub fn run_to_block(n: u32) {
    while System::block_number() < n {
        let block_number = System::block_number();
        Timestamp::set_timestamp(block_number as u64 * BLOCK_TIME);
        DappsStaking::on_finalize(block_number);
        Authorship::on_finalize(block_number);
        Session::on_finalize(block_number);
        AuraExt::on_finalize(block_number);
        PolkadotXcm::on_finalize(block_number);
        Ethereum::on_finalize(block_number);
        BaseFee::on_finalize(block_number);

        System::set_block_number(block_number + 1);

        TransactionPayment::on_initialize(block_number);
        DappsStaking::on_initialize(block_number);
        Authorship::on_initialize(block_number);
        Aura::on_initialize(block_number);
        AuraExt::on_initialize(block_number);
        Ethereum::on_initialize(block_number);
        BaseFee::on_initialize(block_number);
        #[cfg(any(feature = "shibuya", feature = "shiden", features = "astar"))]
        RandomnessCollectiveFlip::on_initialize(block_number);
        StateTrieMigration::on_initialize(block_number);

        XcmpQueue::on_idle(block_number, Weight::MAX);
        DmpQueue::on_idle(block_number, Weight::MAX);
        Contracts::on_idle(block_number, Weight::MAX);
    }
}

fn last_events(n: usize) -> Vec<RuntimeEvent> {
    frame_system::Pallet::<Runtime>::events()
        .into_iter()
        .rev()
        .take(n)
        .rev()
        .map(|e| e.event)
        .collect()
}

pub fn expect_events(e: Vec<RuntimeEvent>) {
    assert_eq!(last_events(e.len()), e);
}
