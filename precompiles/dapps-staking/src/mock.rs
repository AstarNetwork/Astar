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

use super::*;

use frame_support::{
    construct_runtime, parameter_types,
    traits::{Currency, OnFinalize, OnInitialize},
    weights::{RuntimeDbWeight, Weight},
    PalletId,
};
use pallet_dapps_staking::weights;
use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256};
use sp_io::TestExternalities;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, ConstU32, IdentityLookup},
    AccountId32,
};
extern crate alloc;

pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type EraIndex = u32;
pub(crate) const MILLIAST: Balance = 1_000_000_000_000_000;
pub(crate) const AST: Balance = 1_000 * MILLIAST;

pub(crate) const TEST_CONTRACT: H160 = H160::repeat_byte(0x09);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const MAX_NUMBER_OF_STAKERS: u32 = 4;
/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const MINIMUM_STAKING_AMOUNT: Balance = 10 * AST;
pub(crate) const MINIMUM_REMAINING_AMOUNT: Balance = 1;
pub(crate) const MAX_UNLOCKING_CHUNKS: u32 = 4;
pub(crate) const UNBONDING_PERIOD: EraIndex = 3;
pub(crate) const MAX_ERA_STAKE_VALUES: u32 = 10;

// Do note that this needs to at least be 3 for tests to be valid. It can be greater but not smaller.
pub(crate) const BLOCKS_PER_ERA: BlockNumber = 3;

pub(crate) const REGISTER_DEPOSIT: Balance = 10 * AST;

pub(crate) const STAKER_BLOCK_REWARD: Balance = 531911;
pub(crate) const DAPP_BLOCK_REWARD: Balance = 773333;

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Encode,
    Decode,
    Debug,
    MaxEncodedLen,
    Serialize,
    Deserialize,
    derive_more::Display,
    scale_info::TypeInfo,
)]

pub enum TestAccount {
    Empty,
    Alex,
    Bobo,
    Dino,
}

impl Default for TestAccount {
    fn default() -> Self {
        Self::Empty
    }
}

// needed for associated type in pallet_evm
impl AddressMapping<AccountId32> for TestAccount {
    fn into_account_id(h160_account: H160) -> AccountId32 {
        match h160_account {
            a if a == H160::repeat_byte(0x01) => TestAccount::Alex.into(),
            a if a == H160::repeat_byte(0x02) => TestAccount::Bobo.into(),
            a if a == H160::repeat_byte(0x03) => TestAccount::Dino.into(),
            _ => TestAccount::Empty.into(),
        }
    }
}

impl From<TestAccount> for H160 {
    fn from(x: TestAccount) -> H160 {
        match x {
            TestAccount::Alex => H160::repeat_byte(0x01),
            TestAccount::Bobo => H160::repeat_byte(0x02),
            TestAccount::Dino => H160::repeat_byte(0x03),
            _ => Default::default(),
        }
    }
}

trait H160Conversion {
    fn to_h160(&self) -> H160;
}

impl H160Conversion for AccountId32 {
    fn to_h160(&self) -> H160 {
        let x = self.encode()[31];
        H160::repeat_byte(x)
    }
}

impl From<TestAccount> for AccountId32 {
    fn from(x: TestAccount) -> Self {
        match x {
            TestAccount::Alex => AccountId32::from([1u8; 32]),
            TestAccount::Bobo => AccountId32::from([2u8; 32]),
            TestAccount::Dino => AccountId32::from([3u8; 32]),
            _ => AccountId32::from([0u8; 32]),
        }
    }
}

pub const READ_WEIGHT: u64 = 3;
pub const WRITE_WEIGHT: u64 = 7;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1024));
    pub const TestWeights: RuntimeDbWeight = RuntimeDbWeight {
        read: READ_WEIGHT,
        write: WRITE_WEIGHT,
    };
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
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<AccountId32>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = TestWeights;
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

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}
impl pallet_balances::Config for TestRuntime {
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 4];
    type MaxLocks = ();
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

pub fn precompile_address() -> H160 {
    H160::from_low_u64_be(0x5001)
}

#[derive(Debug, Clone, Copy)]
pub struct DappPrecompile<R>(PhantomData<R>);

impl<R> PrecompileSet for DappPrecompile<R>
where
    R: pallet_evm::Config,
    DappsStakingWrapper<R>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == precompile_address() => Some(DappsStakingWrapper::<R>::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: sp_core::H160) -> bool {
        address == precompile_address()
    }
}

parameter_types! {
    pub PrecompilesValue: DappPrecompile<TestRuntime> = DappPrecompile(Default::default());
    pub WeightPerGas: Weight = Weight::from_ref_time(1);
}

