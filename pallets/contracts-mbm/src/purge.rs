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

//! Purges the storage of the removed `pallet-contracts`, spread over multiple blocks.

use frame_support::{
    migrations::{SteppedMigration, SteppedMigrationError},
    storage::unhashed,
    traits::{ConstU32, Get},
    weights::{Weight, WeightMeter},
    BoundedVec, ReversibleStorageHasher, Twox64Concat,
};
use parity_scale_codec::Decode;
use sp_io::hashing::{blake2_256, twox_128};
use sp_std::{marker::PhantomData, vec::Vec};

use crate::{WeightInfo, LOG_TARGET};

/// Upper bound on the length of a storage key handed around as a migration cursor.
///
/// The longest key any of these migrations sees is `twox_128(pallet) ++ twox_128(item) ++
/// hasher(key)`, i.e. 72 bytes for `ContractInfoOf`. Measured over every live key on both
/// mainnets; 512 leaves ~7x headroom.
pub type MaxKeyLen = ConstU32<512>;

/// Hard cap on the number of storage keys touched in a single step.
///
/// The weight meter is the real limit - every key costs at least a read and a write, so the meter
/// always binds first. This is only a guard against an unbounded loop should that ever stop being
/// true.
pub const MAX_KEYS_PER_STEP: u32 = 10_000;

/// `pallet-contracts` storage holding the `trie_id`s of contracts pending lazy deletion.
pub const DELETION_QUEUE: &[u8] = b"DeletionQueue";
/// `pallet-contracts` storage holding the info (starting with the `trie_id`) of live contracts.
pub const CONTRACT_INFO_OF: &[u8] = b"ContractInfoOf";

/// `pallet-contracts`' `TrieId`, i.e. `BoundedVec<u8, ConstU32<128>>`.
pub type TrieId = BoundedVec<u8, ConstU32<128>>;

/// Exact length of a `pallet-contracts` `trie_id`.
///
/// `ContractInfo::new` derives it from `blake2_256`, so every trie id is 32 bytes even though the
/// type allows up to 128. Verified against all 305 Astar and 923 Shiden `ContractInfoOf` entries.
/// Enforcing it turns a mis-parse (e.g. an upstream field reordering) into a loud, logged skip
/// rather than a silent wrong-offset deletion.
pub const TRIE_ID_LEN: usize = 32;

/// Length of `twox_128(pallet) ++ twox_128(item)`.
const MAP_PREFIX_LEN: usize = 32;

/// Wraps a storage key into a migration cursor.
fn to_cursor(key: Vec<u8>) -> Result<BoundedVec<u8, MaxKeyLen>, SteppedMigrationError> {
    BoundedVec::try_from(key).map_err(|_| {
        log::error!(
            target: LOG_TARGET,
            "Encountered a storage key longer than {} bytes, cannot resume 🚨",
            <MaxKeyLen as Get<u32>>::get(),
        );
        SteppedMigrationError::Failed
    })
}

/// Next storage key strictly after `from`, as long as it still lives under `prefix`.
///
/// Seeding `from` with the bare `prefix` yields the first key of that prefix, because `next_key`
/// is strictly-greater and every real key is strictly longer than the prefix it lives under.
fn next_key_under(from: &[u8], prefix: &[u8]) -> Option<Vec<u8>> {
    sp_io::storage::next_key(from).filter(|key| key.starts_with(prefix))
}

/// Translates "ran out of weight" into the right [`SteppedMigrationError`].
///
/// `SteppedMigration::transactional_step` rolls back on `Err`, so `Err` is only correct when this
/// step has not written anything yet. Conversely, a step that returns `Ok` without having removed
/// anything would spin forever, and multi block migrations block all extrinsics while they run.
fn not_enough_weight(
    removed_so_far: u32,
    required: Weight,
) -> Result<Option<()>, SteppedMigrationError> {
    if removed_so_far == 0 {
        Err(SteppedMigrationError::InsufficientWeight { required })
    } else {
        Ok(Some(()))
    }
}

/// Percentage of `b` that `a` represents, saturating and zero-safe.
fn pct(a: u64, b: u64) -> u64 {
    if b == 0 {
        0
    } else {
        a.saturating_mul(100) / b
    }
}

