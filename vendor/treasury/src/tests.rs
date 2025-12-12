// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Treasury pallet tests.

#![cfg(test)]

use core::marker::PhantomData;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{tokens::ConversionFromAssetBalance, ConstU32, ConstU64, OnInitialize},
    PalletId,
};

use super::*;
use crate as treasury;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Treasury: treasury,
        Utility: pallet_utility,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type AccountId = u128; // u64 is not enough to hold bytes used to generate bounty account
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type AccountStore = System;
}

impl pallet_utility::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = ();
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const Burn: Permill = Permill::from_percent(50);
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub TreasuryAccount: u128 = Treasury::account_id();
    pub const SpendPayoutPeriod: u64 = 5;
}

pub struct TestSpendOrigin;
impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for TestSpendOrigin {
    type Success = u64;
    fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
        Result::<frame_system::RawOrigin<_>, RuntimeOrigin>::from(o).and_then(|o| match o {
            frame_system::RawOrigin::Root => Ok(u64::max_value()),
            frame_system::RawOrigin::Signed(10) => Ok(5),
            frame_system::RawOrigin::Signed(11) => Ok(10),
            frame_system::RawOrigin::Signed(12) => Ok(20),
            frame_system::RawOrigin::Signed(13) => Ok(50),
            r => Err(RuntimeOrigin::from(r)),
        })
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(frame_system::RawOrigin::Root.into())
    }
}

pub struct MulBy<N>(PhantomData<N>);
impl<N: Get<u64>> ConversionFromAssetBalance<u64, u32, u64> for MulBy<N> {
    type Error = ();
    fn from_asset_balance(balance: u64, _asset_id: u32) -> Result<u64, Self::Error> {
        return balance.checked_mul(N::get()).ok_or(());
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn ensure_successful(_: u32) {}
}

impl Config for Test {
    type PalletId = TreasuryPalletId;
    type Currency = pallet_balances::Pallet<Test>;
    type ApproveOrigin = frame_system::EnsureRoot<u128>;
    type RejectOrigin = frame_system::EnsureRoot<u128>;
    type RuntimeEvent = RuntimeEvent;
    type OnSlash = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ConstU64<1>;
    type ProposalBondMaximum = ();
    type SpendPeriod = ConstU64<2>;
    type Burn = Burn;
    type BurnDestination = (); // Just gets burned.
    type WeightInfo = ();
    type SpendFunds = ();
    type MaxApprovals = ConstU32<100>;
}

#[derive(Default)]
pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> {
            // Total issuance will be 200 with treasury account initialized at ED.
            balances: vec![(0, 100), (1, 98), (2, 1)],
            ..Default::default()
        }
        .assimilate_storage(&mut t)
        .unwrap();
        crate::GenesisConfig::<Test>::default()
            .assimilate_storage(&mut t)
            .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

#[test]
fn genesis_config_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Treasury::pot(), 0);
        assert_eq!(Treasury::proposal_count(), 0);
    });
}

#[test]
fn minting_works() {
    ExtBuilder::default().build().execute_with(|| {
        // Check that accumulate works when we have Some value in Dummy already.
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_eq!(Treasury::pot(), 100);
    });
}

#[test]
fn spend_proposal_takes_min_deposit() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 1, 3)
        });
        assert_eq!(Balances::free_balance(0), 99);
        assert_eq!(Balances::reserved_balance(0), 1);
    });
}

#[test]
fn spend_proposal_takes_proportional_deposit() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
        });
        assert_eq!(Balances::free_balance(0), 95);
        assert_eq!(Balances::reserved_balance(0), 5);
    });
}

#[test]
fn spend_proposal_fails_when_proposer_poor() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            {
                #[allow(deprecated)]
                Treasury::propose_spend(RuntimeOrigin::signed(2), 100, 3)
            },
            Error::<Test, _>::InsufficientProposersBalance,
        );
    });
}

#[test]
fn accepted_spend_proposal_ignored_outside_spend_period() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 0)
        });

        <Treasury as OnInitialize<u64>>::on_initialize(1);
        assert_eq!(Balances::free_balance(3), 0);
        assert_eq!(Treasury::pot(), 100);
    });
}

#[test]
fn unused_pot_should_diminish() {
    ExtBuilder::default().build().execute_with(|| {
        let init_total_issuance = Balances::total_issuance();
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_eq!(Balances::total_issuance(), init_total_issuance + 100);

        <Treasury as OnInitialize<u64>>::on_initialize(2);
        assert_eq!(Treasury::pot(), 50);
        assert_eq!(Balances::total_issuance(), init_total_issuance + 50);
    });
}