impl pallet_evm::Config for TestRuntime {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId32>;
    type WithdrawOrigin = EnsureAddressNever<AccountId32>;
    type AddressMapping = TestAccount;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = DappPrecompile<TestRuntime>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
    type OnCreate = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

#[derive(
    PartialEq, Eq, Copy, Clone, Encode, Decode, Debug, scale_info::TypeInfo, MaxEncodedLen,
)]
pub enum MockSmartContract<AccountId32> {
    Evm(sp_core::H160),
    Wasm(AccountId32),
}

impl<AccountId32> Default for MockSmartContract<AccountId32> {
    fn default() -> Self {
        MockSmartContract::Evm(H160::repeat_byte(0x00))
    }
}

parameter_types! {
    pub const RegisterDeposit: Balance = REGISTER_DEPOSIT;
    pub const BlockPerEra: BlockNumber = BLOCKS_PER_ERA;
    pub const MaxNumberOfStakersPerContract: u32 = MAX_NUMBER_OF_STAKERS;
    pub const MinimumStakingAmount: Balance = MINIMUM_STAKING_AMOUNT;
    pub const DappsStakingPalletId: PalletId = PalletId(*b"mokdpstk");
    pub const MinimumRemainingAmount: Balance = MINIMUM_REMAINING_AMOUNT;
    pub const MaxUnlockingChunks: u32 = MAX_UNLOCKING_CHUNKS;
    pub const UnbondingPeriod: EraIndex = UNBONDING_PERIOD;
    pub const MaxEraStakeValues: u32 = MAX_ERA_STAKE_VALUES;
}

impl pallet_dapps_staking::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockPerEra = BlockPerEra;
    type RegisterDeposit = RegisterDeposit;
    type SmartContract = MockSmartContract<AccountId32>;
    type WeightInfo = weights::SubstrateWeight<TestRuntime>;
    type MaxNumberOfStakersPerContract = MaxNumberOfStakersPerContract;
    type MinimumStakingAmount = MinimumStakingAmount;
    type PalletId = DappsStakingPalletId;
    type MinimumRemainingAmount = MinimumRemainingAmount;
    type MaxUnlockingChunks = MaxUnlockingChunks;
    type UnbondingPeriod = UnbondingPeriod;
    type MaxEraStakeValues = MaxEraStakeValues;
    type UnregisteredDappRewardRetention = ConstU32<2>;
}

pub struct ExternalityBuilder {
    balances: Vec<(AccountId32, Balance)>,
}

impl Default for ExternalityBuilder {
    fn default() -> ExternalityBuilder {
        ExternalityBuilder { balances: vec![] }
    }
}

impl ExternalityBuilder {
    pub fn build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: self.balances,
        }
        .assimilate_storage(&mut storage)
        .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }

    pub(crate) fn with_balances(mut self, balances: Vec<(AccountId32, Balance)>) -> Self {
        self.balances = balances;
        self
    }
}

construct_runtime!(
    pub struct TestRuntime
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        Evm: pallet_evm,
        Timestamp: pallet_timestamp,
        DappsStaking: pallet_dapps_staking,
    }
);

/// Used to run to the specified block number
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        DappsStaking::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        // This is performed outside of dapps staking but we expect it before on_initialize
        payout_block_rewards();
        DappsStaking::on_initialize(System::block_number());
    }
}

/// Used to run the specified number of blocks
pub fn run_for_blocks(n: u64) {
    run_to_block(System::block_number() + n);
}

/// Advance blocks to the beginning of an era.
///
/// Function has no effect if era is already passed.
pub fn advance_to_era(n: EraIndex) {
    while DappsStaking::current_era() < n {
        run_for_blocks(1);
    }
}

/// Initialize first block.
/// This method should only be called once in a UT otherwise the first block will get initialized multiple times.
pub fn initialize_first_block() {
    // This assert prevents method misuse
    assert_eq!(System::block_number(), 1 as BlockNumber);

    // This is performed outside of dapps staking but we expect it before on_initialize
    payout_block_rewards();
    DappsStaking::on_initialize(System::block_number());
    run_to_block(2);
}

/// Returns total block rewards that goes to dapps-staking.
/// Contains both `dapps` reward and `stakers` reward.
pub fn joint_block_reward() -> Balance {
    STAKER_BLOCK_REWARD + DAPP_BLOCK_REWARD
}

/// Payout block rewards to stakers & dapps
fn payout_block_rewards() {
    DappsStaking::rewards(
        Balances::issue(STAKER_BLOCK_REWARD.into()),
        Balances::issue(DAPP_BLOCK_REWARD.into()),
    );
}