/// Renders a meter's whole per-block budget. Only formatted if the log level is enabled.
struct Budget<'a>(&'a WeightMeter);

impl core::fmt::Display for Budget<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let limit = self.0.limit();
        write!(
            f,
            "{} ref_time / {} proof",
            limit.ref_time(),
            limit.proof_size()
        )
    }
}

/// Renders how much of the per-block budget a step has spent.
///
/// The proof percentage is the number to watch on a dry run: it is what the placeholder weights in
/// `weights.rs` are guessing at, and what keeps a block from going PoV-oversized.
struct Spent<'a>(&'a WeightMeter);

impl core::fmt::Display for Spent<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let (used, limit) = (self.0.consumed(), self.0.limit());
        write!(
            f,
            "spent {}/{} ref_time ({}%), {}/{} proof ({}%)",
            used.ref_time(),
            limit.ref_time(),
            pct(used.ref_time(), limit.ref_time()),
            used.proof_size(),
            limit.proof_size(),
            pct(used.proof_size(), limit.proof_size()),
        )
    }
}

/// Human readable name of a `pallet-contracts` storage item, for logging.
fn source_name(item: &[u8]) -> &str {
    sp_std::str::from_utf8(item).unwrap_or("<non-utf8>")
}

/// Per-step tally, logged on every exit path so a dry run can be followed block by block.
#[derive(Default)]
struct StepStats {
    /// Top level `DeletionQueue` / `ContractInfoOf` entries dropped.
    entries: u32,
    /// Child trie keys removed.
    child_keys: u32,
    /// Consumer references handed back to contract accounts.
    consumers: u32,
}

impl StepStats {
    /// Every key this step actually removed - what the budget and the rollback rule count.
    fn removed(&self) -> u32 {
        self.entries.saturating_add(self.child_keys)
    }
}

impl core::fmt::Display for StepStats {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "{} entries, {} child keys, {} consumer refs returned",
            self.entries, self.child_keys, self.consumers
        )
    }
}

/// Outcome of a bounded removal loop.
pub(crate) enum Progress {
    /// Nothing is left to remove.
    Finished,
    /// The loop stopped early. `required` carries the weight of the key that could not be paid
    /// for, and is `None` when the loop stopped because it hit its key budget instead.
    Exhausted { required: Option<Weight> },
}

/// Multi block variant of `frame_support::migrations::RemovePallet`.
///
/// Removes every storage key under a pallet's prefix, spread over as many blocks as
/// `pallet-migrations` needs. `RemovePallet` itself is unusable here: it wipes the whole prefix
/// within a single block.
///
/// The cursor carries the last removed key so that iteration always resumes where it left off.
/// Restarting from the bare prefix instead would be quadratic: `next_key` has to walk over every
/// deleted-but-not-yet-committed key sitting in the block's storage overlay.
pub struct RemovePalletStepped<P, W>(PhantomData<(P, W)>);

impl<P: Get<&'static str>, W> RemovePalletStepped<P, W> {
    /// Hashed storage prefix of the pallet that is being removed.
    fn hashed_prefix() -> [u8; 16] {
        twox_128(P::get().as_bytes())
    }
}