#[test]
fn rejected_spend_proposal_ignored_on_spend_period() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::reject_proposal(RuntimeOrigin::root(), 0)
        });

        <Treasury as OnInitialize<u64>>::on_initialize(2);
        assert_eq!(Balances::free_balance(3), 0);
        assert_eq!(Treasury::pot(), 50);
    });
}

#[test]
fn reject_already_rejected_spend_proposal_fails() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::reject_proposal(RuntimeOrigin::root(), 0)
        });
        assert_noop!(
            {
                #[allow(deprecated)]
                Treasury::reject_proposal(RuntimeOrigin::root(), 0)
            },
            Error::<Test, _>::InvalidIndex
        );
    });
}

#[test]
fn reject_non_existent_spend_proposal_fails() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            {
                #[allow(deprecated)]
                Treasury::reject_proposal(RuntimeOrigin::root(), 0)
            },
            Error::<Test, _>::InvalidIndex
        );
    });
}

#[test]
fn accept_non_existent_spend_proposal_fails() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            {
                #[allow(deprecated)]
                Treasury::approve_proposal(RuntimeOrigin::root(), 0)
            },
            Error::<Test, _>::InvalidIndex
        );
    });
}

#[test]
fn accept_already_rejected_spend_proposal_fails() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::reject_proposal(RuntimeOrigin::root(), 0)
        });
        assert_noop!(
            {
                #[allow(deprecated)]
                Treasury::approve_proposal(RuntimeOrigin::root(), 0)
            },
            Error::<Test, _>::InvalidIndex
        );
    });
}

#[test]
fn accepted_spend_proposal_enacted_on_spend_period() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_eq!(Treasury::pot(), 100);

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 0)
        });

        <Treasury as OnInitialize<u64>>::on_initialize(2);
        assert_eq!(Balances::free_balance(3), 100);
        assert_eq!(Treasury::pot(), 0);
    });
}

#[test]
fn pot_underflow_should_not_diminish() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_eq!(Treasury::pot(), 100);

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 150, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 0)
        });

        <Treasury as OnInitialize<u64>>::on_initialize(2);
        assert_eq!(Treasury::pot(), 100); // Pot hasn't changed

        let _ = Balances::deposit_into_existing(&Treasury::account_id(), 100).unwrap();
        <Treasury as OnInitialize<u64>>::on_initialize(4);
        assert_eq!(Balances::free_balance(3), 150); // Fund has been spent
        assert_eq!(Treasury::pot(), 25); // Pot has finally changed
    });
}

// Treasury account doesn't get deleted if amount approved to spend is all its free balance.
// i.e. pot should not include existential deposit needed for account survival.
#[test]
fn treasury_account_doesnt_get_deleted() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), 101);
        assert_eq!(Treasury::pot(), 100);
        let treasury_balance = Balances::free_balance(&Treasury::account_id());

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), treasury_balance, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 0)
        });

        <Treasury as OnInitialize<u64>>::on_initialize(2);
        assert_eq!(Treasury::pot(), 100); // Pot hasn't changed

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), Treasury::pot(), 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 1)
        });

        <Treasury as OnInitialize<u64>>::on_initialize(4);
        assert_eq!(Treasury::pot(), 0); // Pot is emptied
        assert_eq!(Balances::free_balance(Treasury::account_id()), 1); // but the account is still there
    });
}

// In case treasury account is not existing then it works fine.
// This is useful for chain that will just update runtime.
#[test]
fn inexistent_account_works() {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(0, 100), (1, 99), (2, 1)],
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();
    // Treasury genesis config is not build thus treasury account does not exist
    let mut t: sp_io::TestExternalities = t.into();

    t.execute_with(|| {
        assert_eq!(Balances::free_balance(Treasury::account_id()), 0); // Account does not exist
        assert_eq!(Treasury::pot(), 0); // Pot is empty

        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 99, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 0)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 1, 3)
        });
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 1)
        });
        <Treasury as OnInitialize<u64>>::on_initialize(2);
        assert_eq!(Treasury::pot(), 0); // Pot hasn't changed
        assert_eq!(Balances::free_balance(3), 0); // Balance of `3` hasn't changed

        Balances::make_free_balance_be(&Treasury::account_id(), 100);
        assert_eq!(Treasury::pot(), 99); // Pot now contains funds
        assert_eq!(Balances::free_balance(Treasury::account_id()), 100); // Account does exist

        <Treasury as OnInitialize<u64>>::on_initialize(4);

        assert_eq!(Treasury::pot(), 0); // Pot has changed
        assert_eq!(Balances::free_balance(3), 99); // Balance of `3` has changed
    });
}

