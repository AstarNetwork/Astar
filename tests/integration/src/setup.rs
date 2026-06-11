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

//! Runtime integration tests setup & imports.

pub use frame_support::{
    assert_noop, assert_ok,
    traits::{OnFinalize, OnIdle, OnInitialize},
    weights::{Weight, WeightToFee as WeightToFeeT},
};
use parity_scale_codec::Encode;
pub use sp_core::{sr25519, Get, Pair, H160};
pub use sp_runtime::{AccountId32, Digest, DigestItem, MultiAddress};

use cumulus_pallet_parachain_system::parachain_inherent::{
    BasicParachainInherentData, InboundMessagesData,
};
use cumulus_primitives_core::{relay_chain::HeadData, PersistedValidationData};
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use sp_consensus_aura::{Slot, SlotDuration, AURA_ENGINE_ID};

#[cfg(any(feature = "shibuya", feature = "astar"))]
pub use astar_primitives::governance::{
    CommunityCouncilMembershipInst, MainCouncilCollectiveInst, MainCouncilMembershipInst,
    TechnicalCommitteeCollectiveInst, TechnicalCommitteeMembershipInst,
};
pub use astar_primitives::{genesis::GenesisAccount, BlockNumber};

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
use cumulus_pallet_parachain_system::RelaychainDataProvider;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_dapp_staking as DappStakingCall;
pub use pallet_proxy::Event as ProxyEvent;
pub use pallet_session::Call as SessionCall;
pub use pallet_utility::{Call as UtilityCall, Event as UtilityEvent};
use sp_runtime::traits::BlockNumberProvider;

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
        let mut t = frame_system::GenesisConfig::<Runtime>::default()
            .build_storage()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime> {
            balances: self.balances,
            ..Default::default()
        }
        .assimilate_storage(&mut t)
        .unwrap();

        <pallet_aura::GenesisConfig<Runtime> as BuildStorage>::assimilate_storage(
            &pallet_aura::GenesisConfig::<Runtime> {
                authorities: vec![GenesisAccount::<sr25519::Public>::from_seed("Alice")
                    .pub_key()
                    .into()],
            },
            &mut t,
        )
        .unwrap();

        <pallet_session::GenesisConfig<Runtime> as BuildStorage>::assimilate_storage(
            &pallet_session::GenesisConfig::<Runtime> {
                keys: [ALICE, BOB, CAT]
                    .iter()
                    .map(|i| {
                        (
                            i.clone(),
                            i.clone(),
                            SessionKeys {
                                aura: sr25519::Pair::from_seed_slice(i.encode().as_slice())
                                    .unwrap()
                                    .public()
                                    .into(),
                            },
                        )
                    })
                    .collect::<_>(),
                ..Default::default()
            },
            &mut t,
        )
        .unwrap();

        <pallet_collator_selection::GenesisConfig<Runtime> as BuildStorage>::assimilate_storage(
            &pallet_collator_selection::GenesisConfig::<Runtime> {
                candidacy_bond: 100 * UNIT,
                desired_candidates: 2,
                ..Default::default()
            },
            &mut t,
        )
        .unwrap();

        // Needed to trigger initial inflation config setting.
        <pallet_inflation::GenesisConfig<Runtime> as BuildStorage>::assimilate_storage(
            &pallet_inflation::GenesisConfig::default(),
            &mut t,
        )
        .unwrap();

        #[cfg(any(feature = "shibuya", feature = "astar"))]
        // Governance related storage initialization
        {
            <pallet_membership::GenesisConfig<Runtime, MainCouncilMembershipInst> as BuildStorage>::assimilate_storage(
                &pallet_membership::GenesisConfig::<Runtime, MainCouncilMembershipInst> {
                    members: vec![ALICE, BOB, CAT].try_into().expect("Safe to assume at least 3 members are supported."),
                    ..Default::default()
                },
                &mut t)
            .unwrap();

            <pallet_membership::GenesisConfig<Runtime, TechnicalCommitteeMembershipInst> as BuildStorage>::assimilate_storage(
                &pallet_membership::GenesisConfig::<Runtime, TechnicalCommitteeMembershipInst> {
                    members: vec![ALICE, BOB, CAT].try_into().expect("Safe to assume at least 3 members are supported."),
                    ..Default::default()
                },
                &mut t)
            .unwrap();

            <pallet_membership::GenesisConfig<Runtime, CommunityCouncilMembershipInst> as BuildStorage>::assimilate_storage(
                &pallet_membership::GenesisConfig::<Runtime, CommunityCouncilMembershipInst> {
                    members: vec![ALICE, BOB, CAT].try_into().expect("Safe to assume at least 3 members are supported."),
                    ..Default::default()
                },
                &mut t)
            .unwrap();
        }

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            // Ensure the initial state is set for the first block
            System::initialize(
                &1,
                &Default::default(),
                &Digest {
                    logs: vec![DigestItem::PreRuntime(
                        AURA_ENGINE_ID,
                        Slot::from(1).encode(),
                    )],
                },
            );
            AllPalletsWithoutSystem::on_initialize(1);
            set_timestamp();
            set_validation_data();

            pallet_dapp_staking::Safeguard::<Runtime>::put(false);
        });
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