impl<P, W> SteppedMigration for RemovePalletStepped<P, W>
where
    P: Get<&'static str>,
    W: WeightInfo,
{
    /// Last removed key. `None` means "start from the beginning of the prefix".
    type Cursor = BoundedVec<u8, MaxKeyLen>;
    /// Derived from the pallet name so that two instances never collide.
    type Identifier = [u8; 32];

    fn id() -> Self::Identifier {
        blake2_256(
            &[
                b"contracts-mbm::RemovePalletStepped::".as_slice(),
                P::get().as_bytes(),
            ]
            .concat(),
        )
    }

    fn step(
        cursor: Option<Self::Cursor>,
        meter: &mut WeightMeter,
    ) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
        let prefix = Self::hashed_prefix();
        // The only bit the cursor carries: `None` means this is the first step of the migration.
        if cursor.is_none() {
            log::info!(
                target: LOG_TARGET,
                "remove<{}>: starting, budget {}",
                P::get(),
                Budget(meter),
            );
        }
        let mut from = cursor
            .map(BoundedVec::into_inner)
            .unwrap_or_else(|| prefix.to_vec());
        let mut removed = 0u32;
        let mut bytes = 0u64;

        loop {
            let Some(key) = next_key_under(&from, &prefix) else {
                log::info!(
                    target: LOG_TARGET,
                    "remove<{}>: DONE ✅ prefix empty | this step: {removed} keys, {bytes} value \
                     bytes | {}",
                    P::get(),
                    Spent(meter),
                );
                return Ok(None);
            };

            if removed >= MAX_KEYS_PER_STEP {
                log::info!(
                    target: LOG_TARGET,
                    "remove<{}>: paused on the {MAX_KEYS_PER_STEP} key/step cap | this step: \
                     {removed} keys, {bytes} value bytes | {}",
                    P::get(),
                    Spent(meter),
                );
                return to_cursor(from).map(Some);
            }

            // Measure the value without copying it into the runtime; it enters the PoV either way.
            let value_len = sp_io::storage::read(&key, &mut [], 0).unwrap_or_default();
            let cost = W::remove_key(value_len);

            if meter.try_consume(cost).is_err() {
                return if removed == 0 {
                    log::warn!(
                        target: LOG_TARGET,
                        "remove<{}>: cannot afford a single key - needs {} ref_time / {} proof but \
                         the whole block budget is {} 🚨",
                        P::get(),
                        cost.ref_time(),
                        cost.proof_size(),
                        Budget(meter),
                    );
                    Err(SteppedMigrationError::InsufficientWeight { required: cost })
                } else {
                    log::info!(
                        target: LOG_TARGET,
                        "remove<{}>: out of weight, resuming next block | this step: {removed} \
                         keys, {bytes} value bytes | {}",
                        P::get(),
                        Spent(meter),
                    );
                    to_cursor(from).map(Some)
                };
            }

            sp_io::storage::clear(&key);
            removed = removed.saturating_add(1);
            bytes = bytes.saturating_add(value_len as u64);
            from = key;
        }
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
        let prefix = Self::hashed_prefix();
        let (mut keys, mut bytes) = (0u32, 0u64);
        let mut from = prefix.to_vec();
        while let Some(key) = next_key_under(&from, &prefix) {
            keys = keys.saturating_add(1);
            bytes = bytes
                .saturating_add(sp_io::storage::read(&key, &mut [], 0).unwrap_or_default() as u64);
            from = key;
        }

        log::info!(
            target: LOG_TARGET,
            "remove<{}>: pre_upgrade sees {keys} keys holding {bytes} value bytes 👀",
            P::get(),
        );
        Ok(Vec::new())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        use frame_support::storage::unhashed::contains_prefixed_key;

        if contains_prefixed_key(&Self::hashed_prefix()) {
            return Err("Keys remaining post-removal, this should never happen 🚨".into());
        }

        log::info!(
            target: LOG_TARGET,
            "remove<{}>: post_upgrade OK, prefix is empty ✅",
            P::get(),
        );
        Ok(())
    }
}

