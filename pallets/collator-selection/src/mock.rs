// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate as collator_selection;
use frame_support::{
    ord_parameter_types, parameter_types,
    traits::{FindAuthor, GenesisBuild, ValidatorRegistration},
    PalletId,
};
use frame_system as system;
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, IdentityLookup, OpaqueKeys},
    Perbill, RuntimeAppPublic,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub struct Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Session: pallet_session,
        Aura: pallet_aura,
        Balances: pallet_balances,
        CollatorSelection: collator_selection,
        Authorship: pallet_authorship,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 5;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

pub struct Author4;
impl FindAuthor<u64> for Author4 {
    fn find_author<'a, I>(_digests: I) -> Option<u64>
    where
        I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
    {
        Some(4)
    }
}

impl pallet_authorship::Config for Test {
    type FindAuthor = Author4;
    type EventHandler = CollatorSelection;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_aura::Config for Test {
    type AuthorityId = sp_consensus_aura::sr25519::AuthorityId;
    type MaxAuthorities = MaxAuthorities;
    type DisabledValidators = ();
}

sp_runtime::impl_opaque_keys! {
    pub struct MockSessionKeys {
        // a key for aura authoring
        pub aura: UintAuthorityId,
    }
}

impl From<UintAuthorityId> for MockSessionKeys {
    fn from(aura: sp_runtime::testing::UintAuthorityId) -> Self {
        Self { aura }
    }
}

parameter_types! {
    pub static SessionHandlerCollators: Vec<u64> = Vec::new();
    pub static SessionChangeBlock: u64 = 0;
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<u64> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
    fn on_genesis_session<Ks: OpaqueKeys>(keys: &[(u64, Ks)]) {
        SessionHandlerCollators::set(keys.into_iter().map(|(a, _)| *a).collect::<Vec<_>>())
    }
    fn on_new_session<Ks: OpaqueKeys>(_: bool, keys: &[(u64, Ks)], _: &[(u64, Ks)]) {
        SessionChangeBlock::set(System::block_number());
        dbg!(keys.len());
        SessionHandlerCollators::set(keys.into_iter().map(|(a, _)| *a).collect::<Vec<_>>())
    }
    fn on_before_session_ending() {}
    fn on_disabled(_: u32) {}
}

parameter_types! {
    pub const Offset: u64 = 0;
    pub const Period: u64 = 10;
}

impl pallet_session::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    type SessionHandler = TestSessionHandler;
    type Keys = MockSessionKeys;
    type WeightInfo = ();
}

ord_parameter_types! {
    pub const RootAccount: u64 = 777;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const MaxCandidates: u32 = 20;
    pub const MaxInvulnerables: u32 = 20;
    pub const MinCandidates: u32 = 1;
    pub const MaxAuthorities: u32 = 100_000;
    pub const SlashRatio: Perbill = Perbill::from_percent(10);
}

pub struct IsRegistered;
impl ValidatorRegistration<u64> for IsRegistered {
    fn is_registered(id: &u64) -> bool {
        if *id == 7u64 {
            false
        } else {
            true
        }
    }
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = EnsureSignedBy<RootAccount, u64>;
    type PotId = PotId;
    type MaxCandidates = MaxCandidates;
    type MinCandidates = MinCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = IdentityCollator;
    type ValidatorRegistration = IsRegistered;
    type SlashRatio = SlashRatio;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    sp_tracing::try_init_simple();
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    let invulnerables = vec![1, 2];

    let balances = vec![(1, 100), (2, 100), (3, 100), (4, 100), (5, 100)];
    let keys = balances
        .iter()
        .map(|&(i, _)| {
            (
                i,
                i,
                MockSessionKeys {
                    aura: UintAuthorityId(i),
                },
            )
        })
        .collect::<Vec<_>>();
    let collator_selection = collator_selection::GenesisConfig::<Test> {
        desired_candidates: 2,
        candidacy_bond: 10,
        invulnerables,
    };
    let session = pallet_session::GenesisConfig::<Test> { keys };
    pallet_balances::GenesisConfig::<Test> { balances }
        .assimilate_storage(&mut t)
        .unwrap();
    // collator selection must be initialized before session.
    collator_selection.assimilate_storage(&mut t).unwrap();
    session.assimilate_storage(&mut t).unwrap();

    t.into()
}

pub fn initialize_to_block(n: u64) {
    for i in System::block_number() + 1..=n {
        System::set_block_number(i);
        <AllPalletsWithSystem as frame_support::traits::OnInitialize<u64>>::on_initialize(i);
    }
}
