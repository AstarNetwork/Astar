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
use frame_support::traits::Get;
use frame_support::traits::StorePreimage;
use frame_support::{assert_ok, traits::Currency};
use frame_system::RawOrigin;
use pallet_democracy::{
    AccountVote, BoundedCallOf, CallOf, Conviction, Pallet as Democracy, ReferendumCount,
    ReferendumInfoOf, Vote, VoteThreshold,
};
use parity_scale_codec::Encode;
use sp_runtime::traits::Bounded;

const SEED: u32 = 0;
type BalanceOf<T> = <<T as pallet_democracy::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
    let caller: T::AccountId = account(name, index, SEED);
    T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
    caller
}

fn make_proposal<T: Config>(n: u32) -> BoundedCallOf<T> {
    let call: CallOf<T> = frame_system::Call::remark { remark: n.encode() }.into();
    T::Preimages::bound(call).unwrap()
}

fn add_referendum<T: Config>(n: u32, caller: T::AccountId) -> Result<(), &'static str> {
    let proposal = make_proposal::<T>(n);
    assert_ok!(Democracy::<T>::propose(
        RawOrigin::Signed(caller.clone()).into(),
        proposal,
        T::MinimumDeposit::get(),
    ));

    let end = frame_system::Pallet::<T>::block_number() + T::VotingPeriod::get();
    let proposal = make_proposal::<T>(n);
    let threshold = VoteThreshold::SuperMajorityApprove;

    let item = pallet_democracy::ReferendumInfo::Ongoing(pallet_democracy::ReferendumStatus {
        end,
        proposal,
        threshold,
        delay: T::EnactmentPeriod::get(),
        tally: pallet_democracy::Tally::default(),
    });

    ReferendumInfoOf::<T>::insert(n, item);
    ReferendumCount::<T>::put(n + 1);

    Ok(())
}

fn account_vote<T: Config>(b: BalanceOf<T>) -> AccountVote<BalanceOf<T>> {
    let v = Vote {
        aye: true,
        conviction: Conviction::Locked1x,
    };

    AccountVote::Standard {
        vote: v,
        balance: b,
    }
}

#[benchmarks]
mod benches {
    use super::*;

    #[benchmark]
    fn migration_referendum_info<T: Config>() -> Result<(), BenchmarkError> {
        let caller = funded_account::<T>("caller", 0);

        let account_vote = account_vote::<T>(100u32.into());

        add_referendum::<T>(0, caller.clone())?;

        // We need to create existing direct votes
        for i in 0..3 {
            add_referendum::<T>(i, caller.clone())?;
        }

        assert_eq!(
            ReferendumCount::<T>::get(),
            3 as u32,
            "Proposals not created."
        );

        #[block]
        {
            crate::DemocracyMigrationV1ToV2::<T, crate::weights::SubstrateWeight<T>>::migrate_referendum_info(None, 3u32);
        }

        Ok(())
    }

    #[benchmark]
    fn migration_voting_of<T: Config>() -> Result<(), BenchmarkError> {
        #[block]
        {
            crate::DemocracyMigrationV1ToV2::<T, crate::weights::SubstrateWeight<T>>::migrate_voting_of(None, 3u32);
        }

        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