/// Purges the child tries owned by the decommissioned `pallet-contracts`, and hands back the
/// consumer references it took on contract accounts.
///
/// `pallet-contracts` keeps the storage of every instantiated contract in a *child trie* addressed
/// by that contract's `trie_id`, and holds only a pointer to it in its own (top level) storage.
/// Wiping the pallet prefix therefore does **not** free the bulk of the data, it merely orphans it:
/// the `trie_id`s needed to address those child tries would be gone for good.
///
/// This migration walks the two places where `trie_id`s are recorded and empties the corresponding
/// child tries before dropping the top level entry pointing at them:
/// * `Contracts::DeletionQueue` - tries of already terminated contracts, awaiting lazy deletion.
/// * `Contracts::ContractInfoOf` - tries of contracts that are still alive.
///
/// It MUST be scheduled *before* [`RemovePalletStepped`] for the same pallet, otherwise the
/// `trie_id`s are deleted first and the child tries become unreachable forever.
///
/// # Consumer references
///
/// `pallet-contracts` calls `inc_consumers` on the contract account at instantiation and only
/// gives that reference back from `seal_terminate`. `ContractInfoOf` is the last on-chain record
/// of which accounts are contracts, so purging it is the final opportunity to hand the reference
/// back - leave it and every live contract account keeps a dangling consumer forever, and can
/// never be reaped even once its balance is gone. `DeletionQueue` entries are terminated
/// contracts, which already gave theirs back, hence the distinction below.
///
/// # Storage layout coupling
///
/// Values are decoded structurally rather than via `pallet-contracts` types, since the pallet is
/// no longer a dependency of the runtimes. Verified against `polkadot-sdk` `stable2512`:
/// * `DeletionQueue: StorageMap<_, Twox64Concat, u32, TrieId>` - the value *is* a `TrieId`.
/// * `ContractInfoOf: StorageMap<_, Twox64Concat, AccountId, ContractInfo<T>>`, and
///   `ContractInfo`'s **first** field is `pub trie_id: TrieId`, so it decodes off the front.
///
/// A `TrieId` is a length prefixed byte blob; decoding it as a [`TrieId`] (rather than a plain
/// `Vec<u8>`) makes a garbage value fail loudly instead of producing a bogus multi kilobyte id.
pub struct PurgeContractsChildTries<T, P, W>(PhantomData<(T, P, W)>);

impl<T: frame_system::Config, P: Get<&'static str>, W> PurgeContractsChildTries<T, P, W> {
    /// Hashed prefix of a `pallet-contracts` storage map.
    pub(crate) fn map_prefix(item: &[u8]) -> Vec<u8> {
        [twox_128(P::get().as_bytes()), twox_128(item)].concat()
    }

    /// The storage maps holding `trie_id`s, paired with whether their key is a live contract
    /// account still owing a consumer reference. Terminated contracts come first, they are dead
    /// weight.
    fn trie_id_sources() -> [(&'static [u8], bool); 2] {
        [(DELETION_QUEUE, false), (CONTRACT_INFO_OF, true)]
    }

    /// Gives back the consumer reference `pallet-contracts` took on the contract account keyed by
    /// `ContractInfoOf` key `key`.
    pub(crate) fn release_consumer(key: &[u8]) -> bool {
        let account = key
            .get(MAP_PREFIX_LEN..)
            .map(Twox64Concat::reverse)
            .and_then(|mut raw| T::AccountId::decode(&mut raw).ok());

        let Some(account) = account else {
            log::warn!(
                target: LOG_TARGET,
                "purge: no account id in a ContractInfoOf key, consumer ref kept 🚨",
            );
            return false;
        };

        // Guarded rather than unconditional: `dec_consumers` on an account that has none is a
        // logged logic error, and it would write back a default `AccountInfo` for an account that
        // no longer exists.
        let before = frame_system::Pallet::<T>::consumers(&account);
        if before == 0 {
            log::debug!(
                target: LOG_TARGET,
                "purge: {account:?} is already at 0 consumers, nothing to hand back",
            );
            return false;
        }

        frame_system::Pallet::<T>::dec_consumers(&account);
        log::debug!(
            target: LOG_TARGET,
            "purge: {account:?} consumers {before} -> {}",
            before.saturating_sub(1),
        );
        true
    }
}

