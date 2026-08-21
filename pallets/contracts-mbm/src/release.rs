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

//! Settles the balances `pallet-contracts` still owns, so that its pallet index can be retired.

use frame_support::{
    migrations::{SteppedMigration, SteppedMigrationError},
    traits::{
        fungible::MutateHold,
        tokens::{Fortitude, Precision, Restriction},
        Get,
    },
    weights::WeightMeter,
};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;

use crate::{WeightInfo, LOG_TARGET};

type HoldReasonOf<T> = <T as pallet_balances::Config>::RuntimeHoldReason;
type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

/// Moves every balance hold placed by `pallet-contracts` to an escrow account.
///
/// # Why this has to happen *before* the pallet is removed
///
/// Storage and code deposits are live `Balances::Holds` entries whose reason is
/// `RuntimeHoldReason::Contracts(..)`, i.e. tagged with the pallet's index. Dropping the pallet
/// from `construct_runtime` deletes that variant and every such entry becomes undecodable.
/// `Holds` is a `ValueQuery` map, so the decode failure silently yields an empty vector: the hold
/// record disappears while `AccountData::reserved` stays behind. Nothing can decrement that
/// `reserved` again, so the funds are frozen for good - and silently, because
/// `pallet_balances::do_try_state` only bounds the length of each hold vector, it never checks
/// that the holds sum to `reserved`. Hence the `post_upgrade` check below.
///
/// # Why everything goes to escrow
///
/// Storage deposits sit on keyless, derived contract accounts, so releasing them in place would
/// strand the funds; the depositor is not recorded on chain. Code upload deposits do sit on real
/// accounts, but sweeping both to a single escrow account keeps this migration to one code path
/// and leaves the refunding to a plain, auditable batch of transfers afterwards.
///
/// # What this deliberately does not touch
///
/// The *free* balance of contract accounts is contract owned value, not a deposit - for a vault
/// or DEX contract it is user funds whose rightful owner is encoded in the contract's own child
/// trie, not "whoever deployed it". Sweeping it is a policy decision, not a technical necessity:
/// unlike the holds it breaks no invariant if left in place, and `Balances::force_transfer` from
/// Root can still move it at any point in the future. It is deliberately left for the migration
/// that purges `ContractInfoOf`, which is the last on-chain record of which accounts are
/// contracts.
pub struct ReleaseContractsDeposits<T, CodeDeposit, StorageDeposit, Escrow, W>(
    PhantomData<(T, CodeDeposit, StorageDeposit, Escrow, W)>,
);

impl<T, CodeDeposit, StorageDeposit, Escrow, W> SteppedMigration
    for ReleaseContractsDeposits<T, CodeDeposit, StorageDeposit, Escrow, W>
where
    T: pallet_balances::Config,
    HoldReasonOf<T>: PartialEq,
    CodeDeposit: Get<HoldReasonOf<T>>,
    StorageDeposit: Get<HoldReasonOf<T>>,
    Escrow: Get<T::AccountId>,
    W: WeightInfo,
{
    /// Last account whose holds were settled.
    type Cursor = T::AccountId;
    type Identifier = [u8; 32];

    fn id() -> Self::Identifier {
        blake2_256(b"contracts-mbm::ReleaseContractsDeposits")
    }

    fn step(
        cursor: Option<Self::Cursor>,
        meter: &mut WeightMeter,
    ) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
        let required = W::release_deposit();
        // Guarantees the loop below settles at least one account, so a step never returns the
        // cursor it was handed and stalls the migration forever.
        if meter.remaining().any_lt(required) {
            return Err(SteppedMigrationError::InsufficientWeight { required });
        }

        let reasons = [CodeDeposit::get(), StorageDeposit::get()];
        let escrow = Escrow::get();
        // Removing the entry of the account currently being visited is safe: the iterator has
        // already recorded its key to seek from.
        let mut iter = match &cursor {
            Some(last) => pallet_balances::Holds::<T>::iter_from(
                pallet_balances::Holds::<T>::hashed_key_for(last),
            ),
            None => pallet_balances::Holds::<T>::iter(),
        };
        let mut last = cursor;
        let mut settled = 0u32;

        loop {
            if !meter.can_consume(required) {
                return Ok(last);
            }

            let Some((account, holds)) = iter.next() else {
                log::info!(
                    target: LOG_TARGET,
                    "ReleaseContractsDeposits: finished, {settled} accounts settled in this step 💸",
                );
                return Ok(None);
            };

            // Charged whether or not the account turns out to be relevant: the read happened
            // either way, and this keeps the loop guaranteed to make progress.
            meter.consume(required);
            settled = settled.saturating_add(1);
            last = Some(account.clone());

            for hold in holds.iter() {
                if hold.amount.is_zero() || !reasons.contains(&hold.id) {
                    continue;
                }
                Self::sweep_to_escrow(&account, &escrow, hold.amount, &hold.id);
            }
        }
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: sp_std::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        let reasons = [CodeDeposit::get(), StorageDeposit::get()];

        for (_, holds) in pallet_balances::Holds::<T>::iter() {
            if holds.iter().any(|hold| reasons.contains(&hold.id)) {
                return Err(
                    "A pallet-contracts hold survived; removing the pallet index would \
                            freeze these funds permanently 🚨"
                        .into(),
                );
            }
        }

        Ok(())
    }
}

impl<T, CodeDeposit, StorageDeposit, Escrow, W>
    ReleaseContractsDeposits<T, CodeDeposit, StorageDeposit, Escrow, W>
where
    T: pallet_balances::Config,
{
    fn sweep_to_escrow(
        account: &T::AccountId,
        escrow: &T::AccountId,
        amount: BalanceOf<T>,
        reason: &HoldReasonOf<T>,
    ) {
        match pallet_balances::Pallet::<T>::transfer_on_hold(
            reason,
            account,
            escrow,
            amount,
            Precision::BestEffort,
            Restriction::Free,
            // The source is typically a keyless contract account; the existential deposit must
            // not stand in the way of recovering the funds.
            Fortitude::Force,
        ) {
            Ok(moved) => log::info!(
                target: LOG_TARGET,
                "ReleaseContractsDeposits: swept {moved:?} from {account:?} to the escrow",
            ),
            // Failing the whole migration would leave the remaining holds in place, which is
            // exactly the outcome this is meant to avoid. `post_upgrade` catches any leftover.
            Err(err) => log::error!(
                target: LOG_TARGET,
                "ReleaseContractsDeposits: failed to sweep {amount:?} from {account:?}: {err:?}",
            ),
        }
    }
}
