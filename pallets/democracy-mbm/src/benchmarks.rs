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

#![cfg(feature = "runtime-benchmarks")]

extern crate alloc;

use crate::{Config, Pallet};
use frame_benchmarking::v2::*;
use frame_support::traits::{Currency, Get, StorePreimage};
use frame_system::RawOrigin;
use pallet_democracy::{
    AccountVote, BoundedCallOf, CallOf, Conviction, Pallet as Democracy, ReferendumCount,
    ReferendumIndex, ReferendumInfo, ReferendumInfoOf, Vote, VoteThreshold, Voting, VotingOf,
};
use parity_scale_codec::Encode;
use sp_runtime::{traits::Bounded, SaturatedConversion};

const SEED: u32 = 0;
type BalanceOf<T> = <<T as pallet_democracy::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;
type AccountId<T> = <T as frame_system::Config>::AccountId;
type BlockNumber<T> = <<<T as frame_system::Config>::Block as sp_runtime::traits::Block>::Header as sp_runtime::traits::Header>::Number;
type MaxVotes<T> = <T as pallet_democracy::Config>::MaxVotes;
type VotingType<T> = Voting<BalanceOf<T>, AccountId<T>, BlockNumber<T>, MaxVotes<T>>;

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
    caller
}

fn inject_prior_lock<T: Config>(
    caller: &T::AccountId,
    voting: VotingType<T>,
) -> Result<(), &'static str> {
    let mut new_voting = voting;

    if let Voting::Direct { ref mut prior, .. } = new_voting {
        // 1. Clear lock
        prior.rejig(u32::MAX.into());

        // 2. Create lock with a future block and a non-zero amount
        let block_number = frame_system::Pallet::<T>::block_number()
            .saturated_into::<u32>()
            .saturating_add(2)
            .into();
        let lock_amount = 100u32.into();
        prior.accumulate(block_number, lock_amount);
    }

    VotingOf::<T>::insert(&caller, new_voting);
    Ok(())
}

fn add_referendum<T: Config>(n: u32) -> Result<ReferendumIndex, &'static str> {
    let end = frame_system::Pallet::<T>::block_number()
        .saturated_into::<u32>()
        .saturating_add(2)
        .into();
    let proposal = make_proposal::<T>(n);
    let threshold = VoteThreshold::SuperMajorityApprove;

    let item = pallet_democracy::ReferendumInfo::Ongoing(pallet_democracy::ReferendumStatus {
        end,
        proposal,
        threshold,
        delay: T::EnactmentPeriod::get(),
        tally: pallet_democracy::Tally::default(),
    });

    let ref_index = ReferendumCount::<T>::get();
    ReferendumInfoOf::<T>::insert(ref_index, item);
    ReferendumCount::<T>::put(n + 1);

    Ok(ref_index)
}

fn make_proposal<T: Config>(n: u32) -> BoundedCallOf<T> {
    let call: CallOf<T> = frame_system::Call::remark { remark: n.encode() }.into();
    T::Preimages::bound(call).unwrap()
}

fn account_vote<T: Config>(b: BalanceOf<T>) -> AccountVote<BalanceOf<T>> {
    let v = Vote {
        aye: true,
        conviction: Conviction::Locked6x,
    };

    AccountVote::Standard {
        vote: v,
        balance: b,
    }
}

fn unlock_block_number<T: Config>(voting: VotingType<T>) -> Option<u32> {
    let prior = match voting {
        Voting::Direct { ref prior, .. } => prior,
        _ => return None,
    };
    let encoded = prior.encode();
    // As the field block_number is private in PriorLock enum
    // it encodes the enum and decodes the 4 bytes (as it's an u32)
    Some(u32::from_le_bytes([
        encoded[0], encoded[1], encoded[2], encoded[3],
    ]))
}

#[benchmarks]
mod benches {
    use super::*;

    #[benchmark]
    fn migration_referendum_info<T: Config>() -> Result<(), BenchmarkError> {
        // Set block_number to 2 so it's not 0 by default
        frame_system::Pallet::<T>::set_block_number(2u32.into());
        let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

        // Create a referendum
        let ref_index = add_referendum::<T>(1)?;

        // Ensure the referendum end is 4
        assert!(matches!(
            ReferendumInfoOf::<T>::get(ref_index).unwrap(),
            ReferendumInfo::Ongoing(ref status) if status.end == 4u32.into()
        ));

        #[block]
        {
            crate::DemocracyMigrationV1ToV2::<T, crate::weights::SubstrateWeight<T>>::migrate_referendum_info(None, current_block);
        }

        // Ensure referendum end has been migrated
        assert!(matches!(
            ReferendumInfoOf::<T>::get(ref_index).unwrap(),
            ReferendumInfo::Ongoing(ref status) if status.end == 6u32.into()
        ));

        Ok(())
    }

    #[benchmark]
    fn migration_voting_of<T: Config>() -> Result<(), BenchmarkError> {
        // Set block number to 2 so it's not 0 by default
        frame_system::Pallet::<T>::set_block_number(2u32.into());
        let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u32>();

        // Create a referendum and vote on it to create a VotingOf for the account
        // only one vote for one referendum is needed
        let caller = funded_account::<T>("caller", 0);
        let account_vote = account_vote::<T>(100u32.into());
        let ref_index = add_referendum::<T>(1)?;
        Democracy::<T>::vote(
            RawOrigin::Signed(caller.clone()).into(),
            ref_index,
            account_vote,
        )?;

        // Ensure the vote is properly saved into storage
        let voting_before = VotingOf::<T>::get(&caller);
        let vote_before = match voting_before {
            Voting::Direct { ref votes, .. } => votes,
            _ => return Err("Votes are not direct".into()),
        };
        assert_eq!(vote_before.len(), 1 as usize, "Votes not created");

        // Add a prior lock for the caller
        inject_prior_lock::<T>(&caller, voting_before)?;

        // Ensure the lock duration is current_block (2) + 2 = 4
        let voting_before_migration = VotingOf::<T>::get(&caller);
        assert_eq!(
            unlock_block_number::<T>(voting_before_migration).unwrap(),
            4
        );

        #[block]
        {
            crate::DemocracyMigrationV1ToV2::<T, crate::weights::SubstrateWeight<T>>::migrate_voting_of(None, current_block);
        }

        // Ensure migration worked
        let voting_after_migration = VotingOf::<T>::get(&caller);
        assert_eq!(unlock_block_number::<T>(voting_after_migration).unwrap(), 6);

        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