impl<T, P, W> SteppedMigration for PurgeContractsChildTries<T, P, W>
where
    T: frame_system::Config,
    P: Get<&'static str>,
    W: WeightInfo,
{
    /// Progress lives in storage itself: every step restarts from the first remaining entry, and
    /// an entry is only dropped once its child trie has been fully emptied. Restarting is cheap
    /// because the number of contracts is small - the child tries themselves, which are not, are
    /// walked with a local cursor.
    type Cursor = ();
    /// Derived from the pallet name so that two instances never collide.
    type Identifier = [u8; 32];

    fn id() -> Self::Identifier {
        blake2_256(
            &[
                b"contracts-mbm::PurgeContractsChildTries::".as_slice(),
                P::get().as_bytes(),
            ]
            .concat(),
        )
    }

    fn step(
        cursor: Option<Self::Cursor>,
        meter: &mut WeightMeter,
    ) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
        // The only bit the cursor carries: `None` means this is the first step of the migration.
        // Progress itself lives in storage - an entry is dropped only once its child trie is
        // empty, so a restart resumes at the first entry that is still there.
        if cursor.is_none() {
            log::info!(
                target: LOG_TARGET,
                "purge: starting, budget {}",
                Budget(meter),
            );
        }

        // Counts every key this step actually removed, top level entries and child trie keys
        // alike. Used to decide between `Err(InsufficientWeight)` (rolls back, so only valid when
        // nothing was written) and `Ok(Some(()))` (commits and resumes).
        let mut stats = StepStats::default();

        for (item, live) in Self::trie_id_sources() {
            let name = source_name(item);
            let prefix = Self::map_prefix(item);
            let mut from = prefix.clone();
            let entries_before = stats.entries;

            while let Some(key) = next_key_under(&from, &prefix) {
                if stats.removed() >= MAX_KEYS_PER_STEP {
                    log::info!(
                        target: LOG_TARGET,
                        "purge: paused on the {MAX_KEYS_PER_STEP} key/step cap in {name} | this \
                         step: {stats} | {}",
                        Spent(meter),
                    );
                    return Ok(Some(()));
                }

                // The whole value must be pulled into the PoV to get at the `trie_id` anyway.
                let raw = unhashed::get_raw(&key);
                let mut cost =
                    W::remove_key(raw.as_ref().map(|v| v.len() as u32).unwrap_or_default());
                if live {
                    cost = cost.saturating_add(W::release_contract_consumer());
                }
                if meter.try_consume(cost).is_err() {
                    log::info!(
                        target: LOG_TARGET,
                        "purge: out of weight before a {name} entry | this step: {stats} | {}",
                        Spent(meter),
                    );
                    return not_enough_weight(stats.removed(), cost);
                }

                let maybe_trie_id = raw
                    .and_then(|raw| TrieId::decode(&mut &raw[..]).ok())
                    .filter(|trie_id| trie_id.len() == TRIE_ID_LEN);

                if let Some(trie_id) = maybe_trie_id {
                    let budget = MAX_KEYS_PER_STEP.saturating_sub(stats.removed());
                    let (progress, child_keys_removed) =
                        clear_child_trie_metered::<W>(&trie_id, meter, budget);
                    stats.child_keys = stats.child_keys.saturating_add(child_keys_removed);

                    if let Progress::Exhausted { required } = progress {
                        // Keep the top level entry so the next step picks the same trie up again.
                        log::info!(
                            target: LOG_TARGET,
                            "purge: child trie only partly emptied ({child_keys_removed} keys \
                             this step), its {name} entry is kept and will be resumed | this \
                             step: {stats} | {}",
                            Spent(meter),
                        );
                        return match required {
                            Some(cost) => not_enough_weight(stats.removed(), cost),
                            None => Ok(Some(())),
                        };
                    }

                    log::debug!(
                        target: LOG_TARGET,
                        "purge: {name} entry cleared, {child_keys_removed} child trie keys removed",
                    );
                } else {
                    // Either the value is too short, or what sits at the front is not a 32 byte
                    // blob - in both cases there is nothing addressable behind it. Dropping the
                    // entry is safe; the loud warning is what matters, because a systematic
                    // occurrence would mean the assumed `ContractInfo` layout is wrong.
                    log::warn!(
                        target: LOG_TARGET,
                        "purge: no {TRIE_ID_LEN}-byte trie id at the front of a {name} entry, \
                         dropping it without touching any child trie 🚨",
                    );
                }

                if live && Self::release_consumer(&key) {
                    stats.consumers = stats.consumers.saturating_add(1);
                }
                unhashed::kill(&key);
                stats.entries = stats.entries.saturating_add(1);
                from = key;
            }

            // Only worth saying when this step is the one that finished the map off; the scan
            // restarts from the top every step, so an already-empty map falls through here too.
            if stats.entries > entries_before {
                log::info!(
                    target: LOG_TARGET,
                    "purge: {name} is now empty, {} entries dropped this step",
                    stats.entries.saturating_sub(entries_before),
                );
            }
        }

        log::info!(
            target: LOG_TARGET,
            "purge: DONE ✅ every contract child trie purged | this step: {stats} | {}",
            Spent(meter),
        );
        Ok(None)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
        use parity_scale_codec::Encode;

        // Snapshot every trie id so `post_upgrade` can prove the child tries are really gone -
        // once the top level entries are removed they would be unreachable and unverifiable.
        let mut trie_ids: Vec<Vec<u8>> = Vec::new();
        let mut consumers_owed = 0u32;

        for (item, live) in Self::trie_id_sources() {
            let name = source_name(item);
            let prefix = Self::map_prefix(item);
            let mut from = prefix.clone();
            let (mut entries, mut undecodable) = (0u32, 0u32);

            while let Some(key) = next_key_under(&from, &prefix) {
                entries = entries.saturating_add(1);
                match unhashed::get_raw(&key)
                    .and_then(|raw| TrieId::decode(&mut &raw[..]).ok())
                    .filter(|trie_id| trie_id.len() == TRIE_ID_LEN)
                {
                    Some(trie_id) => trie_ids.push(trie_id.into_inner()),
                    None => undecodable = undecodable.saturating_add(1),
                }

                if live {
                    // How many accounts will actually get a reference back. Anything short of
                    // `entries` means some contract accounts are not carrying the ref this
                    // migration assumes - worth knowing before enactment, not after.
                    let has_ref = key
                        .get(MAP_PREFIX_LEN..)
                        .map(Twox64Concat::reverse)
                        .and_then(|mut raw| T::AccountId::decode(&mut raw).ok())
                        .is_some_and(|who| frame_system::Pallet::<T>::consumers(&who) > 0);
                    if has_ref {
                        consumers_owed = consumers_owed.saturating_add(1);
                    }
                }
                from = key;
            }

            log::info!(
                target: LOG_TARGET,
                "purge: pre_upgrade sees {entries} {name} entries ({undecodable} without a usable \
                 trie id) 👀",
            );
        }

        log::info!(
            target: LOG_TARGET,
            "purge: pre_upgrade snapshotted {} child tries to empty, and {consumers_owed} contract \
             accounts are owed a consumer ref 👀",
            trie_ids.len(),
        );

        Ok(trie_ids.encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
        use frame_support::storage::unhashed::contains_prefixed_key;

        for (item, _) in Self::trie_id_sources() {
            if contains_prefixed_key(&Self::map_prefix(item)) {
                return Err("Contract trie id entries remaining post-removal 🚨".into());
            }
        }

        let trie_ids = Vec::<Vec<u8>>::decode(&mut &state[..])
            .map_err(|_| "Failed to decode the pre-upgrade trie id snapshot")?;
        let checked = trie_ids.len();
        for trie_id in trie_ids {
            if sp_io::default_child_storage::next_key(&trie_id, &[]).is_some() {
                return Err("An orphaned contract child trie is still populated 🚨".into());
            }
        }

        log::info!(
            target: LOG_TARGET,
            "purge: post_upgrade OK, {checked} child tries verified empty and no trie id entries \
             remain ✅",
        );
        Ok(())
    }
}

/// Removes keys of the default child trie `trie_id` until the meter or `max_keys` is exhausted.
///
/// Returns how many keys were removed alongside the progress made. Iteration uses a local cursor,
/// so the cost stays linear even when a single trie spans several steps within one block.
pub(crate) fn clear_child_trie_metered<W: WeightInfo>(
    trie_id: &[u8],
    meter: &mut WeightMeter,
    max_keys: u32,
) -> (Progress, u32) {
    // The empty key sorts before every real key, so it yields the first entry of the trie.
    let mut from: Vec<u8> = Vec::new();
    let mut removed = 0u32;

    while let Some(key) = sp_io::default_child_storage::next_key(trie_id, &from) {
        if removed >= max_keys {
            return (Progress::Exhausted { required: None }, removed);
        }

        let value_len =
            sp_io::default_child_storage::read(trie_id, &key, &mut [], 0).unwrap_or_default();
        let cost = W::remove_child_key(value_len);

        if meter.try_consume(cost).is_err() {
            return (
                Progress::Exhausted {
                    required: Some(cost),
                },
                removed,
            );
        }

        sp_io::default_child_storage::clear(trie_id, &key);
        removed = removed.saturating_add(1);
        from = key;
    }

    (Progress::Finished, removed)
}