fn set_timestamp() {
    assert_ok!(Timestamp::set(
        RuntimeOrigin::none(),
        pallet_timestamp::Now::<Runtime>::get() + SLOT_DURATION
    ));
}

fn set_validation_data() {
    let para_id = <Runtime as cumulus_pallet_parachain_system::Config>::SelfParaId::get();

    let parent_head = HeadData(b"deadbeef".into());
    let sproof_builder = RelayStateSproofBuilder {
        para_id,
        included_para_head: Some(parent_head.clone()),
        current_slot: Slot::from_timestamp(
            pallet_timestamp::Now::<Runtime>::get().into(),
            SlotDuration::from_millis(6_000),
        ),
        ..Default::default()
    };

    #[cfg(feature = "shibuya")]
    let relay_parent_offset = shibuya_runtime::RelayParentOffset::get();
    #[cfg(feature = "shiden")]
    let relay_parent_offset = shiden_runtime::RelayParentOffset::get();
    #[cfg(feature = "astar")]
    let relay_parent_offset = astar_runtime::RelayParentOffset::get();

    let (relay_parent_storage_root, relay_chain_state, relay_parent_descendants) =
        sproof_builder.into_state_root_proof_and_descendants(relay_parent_offset.into());
    let basic_inherent_data = BasicParachainInherentData {
        validation_data: PersistedValidationData {
            parent_head,
            relay_parent_number: RelaychainDataProvider::<Runtime>::current_block_number() + 1,
            relay_parent_storage_root,
            max_pov_size: polkadot_primitives::MAX_POV_SIZE,
        },
        relay_chain_state,
        relay_parent_descendants,
        collator_peer_id: Default::default(),
    };
    let inbound_messages_data = InboundMessagesData::new(Default::default(), Default::default());

    assert_ok!(ParachainSystem::set_validation_data(
        RuntimeOrigin::none(),
        basic_inherent_data,
        inbound_messages_data,
    ));
}

pub fn run_to_block(n: BlockNumber) {
    while System::block_number() < n {
        let block_number = System::block_number();

        // finalize block
        AllPalletsWithSystem::on_idle(block_number, Weight::MAX.div(2));
        AllPalletsWithSystem::on_finalize(block_number);

        // Mock some storage to make consensus hook happy
        sp_io::storage::clear(&frame_support::storage::storage_prefix(
            b"ParachainSystem",
            b"UnincludedSegment",
        ));

        sp_io::storage::set(
            &frame_support::storage::storage_prefix(b"AuraExt", b"SlotInfo"),
            &(pallet_aura::CurrentSlot::<Runtime>::get(), 0u32).encode(),
        );

        // initialize block
        let slot = Slot::from_timestamp(
            (pallet_timestamp::Now::<Runtime>::get() + SLOT_DURATION).into(),
            SlotDuration::from_millis(SLOT_DURATION),
        );
        System::initialize(
            &(block_number + 1),
            &Default::default(),
            &Digest {
                logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, slot.encode())],
            },
        );
        AllPalletsWithoutSystem::on_initialize(block_number + 1);
        // apply inherent
        set_timestamp();
        set_validation_data();
    }
}

pub fn run_for_blocks(n: BlockNumber) {
    run_to_block(System::block_number() + n)
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

/// Initialize `env_logger` for tests. It will enable logging like `DEBUG`
/// and `TRACE` in runtime.
#[allow(dead_code)]
pub fn init_env_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}
