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
    weights::Weight,
};
use parity_scale_codec::Encode;
pub use sp_core::{Get, H160};
pub use sp_runtime::{AccountId32, MultiAddress};

use cumulus_primitives_core::{relay_chain::HeadData, PersistedValidationData};
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;

pub use astar_primitives::{
    dapp_staking::CycleConfiguration,
    governance::{
        CommunityCouncilMembershipInst, MainCouncilMembershipInst, OracleMembershipInst,
        TechnicalCommitteeMembershipInst,
    },
    oracle::Price,
    BlockNumber,
};

#[cfg(feature = "shibuya")]
pub use shibuya::*;
#[cfg(feature = "shibuya")]
mod shibuya {
    use super::*;
    pub use shibuya_runtime::*;

    /// 1 SBY.
    pub const UNIT: Balance = SBY;

    pub fn alith_secret_key() -> libsecp256k1::SecretKey {
        libsecp256k1::SecretKey::parse(&sp_io::hashing::keccak_256(b"Alith")).unwrap()
    }

    /// H160 address mapped to `ALICE`.
    pub fn alith() -> H160 {
        UnifiedAccounts::eth_address(&alith_secret_key())
    }

    /// Build the signature payload for given native account and eth private key
    fn get_evm_signature(who: &AccountId32, secret: &libsecp256k1::SecretKey) -> [u8; 65] {
        // sign the payload
        UnifiedAccounts::eth_sign_prehash(&UnifiedAccounts::build_signing_payload(who), secret)
    }

    /// Create the mappings for the accounts
    pub fn connect_accounts(who: &AccountId32, secret: &libsecp256k1::SecretKey) {
        assert_ok!(UnifiedAccounts::claim_evm_address(
            RuntimeOrigin::signed(who.clone()),
            UnifiedAccounts::eth_address(secret),
            get_evm_signature(who, secret)
        ));
    }
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

pub const INIT_PRICE: Price = Price::from_rational(1, 10);

pub type SystemError = frame_system::Error<Runtime>;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_dapp_staking_v3 as DappStakingCall;
pub use pallet_proxy::Event as ProxyEvent;
pub use pallet_utility::{Call as UtilityCall, Event as UtilityEvent};
use parity_scale_codec::Decode;

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
        }
        .assimilate_storage(&mut t)
        .unwrap();

        // Setup initial oracle members
        <pallet_membership::GenesisConfig<Runtime, OracleMembershipInst> as BuildStorage>::assimilate_storage(
            &pallet_membership::GenesisConfig::<Runtime, OracleMembershipInst> {
                members: vec![ALICE, BOB].try_into().expect("Safe to assume at least 2 members are supported."),
                ..Default::default()
            },
            &mut t)
        .unwrap();

        // Setup initial native currency price
        <pallet_price_aggregator::GenesisConfig<Runtime> as BuildStorage>::assimilate_storage(
            &pallet_price_aggregator::GenesisConfig::<Runtime> {
                circular_buffer: vec![INIT_PRICE].try_into().unwrap(),
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

        #[cfg(any(feature = "shibuya"))]
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
            System::set_block_number(1);

            let era_length = <Runtime as pallet_dapp_staking_v3::Config>::CycleConfiguration::blocks_per_era();
            let voting_period_length_in_eras =
            <Runtime as pallet_dapp_staking_v3::Config>::CycleConfiguration::eras_per_voting_subperiod();

            pallet_dapp_staking_v3::ActiveProtocolState::<Runtime>::put(pallet_dapp_staking_v3::ProtocolState {
                era: 1,
                next_era_start: era_length.saturating_mul(voting_period_length_in_eras.into()) + 1,
                period_info: pallet_dapp_staking_v3::PeriodInfo {
                    number: 1,
                    subperiod: pallet_dapp_staking_v3::Subperiod::Voting,
                    next_subperiod_start_era: 2,
                },
                maintenance: false,
            });
            pallet_dapp_staking_v3::Safeguard::<Runtime>::put(false);

            // Ensure the initial state is set for the first block
            AllPalletsWithSystem::on_initialize(1);
            set_timestamp();
            set_validation_data();
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
    let block_number = System::block_number();
    let para_id = <Runtime as cumulus_pallet_parachain_system::Config>::SelfParaId::get();

    let parent_head = HeadData(b"deadbeef".into());
    let sproof_builder = RelayStateSproofBuilder {
        para_id,
        included_para_head: Some(parent_head.clone()),
        ..Default::default()
    };
    let (relay_parent_storage_root, relay_chain_state) = sproof_builder.into_state_root_and_proof();
    let para_inherent_data = ParachainInherentData {
        validation_data: PersistedValidationData {
            parent_head,
            relay_parent_number: block_number,
            relay_parent_storage_root,
            max_pov_size: 5_000_000,
        },
        relay_chain_state,
        downward_messages: Default::default(),
        horizontal_messages: Default::default(),
    };

    assert_ok!(ParachainSystem::set_validation_data(
        RuntimeOrigin::none(),
        para_inherent_data
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
        System::set_block_number(block_number + 1);
        AllPalletsWithSystem::on_initialize(block_number + 1);
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

/// Deploy a WASM contract via ALICE as origin. (The code is in `../ink-contracts/`.)
/// Assumption: Contract constructor is called "new" and take no arguments
pub fn deploy_wasm_contract(name: &str) -> AccountId32 {
    let (address, _) = astar_test_utils::deploy_wasm_contract::<Runtime>(
        name,
        ALICE,
        0,
        Weight::from_parts(10_000_000_000, 1024 * 1024),
        None,
        hex::decode("9bae9d5e").expect("invalid data hex"),
    );

    // On instantiation, the contract got existential deposit.
    assert_eq!(Balances::free_balance(&address), ExistentialDeposit::get(),);
    address
}

/// Call a wasm smart contract method
pub fn call_wasm_contract_method<V: Decode>(
    origin: AccountId,
    contract_id: AccountId,
    data: Vec<u8>,
) -> V {
    let (value, _, _) = astar_test_utils::call_wasm_contract_method::<Runtime, V>(
        origin,
        contract_id,
        0,
        Weight::from_parts(10_000_000_000, 1024 * 1024),
        None,
        data,
        false,
    );
    value
}
