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

use crate::*;

use fp_evm::{IsPrecompileResult, Precompile};
use frame_support::{
    assert_ok, construct_runtime, parameter_types,
    traits::{
        fungible::{Mutate as FunMutate, Unbalanced as FunUnbalanced},
        ConstU128, ConstU64, GenesisBuild, Hooks,
    },
    weights::{RuntimeDbWeight, Weight},
};
use frame_system::RawOrigin;
use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use sp_arithmetic::{fixed_point::FixedU64, Permill};
use sp_core::{H160, H256};
use sp_io::TestExternalities;
use sp_runtime::traits::{BlakeTwo256, ConstU32, IdentityLookup};
extern crate alloc;

use astar_primitives::{
    dapp_staking::{CycleConfiguration, SmartContract, StakingRewardHandler},
    testing::Header,
    AccountId, Balance, BlockNumber,
};
use pallet_dapp_staking_v3::{EraNumber, PeriodNumber, PriceProvider, TierThreshold};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub struct AddressMapper;
impl AddressMapping<AccountId> for AddressMapper {
    fn into_account_id(account: H160) -> AccountId {
        let mut account_id = [0u8; 32];
        account_id[0..20].clone_from_slice(&account.as_bytes());

        account_id
            .try_into()
            .expect("H160 is 20 bytes long so it must fit into 32 bytes; QED")
    }
}

pub const READ_WEIGHT: u64 = 3;
pub const WRITE_WEIGHT: u64 = 7;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
        pub const TestWeights: RuntimeDbWeight = RuntimeDbWeight {
            read: READ_WEIGHT,
            write: WRITE_WEIGHT,
        };
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
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type HoldIdentifier = ();
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<1>;
    type WeightInfo = ();
}

pub fn precompile_address() -> H160 {
    H160::from_low_u64_be(0x5001)
}

#[derive(Debug, Clone, Copy)]
pub struct DappStakingPrecompile<R>(PhantomData<R>);
impl<R> PrecompileSet for DappStakingPrecompile<R>
where
    R: pallet_evm::Config,
    DappStakingV3Precompile<R>: Precompile,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        match handle.code_address() {
            a if a == precompile_address() => Some(DappStakingV3Precompile::<R>::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: sp_core::H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: address == precompile_address(),
            extra_cost: 0,
        }
    }
}

pub type PrecompileCall = DappStakingV3PrecompileCall<Test>;

parameter_types! {
    pub PrecompilesValue: DappStakingPrecompile<Test> = DappStakingPrecompile(Default::default());
    pub WeightPerGas: Weight = Weight::from_parts(1, 0);
}

impl pallet_evm::Config for Test {
    type FeeCalculator = ();
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = AddressMapper;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = DappStakingPrecompile<Test>;
    type PrecompilesValue = PrecompilesValue;
    type Timestamp = Timestamp;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
    type OnCreate = ();
    type WeightInfo = ();
    type GasLimitPovSizeRatio = ConstU64<4>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<5>;
    type WeightInfo = ();
}

type MockSmartContract = SmartContract<<Test as frame_system::Config>::AccountId>;

pub struct DummyPriceProvider;
impl PriceProvider for DummyPriceProvider {
    fn average_price() -> FixedU64 {
        FixedU64::from_rational(1, 10)
    }
}

pub struct DummyStakingRewardHandler;
impl StakingRewardHandler<AccountId> for DummyStakingRewardHandler {
    fn staker_and_dapp_reward_pools(_total_staked_value: Balance) -> (Balance, Balance) {
        (
            Balance::from(1_000_000_000_000_u128),
            Balance::from(1_000_000_000_u128),
        )
    }

    fn bonus_reward_pool() -> Balance {
        Balance::from(3_000_000_u128)
    }

    fn payout_reward(beneficiary: &AccountId, reward: Balance) -> Result<(), ()> {
        let _ = Balances::mint_into(beneficiary, reward);
        Ok(())
    }
}

pub struct DummyCycleConfiguration;
impl CycleConfiguration for DummyCycleConfiguration {
    fn periods_per_cycle() -> u32 {
        4
    }

    fn eras_per_voting_subperiod() -> u32 {
        8
    }

    fn eras_per_build_and_earn_subperiod() -> u32 {
        16
    }

    fn blocks_per_era() -> u32 {
        10
    }
}

// Just to satsify the trait bound
#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper<SC, ACC>(sp_std::marker::PhantomData<(SC, ACC)>);
#[cfg(feature = "runtime-benchmarks")]
impl pallet_dapp_staking_v3::BenchmarkHelper<MockSmartContract, AccountId>
    for BenchmarkHelper<MockSmartContract, AccountId>
{
    fn get_smart_contract(id: u32) -> MockSmartContract {
        MockSmartContract::evm(H160::from_low_u64_be(id as u64))
    }

    fn set_balance(_account: &AccountId, _amount: Balance) {}
}

impl pallet_dapp_staking_v3::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type Currency = Balances;
    type SmartContract = MockSmartContract;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type NativePriceProvider = DummyPriceProvider;
    type StakingRewardHandler = DummyStakingRewardHandler;
    type CycleConfiguration = DummyCycleConfiguration;
    type EraRewardSpanLength = ConstU32<8>;
    type RewardRetentionInPeriods = ConstU32<2>;
    type MaxNumberOfContracts = ConstU32<10>;
    type MaxUnlockingChunks = ConstU32<5>;
    type MinimumLockedAmount = ConstU128<10>;
    type UnlockingPeriod = ConstU32<2>;
    type MaxNumberOfStakedContracts = ConstU32<5>;
    type MinimumStakeAmount = ConstU128<3>;
    type NumberOfTiers = ConstU32<4>;
    type WeightInfo = pallet_dapp_staking_v3::weights::SubstrateWeight<Test>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper<MockSmartContract, AccountId>;
}

