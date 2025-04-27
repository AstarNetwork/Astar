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

use super::*;
use crate as pallet_unified_accounts;
use astar_primitives::evm::HashedDefaultMappings;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU64, FindAuthor},
    weights::Weight,
};
use pallet_ethereum::PostLogContent;
use pallet_evm::FeeCalculator;
use sp_core::{keccak_256, H160, U256};
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256},
    AccountId32, BuildStorage, ConsensusEngineId,
};

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
    pub const ExistentialDeposit: u128 = 100;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for TestRuntime {
    type Block = Block;
    type AccountId = AccountId;
    type Lookup = (AccountIdLookup<Self::AccountId, ()>, UnifiedAccounts);
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnKilledAccount = KillAccountMapping<Self>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for TestRuntime {
    type Balance = Balance;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for TestRuntime {
    type MinimumPeriod = ConstU64<3>;
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
    type AccountProvider = pallet_evm::FrameSystemAccountProvider<Self>;
    type GasLimitStorageGrowthRatio = ConstU64<0>;}

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<<TestRuntime as frame_system::Config>::Version>;
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

type Block = frame_system::mocking::MockBlock<TestRuntime>;

construct_runtime!(
    pub struct TestRuntime {
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
        let mut t = frame_system::GenesisConfig::<TestRuntime>::default()
            .build_storage()
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