#[test]
fn genesis_funding_works() {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let initial_funding = 100;
    pallet_balances::GenesisConfig::<Test> {
        // Total issuance will be 200 with treasury account initialized with 100.
        balances: vec![(0, 100), (Treasury::account_id(), initial_funding)],
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();
    crate::GenesisConfig::<Test>::default()
        .assimilate_storage(&mut t)
        .unwrap();
    let mut t: sp_io::TestExternalities = t.into();

    t.execute_with(|| {
        assert_eq!(
            Balances::free_balance(Treasury::account_id()),
            initial_funding
        );
        assert_eq!(
            Treasury::pot(),
            initial_funding - Balances::minimum_balance()
        );
    });
}

#[test]
fn max_approvals_limited() {
    ExtBuilder::default().build().execute_with(|| {
        Balances::make_free_balance_be(&Treasury::account_id(), u64::MAX);
        Balances::make_free_balance_be(&0, u64::MAX);

        for _ in 0..<Test as Config>::MaxApprovals::get() {
            assert_ok!({
                #[allow(deprecated)]
                Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
            });
            assert_ok!({
                #[allow(deprecated)]
                Treasury::approve_proposal(RuntimeOrigin::root(), 0)
            });
        }

        // One too many will fail
        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 100, 3)
        });
        assert_noop!(
            {
                #[allow(deprecated)]
                Treasury::approve_proposal(RuntimeOrigin::root(), 0)
            },
            Error::<Test, _>::TooManyApprovals
        );
    });
}

#[test]
fn try_state_proposals_invariant_1_works() {
    ExtBuilder::default().build().execute_with(|| {
        use frame_support::pallet_prelude::DispatchError::Other;
        // Add a proposal using `propose_spend`
        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 1, 3)
        });
        assert_eq!(Proposals::<Test>::iter().count(), 1);
        assert_eq!(ProposalCount::<Test>::get(), 1);
        // Check invariant 1 holds
        assert!(ProposalCount::<Test>::get() as usize >= Proposals::<Test>::iter().count());
        // Break invariant 1 by decreasing `ProposalCount`
        ProposalCount::<Test>::put(0);
        // Invariant 1 should be violated
        assert_eq!(
            Treasury::do_try_state(),
            Err(Other("Actual number of proposals exceeds `ProposalCount`."))
        );
    });
}

#[test]
fn try_state_proposals_invariant_2_works() {
    ExtBuilder::default().build().execute_with(|| {
		use frame_support::pallet_prelude::DispatchError::Other;
		// Add a proposal using `propose_spend`
		assert_ok!({
			#[allow(deprecated)]
			Treasury::propose_spend(RuntimeOrigin::signed(0), 1, 3)
		});
		assert_eq!(Proposals::<Test>::iter().count(), 1);
		let current_proposal_count = ProposalCount::<Test>::get();
		assert_eq!(current_proposal_count, 1);
		// Check invariant 2 holds
		assert!(
			Proposals::<Test>::iter_keys()
			.all(|proposal_index| {
					proposal_index < current_proposal_count
			})
		);
		// Break invariant 2 by inserting the proposal under key = 1
		let proposal = Proposals::<Test>::take(0).unwrap();
		Proposals::<Test>::insert(1, proposal);
		// Invariant 2 should be violated
		assert_eq!(
			Treasury::do_try_state(),
			Err(Other("`ProposalCount` should by strictly greater than any ProposalIndex used as a key for `Proposals`."))
		);
	});
}

#[test]
fn try_state_proposals_invariant_3_works() {
    ExtBuilder::default().build().execute_with(|| {
        use frame_support::pallet_prelude::DispatchError::Other;
        // Add a proposal using `propose_spend`
        assert_ok!({
            #[allow(deprecated)]
            Treasury::propose_spend(RuntimeOrigin::signed(0), 10, 3)
        });
        assert_eq!(Proposals::<Test>::iter().count(), 1);
        // Approve the proposal
        assert_ok!({
            #[allow(deprecated)]
            Treasury::approve_proposal(RuntimeOrigin::root(), 0)
        });
        assert_eq!(Approvals::<Test>::get().len(), 1);
        // Check invariant 3 holds
        assert!(Approvals::<Test>::get()
            .iter()
            .all(|proposal_index| { Proposals::<Test>::contains_key(proposal_index) }));
        // Break invariant 3 by adding another key to `Approvals`
        let mut approvals_modified = Approvals::<Test>::get();
        approvals_modified.try_push(2).unwrap();
        Approvals::<Test>::put(approvals_modified);
        // Invariant 3 should be violated
        assert_eq!(
            Treasury::do_try_state(),
            Err(Other(
                "Proposal indices in `Approvals` must also be contained in `Proposals`."
            ))
        );
    });
}
