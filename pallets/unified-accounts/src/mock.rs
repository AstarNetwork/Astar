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

#![cfg(test)]

use super::*;
use crate as pallet_unified_accounts;
use astar_primitives::evm::HashedDefaultMappings;
use frame_support::{
    construct_runtime, parameter_types,
    sp_io::TestExternalities,
    traits::{ConstU64, FindAuthor},
    weights::Weight,
};
use pallet_ethereum::PostLogContent;
use pallet_evm::FeeCalculator;
use sp_core::{keccak_256, H160, H256, U256};
use sp_runtime::{
    testing::Header,
    traits::{AccountIdLookup, BlakeTwo256},
    AccountId32, ConsensusEngineId,
};

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
    pub const ExistentialDeposit: u128 = 100;
}

impl frame_system::Config for TestRuntime {
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
    type Lookup = (AccountIdLookup<Self::AccountId, ()>, UnifiedAccounts);
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = KillAccountMapping<Self>;
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for TestRuntime {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
}

impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<3>;
    type WeightInfo = ();
}

pub struct MockFeeCalculator;
impl FeeCalculator for MockFeeCalculator {
    fn min_gas_price() -> (U256, Weight) {
        (U256::one(), Weight::zero())
    }
}

pub struct MockFindAuthor;
impl FindAuthor<H160> for MockFindAuthor {
    fn find_author<'a, I>(_digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        Some(H160::from_low_u64_be(1))
    }
}

parameter_types! {
    pub WeightPerGas: Weight = Weight::from_parts(1, 0);
    pub const BlockGasLimit: U256 = U256::MAX;
    pub ChainId: u64 = 1024;
}

impl pallet_evm::Config for TestRuntime {
    type FeeCalculator = MockFeeCalculator;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<TestRuntime>;
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = UnifiedAccounts;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = ();
    type PrecompilesValue = ();
    type ChainId = ChainId;
    type OnChargeTransaction = ();
    type BlockGasLimit = BlockGasLimit;
    type OnCreate = ();
    type FindAuthor = MockFindAuthor;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<TestRuntime>;
    type GasLimitPovSizeRatio = ConstU64<4>;
}

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

parameter_types! {
    pub const AccountMappingStorageFee: u128 = 100_000_000;
}

impl pallet_unified_accounts::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type DefaultMappings = HashedDefaultMappings<BlakeTwo256>;
    type ChainId = ChainId;
    type AccountMappingStorageFee = AccountMappingStorageFee;
    type WeightInfo = ();
}

pub(crate) type AccountId = AccountId32;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([2u8; 32]);

pub fn alice_secret() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}

pub fn bob_secret() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Bob")).unwrap()
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

construct_runtime!(
    pub struct TestRuntime
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Evm: pallet_evm,
        Ethereum: pallet_ethereum,
        UnifiedAccounts: pallet_unified_accounts,
    }
);

pub struct ExtBuilder {
    balances: Vec<(AccountId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![
                (ALICE, 1_000_000_000_000),
                (BOB, 1_000_000_000_000),
                (CHARLIE, 1_000_000_000_000),
            ],
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: self.balances,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = TestExternalities::from(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
