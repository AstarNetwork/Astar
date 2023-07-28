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
use crate as pallet_xvm;

use fp_evm::{CallInfo as EvmCallInfo, ExitReason, ExitSucceed, UsedGas};
use frame_support::{
    construct_runtime,
    dispatch::{DispatchErrorWithPostInfo, PostDispatchInfo},
    pallet_prelude::*,
    parameter_types,
    sp_io::TestExternalities,
    traits::{ConstBool, ConstU128, ConstU64, Nothing},
};
use sp_core::{H160, H256};
use sp_runtime::{
    testing::Header,
    traits::{AccountIdLookup, BlakeTwo256},
    AccountId32,
};

parameter_types! {
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
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
    type Lookup = AccountIdLookup<Self::AccountId, ()>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
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

impl pallet_balances::Config for TestRuntime {
    type MaxLocks = ConstU32<4>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<2>;
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

impl pallet_insecure_randomness_collective_flip::Config for TestRuntime {}

parameter_types! {
    pub const DepositPerItem: Balance = 1_000;
    pub const DepositPerByte: Balance = 1_000;
    pub const DefaultDepositLimit: Balance = 1_000;
    pub Schedule: pallet_contracts::Schedule<TestRuntime> = Default::default();
}

impl pallet_contracts::Config for TestRuntime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = Nothing;
    type DepositPerItem = DepositPerItem;
    type DepositPerByte = DepositPerByte;
    type DefaultDepositLimit = DefaultDepositLimit;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    type WeightPrice = ();
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = ConstBool<true>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
}

pub struct HashedAccountMapping;
impl astar_primitives::ethereum_checked::AccountMapping<AccountId> for HashedAccountMapping {
    fn into_h160(account_id: AccountId) -> H160 {
        let data = (b"evm:", account_id);
        return H160::from_slice(&data.using_encoded(sp_io::hashing::blake2_256)[0..20]);
    }
}

pub struct MockEthereumTransact;
impl CheckedEthereumTransact for MockEthereumTransact {
    fn xvm_transact(
        _source: H160,
        _checked_tx: CheckedEthereumTx,
    ) -> Result<(PostDispatchInfo, EvmCallInfo), DispatchErrorWithPostInfo> {
        Ok((
            PostDispatchInfo {
                actual_weight: Default::default(),
                pays_fee: Default::default(),
            },
            EvmCallInfo {
                exit_reason: ExitReason::Succeed(ExitSucceed::Returned),
                value: Default::default(),
                used_gas: UsedGas {
                    standard: Default::default(),
                    effective: Default::default(),
                },
                logs: Default::default(),
                weight_info: None,
            },
        ))
    }
}

pub struct MockGasWeightMapping;
impl GasWeightMapping for MockGasWeightMapping {
    fn gas_to_weight(gas: u64, _without_base_weight: bool) -> Weight {
        Weight::from_parts(gas, 0)
    }
    fn weight_to_gas(weight: Weight) -> u64 {
        weight.ref_time()
    }
}

impl pallet_xvm::Config for TestRuntime {
    type GasWeightMapping = MockGasWeightMapping;
    type AccountMapping = HashedAccountMapping;
    type EthereumTransact = MockEthereumTransact;
    type WeightInfo = ();
}

pub(crate) type AccountId = AccountId32;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

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
        RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
        Contracts: pallet_contracts,
        Xvm: pallet_xvm,
    }
);

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
    #[allow(dead_code)]
    pub fn build(self) -> TestExternalities {
        let t = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        let mut ext = TestExternalities::from(t);
        ext.execute_with(|| {
            System::set_block_number(1);
        });
        ext
    }
}
