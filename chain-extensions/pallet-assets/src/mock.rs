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

use crate::AssetsExtension;
use frame_support::traits::{AsEnsureOriginWithArg, ConstU128, Currency, Randomness};
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU64, Nothing},
    weights::Weight,
};
use frame_system::EnsureSigned;
use pallet_contracts::chain_extension::RegisteredChainExtension;
use pallet_contracts::{Config, Frame};
use sp_core::crypto::AccountId32;
use sp_runtime::{
    testing::H256,
    traits::{Convert, IdentityLookup, Zero},
    BuildStorage, Perbill,
};

pub type BlockNumber = u32;
pub type Balance = u128;
pub type AssetId = u128;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(
            Weight::from_parts(2_000_000_000_000, u64::MAX),
        );
}
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
    pub static UnstableInterface: bool = true;
    pub Schedule: pallet_contracts::Schedule<Test> = Default::default();
    pub static DepositPerByte: Balance = 1;
    pub const DepositPerItem: Balance = 1;
    pub const DefaultDepositLimit: Balance = 1;
    pub const MaxDelegateDependencies: u32 = 32;
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(1);
}

pub struct DummyDeprecatedRandomness;
impl Randomness<H256, BlockNumber> for DummyDeprecatedRandomness {
    fn random(_: &[u8]) -> (H256, BlockNumber) {
        (Default::default(), Zero::zero())
    }
}

#[derive_impl(pallet_contracts::config_preludes::TestDefaultConfig)]
impl pallet_contracts::Config for Test {
    type Time = Timestamp;
    type Randomness = DummyDeprecatedRandomness;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = Nothing;
    type CallStack = [Frame<Self>; 5];
    type ChainExtension = AssetsExtension<Self>;
    type Schedule = Schedule;
    type UnsafeUnstableInterface = UnstableInterface;
    type RuntimeHoldReason = RuntimeHoldReason;
    type UploadOrigin = EnsureSigned<AccountId32>;
    type InstantiateOrigin = EnsureSigned<AccountId32>;
}

impl RegisteredChainExtension<Test> for AssetsExtension<Test> {
    const ID: u16 = 02;
}

parameter_types! {
    pub static ExistentialDeposit: u64 = 1;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Test {
    type MinimumPeriod = ConstU64<1>;
}

#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
impl pallet_assets::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdParameter = u128;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId32>>;
    type ForceOrigin = frame_system::EnsureRoot<AccountId32>;
    type AssetDeposit = ConstU128<1>;
    type AssetAccountDeposit = ConstU128<10>;
    type MetadataDepositBase = ConstU128<1>;
    type MetadataDepositPerByte = ConstU128<1>;
    type ApprovalDeposit = ConstU128<1>;
    type Freezer = ();
}

type Block = frame_system::mocking::MockBlockU32<Test>;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Assets: pallet_assets,
        Timestamp: pallet_timestamp,
        Contracts: pallet_contracts,
    }
);

pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2u8; 32]);
pub const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000, 700_000);
pub const ONE: u128 = 1_000_000_000_000_000_000;

pub const ASSET_ID: u128 = 1;

impl Convert<Weight, BalanceOf<Self>> for Test {
    fn convert(w: Weight) -> BalanceOf<Self> {
        w.ref_time().into()
    }
}

pub struct ExtBuilder {
    existential_deposit: u64,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            existential_deposit: ExistentialDeposit::get(),
        }
    }
}

impl ExtBuilder {
    pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
        self.existential_deposit = existential_deposit;
        self
    }
    pub fn set_associated_consts(&self) {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
    }
    pub fn build(self) -> sp_io::TestExternalities {
        use env_logger::{Builder, Env};
        let env = Env::new().default_filter_or("runtime=debug");
        let _ = Builder::from_env(env).is_test(true).try_init();
        self.set_associated_consts();
        let mut t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> { balances: vec![] }
            .assimilate_storage(&mut t)
            .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