construct_runtime!(
    pub struct Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        Evm: pallet_evm,
        Timestamp: pallet_timestamp,
        DappStaking: pallet_dapp_staking_v3,
    }
);

pub struct ExternalityBuilder;
impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        <pallet_dapp_staking_v3::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(
            &pallet_dapp_staking_v3::GenesisConfig {
                reward_portion: vec![
                    Permill::from_percent(40),
                    Permill::from_percent(30),
                    Permill::from_percent(20),
                    Permill::from_percent(10),
                ],
                slot_distribution: vec![
                    Permill::from_percent(10),
                    Permill::from_percent(20),
                    Permill::from_percent(30),
                    Permill::from_percent(40),
                ],
                tier_thresholds: vec![
                    TierThreshold::DynamicTvlAmount {
                        amount: 100,
                        minimum_amount: 80,
                    },
                    TierThreshold::DynamicTvlAmount {
                        amount: 50,
                        minimum_amount: 40,
                    },
                    TierThreshold::DynamicTvlAmount {
                        amount: 20,
                        minimum_amount: 20,
                    },
                    TierThreshold::FixedTvlAmount { amount: 10 },
                ],
                slots_per_tier: vec![10, 20, 30, 40],
            },
            &mut storage,
        )
        .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| {
            System::set_block_number(1);

            let alice_native = AddressMapper::into_account_id(ALICE);
            assert_ok!(
                <Test as pallet_dapp_staking_v3::Config>::Currency::write_balance(
                    &alice_native,
                    1000_000_000_000_000_000_000 as Balance,
                )
            );
        });
        ext
    }
}

pub fn precompiles() -> DappStakingPrecompile<Test> {
    PrecompilesValue::get()
}

// Utility functions

pub const ALICE: H160 = H160::repeat_byte(0xAA);

/// Used to register a smart contract, and stake some funds on it.
pub fn register_and_stake(
    account: H160,
    smart_contract: <Test as pallet_dapp_staking_v3::Config>::SmartContract,
    amount: Balance,
) {
    let alice_native = AddressMapper::into_account_id(account);

    // 1. Register smart contract
    assert_ok!(DappStaking::register(
        RawOrigin::Root.into(),
        alice_native.clone(),
        smart_contract.clone()
    ));

    // 2. Lock some amount
    assert_ok!(DappStaking::lock(
        RawOrigin::Signed(alice_native.clone()).into(),
        amount,
    ));

    // 3. Stake the locked amount
    assert_ok!(DappStaking::stake(
        RawOrigin::Signed(alice_native.clone()).into(),
        smart_contract.clone(),
        amount,
    ));
}

/// Utility function used to create `DynamicAddress` out of the given `H160` address.
/// The first one is simply byte representation of the H160 address.
/// The second one is byte representation of the derived `AccountId` from the H160 address.
pub fn into_dynamic_addresses(address: H160) -> [DynamicAddress; 2] {
    [
        address.as_bytes().try_into().unwrap(),
        <AccountId as AsRef<[u8]>>::as_ref(&AddressMapper::into_account_id(address))
            .try_into()
            .unwrap(),
    ]
}

/// Initialize first block.
/// This method should only be called once in a UT otherwise the first block will get initialized multiple times.
pub fn initialize() {
    // This assert prevents method misuse
    assert_eq!(System::block_number(), 1 as BlockNumber);
    DappStaking::on_initialize(System::block_number());
    run_to_block(2);
}

/// Run to the specified block number.
/// Function assumes first block has been initialized.
pub(crate) fn run_to_block(n: BlockNumber) {
    while System::block_number() < n {
        DappStaking::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        DappStaking::on_initialize(System::block_number());
    }
}

/// Run for the specified number of blocks.
/// Function assumes first block has been initialized.
pub(crate) fn run_for_blocks(n: BlockNumber) {
    run_to_block(System::block_number() + n);
}

/// Advance blocks until the specified era has been reached.
///
/// Function has no effect if era is already passed.
pub(crate) fn advance_to_era(era: EraNumber) {
    assert!(era >= ActiveProtocolState::<Test>::get().era);
    while ActiveProtocolState::<Test>::get().era < era {
        run_for_blocks(1);
    }
}

/// Advance blocks until next era has been reached.
pub(crate) fn advance_to_next_era() {
    advance_to_era(ActiveProtocolState::<Test>::get().era + 1);
}

/// Advance blocks until next period type has been reached.
pub(crate) fn advance_to_next_subperiod() {
    let subperiod = ActiveProtocolState::<Test>::get().subperiod();
    while ActiveProtocolState::<Test>::get().subperiod() == subperiod {
        run_for_blocks(1);
    }
}

/// Advance blocks until the specified period has been reached.
///
/// Function has no effect if period is already passed.
pub(crate) fn advance_to_period(period: PeriodNumber) {
    assert!(period >= ActiveProtocolState::<Test>::get().period_number());
    while ActiveProtocolState::<Test>::get().period_number() < period {
        run_for_blocks(1);
    }
}

/// Advance blocks until next period has been reached.
pub(crate) fn advance_to_next_period() {
    advance_to_period(ActiveProtocolState::<Test>::get().period_number() + 1);
}

// Return all dApp staking events from the event buffer.
pub fn dapp_staking_events() -> Vec<pallet_dapp_staking_v3::Event<Test>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            <Test as pallet_dapp_staking_v3::Config>::RuntimeEvent::from(e)
                .try_into()
                .ok()
        })
        .collect::<Vec<_>>()
}
