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

use crate::test::mock::*;
use crate::types::*;
use crate::{
    pallet as pallet_dapp_staking, ActiveProtocolState, BlockNumberFor, ContractStake,
    CurrentEraInfo, DAppId, EraRewards, Event, IntegratedDApps, Ledger, NextDAppId, PeriodEnd,
    PeriodEndInfo, StakeAmount, StakerInfo,
};

use frame_support::{assert_ok, traits::Get};
use sp_runtime::{traits::Zero, Perbill};
use std::collections::HashMap;

/// Helper struct used to store the entire pallet state snapshot.
/// Used when comparison of before/after states is required.
#[derive(Debug)]
pub(crate) struct MemorySnapshot {
    active_protocol_state: ProtocolState<BlockNumberFor<Test>>,
    next_dapp_id: DAppId,
    current_era_info: EraInfo,
    integrated_dapps: HashMap<
        <Test as pallet_dapp_staking::Config>::SmartContract,
        DAppInfo<<Test as frame_system::Config>::AccountId>,
    >,
    ledger: HashMap<<Test as frame_system::Config>::AccountId, AccountLedgerFor<Test>>,
    staker_info: HashMap<
        (
            <Test as frame_system::Config>::AccountId,
            <Test as pallet_dapp_staking::Config>::SmartContract,
        ),
        SingularStakingInfo,
    >,
    contract_stake:
        HashMap<<Test as pallet_dapp_staking::Config>::SmartContract, ContractStakeAmountSeries>,
    era_rewards: HashMap<
        EraNumber,
        EraRewardSpan<<Test as pallet_dapp_staking::Config>::EraRewardSpanLength>,
    >,
    period_end: HashMap<PeriodNumber, PeriodEndInfo>,
}

impl MemorySnapshot {
    /// Generate a new memory snapshot, capturing entire dApp staking pallet state.
    pub fn new() -> Self {
        Self {
            active_protocol_state: ActiveProtocolState::<Test>::get(),
            next_dapp_id: NextDAppId::<Test>::get(),
            current_era_info: CurrentEraInfo::<Test>::get(),
            integrated_dapps: IntegratedDApps::<Test>::iter().collect(),
            ledger: Ledger::<Test>::iter().collect(),
            staker_info: StakerInfo::<Test>::iter()
                .map(|(k1, k2, v)| ((k1, k2), v))
                .collect(),
            contract_stake: ContractStake::<Test>::iter().collect(),
            era_rewards: EraRewards::<Test>::iter().collect(),
            period_end: PeriodEnd::<Test>::iter().collect(),
        }
    }

    /// Returns locked balance in dApp staking for the specified account.
    /// In case no balance is locked, returns zero.
    pub fn locked_balance(&self, account: &AccountId) -> Balance {
        self.ledger
            .get(&account)
            .map_or(Balance::zero(), |ledger| ledger.active_locked_amount())
    }
}

/// Register contract for staking and assert success.
pub(crate) fn assert_register(owner: AccountId, smart_contract: &MockSmartContract) {
    // Init check to ensure smart contract hasn't already been integrated
    assert!(!IntegratedDApps::<Test>::contains_key(smart_contract));
    let pre_snapshot = MemorySnapshot::new();

    // Register smart contract
    assert_ok!(DappStaking::register(
        RuntimeOrigin::root(),
        owner,
        smart_contract.clone()
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppRegistered {
        owner,
        smart_contract: smart_contract.clone(),
        dapp_id: pre_snapshot.next_dapp_id,
    }));

    // Verify post-state
    let dapp_info = IntegratedDApps::<Test>::get(smart_contract).unwrap();
    assert_eq!(dapp_info.state, DAppState::Registered);
    assert_eq!(dapp_info.owner, owner);
    assert_eq!(dapp_info.id, pre_snapshot.next_dapp_id);
    assert!(dapp_info.reward_destination.is_none());

    assert_eq!(pre_snapshot.next_dapp_id + 1, NextDAppId::<Test>::get());
    assert_eq!(
        pre_snapshot.integrated_dapps.len() + 1,
        IntegratedDApps::<Test>::count() as usize
    );
}

/// Update dApp reward destination and assert success
pub(crate) fn assert_set_dapp_reward_destination(
    owner: AccountId,
    smart_contract: &MockSmartContract,
    beneficiary: Option<AccountId>,
) {
    // Change reward destination
    assert_ok!(DappStaking::set_dapp_reward_destination(
        RuntimeOrigin::signed(owner),
        smart_contract.clone(),
        beneficiary,
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(
        Event::DAppRewardDestinationUpdated {
            smart_contract: smart_contract.clone(),
            beneficiary: beneficiary,
        },
    ));

    // Sanity check & reward destination update
    assert_eq!(
        IntegratedDApps::<Test>::get(&smart_contract)
            .unwrap()
            .reward_destination,
        beneficiary
    );
}

/// Update dApp owner and assert success.
/// if `caller` is `None`, `Root` origin is used, otherwise standard `Signed` origin is used.
pub(crate) fn assert_set_dapp_owner(
    caller: Option<AccountId>,
    smart_contract: &MockSmartContract,
    new_owner: AccountId,
) {
    let origin = caller.map_or(RuntimeOrigin::root(), |owner| RuntimeOrigin::signed(owner));

    // Change dApp owner
    assert_ok!(DappStaking::set_dapp_owner(
        origin,
        smart_contract.clone(),
        new_owner,
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppOwnerChanged {
        smart_contract: smart_contract.clone(),
        new_owner,
    }));

    // Verify post-state
    assert_eq!(
        IntegratedDApps::<Test>::get(&smart_contract).unwrap().owner,
        new_owner
    );
}

/// Update dApp status to unregistered and assert success.
pub(crate) fn assert_unregister(smart_contract: &MockSmartContract) {
    let pre_snapshot = MemorySnapshot::new();

    // Unregister dApp
    assert_ok!(DappStaking::unregister(
        RuntimeOrigin::root(),
        smart_contract.clone(),
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppUnregistered {
        smart_contract: smart_contract.clone(),
        era: pre_snapshot.active_protocol_state.era,
    }));

    // Verify post-state
    assert_eq!(
        IntegratedDApps::<Test>::get(&smart_contract).unwrap().state,
        DAppState::Unregistered(pre_snapshot.active_protocol_state.era),
    );
}

/// Lock funds into dApp staking and assert success.
pub(crate) fn assert_lock(account: AccountId, amount: Balance) {
    let pre_snapshot = MemorySnapshot::new();

    let free_balance = Balances::free_balance(&account);
    let locked_balance = pre_snapshot.locked_balance(&account);
    let available_balance = free_balance
        .checked_sub(locked_balance)
        .expect("Locked amount cannot be greater than available free balance");
    let expected_lock_amount = available_balance.min(amount);
    assert!(!expected_lock_amount.is_zero());

    // Lock funds
    assert_ok!(DappStaking::lock(RuntimeOrigin::signed(account), amount,));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::Locked {
        account,
        amount: expected_lock_amount,
    }));

    // Verify post-state
    let post_snapshot = MemorySnapshot::new();

    assert_eq!(
        post_snapshot.locked_balance(&account),
        locked_balance + expected_lock_amount,
        "Locked balance should be increased by the amount locked."
    );

    assert_eq!(
        post_snapshot.current_era_info.total_locked,
        pre_snapshot.current_era_info.total_locked + expected_lock_amount,
        "Total locked balance should be increased by the amount locked."
    );
    assert_eq!(
        post_snapshot.current_era_info.active_era_locked,
        pre_snapshot.current_era_info.active_era_locked,
        "Active era locked amount should remain exactly the same."
    );
}

/// Start the unlocking process for locked funds and assert success.
pub(crate) fn assert_unlock(account: AccountId, amount: Balance) {
    let pre_snapshot = MemorySnapshot::new();

    assert!(
        pre_snapshot.ledger.contains_key(&account),
        "Cannot unlock for non-existing ledger."
    );

    // Calculate expected unlock amount
    let pre_ledger = &pre_snapshot.ledger[&account];
    let expected_unlock_amount = {
        // Cannot unlock more than is available
        let possible_unlock_amount = pre_ledger
            .unlockable_amount(pre_snapshot.active_protocol_state.period_number())
            .min(amount);

        // When unlocking would take account below the minimum lock threshold, unlock everything
        let locked_amount = pre_ledger.active_locked_amount();
        let min_locked_amount = <Test as pallet_dapp_staking::Config>::MinimumLockedAmount::get();
        if locked_amount.saturating_sub(possible_unlock_amount) < min_locked_amount {
            locked_amount
        } else {
            possible_unlock_amount
        }
    };

    // Unlock funds
    assert_ok!(DappStaking::unlock(RuntimeOrigin::signed(account), amount,));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::Unlocking {
        account,
        amount: expected_unlock_amount,
    }));

    // Verify post-state
    let post_snapshot = MemorySnapshot::new();

    // Verify ledger is as expected
    let period_number = pre_snapshot.active_protocol_state.period_number();
    let post_ledger = &post_snapshot.ledger[&account];
    assert_eq!(
        pre_ledger.active_locked_amount(),
        post_ledger.active_locked_amount() + expected_unlock_amount,
        "Active locked amount should be decreased by the amount unlocked."
    );
    assert_eq!(
        pre_ledger.unlocking_amount() + expected_unlock_amount,
        post_ledger.unlocking_amount(),
        "Total unlocking amount should be increased by the amount unlocked."
    );
    assert_eq!(
        pre_ledger.total_locked_amount(),
        post_ledger.total_locked_amount(),
        "Total locked amount should remain exactly the same since the unlocking chunks are still locked."
    );
    assert_eq!(
        pre_ledger.unlockable_amount(period_number),
        post_ledger.unlockable_amount(period_number) + expected_unlock_amount,
        "Unlockable amount should be decreased by the amount unlocked."
    );

    // In case ledger is empty, it should have been removed from the storage
    if post_ledger.is_empty() {
        assert!(!Ledger::<Test>::contains_key(&account));
    }

    // Verify era info post-state
    let pre_era_info = &pre_snapshot.current_era_info;
    let post_era_info = &post_snapshot.current_era_info;
    assert_eq!(
        pre_era_info.unlocking + expected_unlock_amount,
        post_era_info.unlocking
    );
    assert_eq!(
        pre_era_info
            .total_locked
            .saturating_sub(expected_unlock_amount),
        post_era_info.total_locked
    );
    assert_eq!(
        pre_era_info
            .active_era_locked
            .saturating_sub(expected_unlock_amount),
        post_era_info.active_era_locked
    );
}

/// Claims the unlocked funds back into free balance of the user and assert success.
pub(crate) fn assert_claim_unlocked(account: AccountId) {
    let pre_snapshot = MemorySnapshot::new();

    assert!(
        pre_snapshot.ledger.contains_key(&account),
        "Cannot claim unlocked for non-existing ledger."
    );

    let current_block = System::block_number();
    let mut consumed_chunks = 0;
    let mut amount = 0;
    for unlock_chunk in pre_snapshot.ledger[&account].clone().unlocking.into_inner() {
        if unlock_chunk.unlock_block <= current_block {
            amount += unlock_chunk.amount;
            consumed_chunks += 1;
        }
    }

    // Claim unlocked chunks
    assert_ok!(DappStaking::claim_unlocked(RuntimeOrigin::signed(account)));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::ClaimedUnlocked {
        account,
        amount,
    }));

    // Verify post-state
    let post_snapshot = MemorySnapshot::new();

    let post_ledger = if let Some(ledger) = post_snapshot.ledger.get(&account) {
        ledger.clone()
    } else {
        Default::default()
    };

    assert_eq!(
        post_ledger.unlocking.len(),
        pre_snapshot.ledger[&account].unlocking.len() - consumed_chunks
    );
    assert_eq!(
        post_ledger.unlocking_amount(),
        pre_snapshot.ledger[&account].unlocking_amount() - amount
    );
    assert_eq!(
        post_snapshot.current_era_info.unlocking,
        pre_snapshot.current_era_info.unlocking - amount
    );
}

/// Claims the unlocked funds back into free balance of the user and assert success.
pub(crate) fn assert_relock_unlocking(account: AccountId) {
    let pre_snapshot = MemorySnapshot::new();

    assert!(
        pre_snapshot.ledger.contains_key(&account),
        "Cannot relock unlocking non-existing ledger."
    );

    let amount = pre_snapshot.ledger[&account].unlocking_amount();

    // Relock unlocking chunks
    assert_ok!(DappStaking::relock_unlocking(RuntimeOrigin::signed(
        account
    )));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::Relock { account, amount }));

    // Verify post-state
    let post_snapshot = MemorySnapshot::new();

    // Account ledger
    let post_ledger = &post_snapshot.ledger[&account];
    assert!(post_ledger.unlocking.is_empty());
    assert!(post_ledger.unlocking_amount().is_zero());
    assert_eq!(
        post_ledger.active_locked_amount(),
        pre_snapshot.ledger[&account].active_locked_amount() + amount
    );

    // Current era info
    assert_eq!(
        post_snapshot.current_era_info.unlocking,
        pre_snapshot.current_era_info.unlocking - amount
    );
    assert_eq!(
        post_snapshot.current_era_info.total_locked,
        pre_snapshot.current_era_info.total_locked + amount
    );
}

/// Stake some funds on the specified smart contract.
pub(crate) fn assert_stake(
    account: AccountId,
    smart_contract: &MockSmartContract,
    amount: Balance,
) {
    // TODO: this is a huge function - I could break it down, but I'm not sure it will help with readability.
    let pre_snapshot = MemorySnapshot::new();
    let pre_ledger = pre_snapshot.ledger.get(&account).unwrap();
    let pre_staker_info = pre_snapshot
        .staker_info
        .get(&(account, smart_contract.clone()));
    let pre_contract_stake = pre_snapshot
        .contract_stake
        .get(&smart_contract)
        .map_or(ContractStakeAmountSeries::default(), |series| {
            series.clone()
        });
    let pre_era_info = pre_snapshot.current_era_info;

    let stake_era = pre_snapshot.active_protocol_state.era + 1;
    let stake_period = pre_snapshot.active_protocol_state.period_number();
    let stake_period_type = pre_snapshot.active_protocol_state.period_type();

    // Stake on smart contract & verify event
    assert_ok!(DappStaking::stake(
        RuntimeOrigin::signed(account),
        smart_contract.clone(),
        amount
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::Stake {
        account,
        smart_contract: smart_contract.clone(),
        amount,
    }));

    // Verify post-state
    let post_snapshot = MemorySnapshot::new();
    let post_ledger = post_snapshot.ledger.get(&account).unwrap();
    let post_staker_info = post_snapshot
        .staker_info
        .get(&(account, *smart_contract))
        .expect("Entry must exist since 'stake' operation was successfull.");
    let post_contract_stake = post_snapshot
        .contract_stake
        .get(&smart_contract)
        .expect("Entry must exist since 'stake' operation was successfull.");
    let post_era_info = post_snapshot.current_era_info;

    // 1. verify ledger
    // =====================
    // =====================
    assert_eq!(
        post_ledger.staked, pre_ledger.staked,
        "Must remain exactly the same."
    );
    assert_eq!(post_ledger.staked_future.unwrap().period, stake_period);
    assert_eq!(
        post_ledger.staked_amount(stake_period),
        pre_ledger.staked_amount(stake_period) + amount,
        "Stake amount must increase by the 'amount'"
    );
    assert_eq!(
        post_ledger.stakeable_amount(stake_period),
        pre_ledger.stakeable_amount(stake_period) - amount,
        "Stakeable amount must decrease by the 'amount'"
    );
    // TODO: maybe expand checks here?

    // 2. verify staker info
    // =====================
    // =====================
    match pre_staker_info {
        // We're just updating an existing entry
        Some(pre_staker_info) if pre_staker_info.period_number() == stake_period => {
            assert_eq!(
                post_staker_info.total_staked_amount(),
                pre_staker_info.total_staked_amount() + amount,
                "Total staked amount must increase by the 'amount'"
            );
            assert_eq!(
                post_staker_info.staked_amount(stake_period_type),
                pre_staker_info.staked_amount(stake_period_type) + amount,
                "Staked amount must increase by the 'amount'"
            );
            assert_eq!(post_staker_info.period_number(), stake_period);
            assert_eq!(
                post_staker_info.is_loyal(),
                pre_staker_info.is_loyal(),
                "Staking operation mustn't change loyalty flag."
            );
        }
        // A new entry is created.
        _ => {
            assert_eq!(
                post_staker_info.total_staked_amount(),
                amount,
                "Total staked amount must be equal to exactly the 'amount'"
            );
            assert!(amount >= <Test as pallet_dapp_staking::Config>::MinimumStakeAmount::get());
            assert_eq!(
                post_staker_info.staked_amount(stake_period_type),
                amount,
                "Staked amount must be equal to exactly the 'amount'"
            );
            assert_eq!(post_staker_info.period_number(), stake_period);
            assert_eq!(
                post_staker_info.is_loyal(),
                stake_period_type == PeriodType::Voting
            );
        }
    }

    // 3. verify contract stake
    // =========================
    // =========================
    assert_eq!(
        post_contract_stake.total_staked_amount(stake_period),
        pre_contract_stake.total_staked_amount(stake_period) + amount,
        "Staked amount must increase by the 'amount'"
    );
    assert_eq!(
        post_contract_stake.staked_amount(stake_period, stake_period_type),
        pre_contract_stake.staked_amount(stake_period, stake_period_type) + amount,
        "Staked amount must increase by the 'amount'"
    );

    assert_eq!(post_contract_stake.last_stake_period(), Some(stake_period));
    assert_eq!(post_contract_stake.last_stake_era(), Some(stake_era));

    // 4. verify era info
    // =========================
    // =========================
    assert_eq!(
        post_era_info.total_staked_amount(),
        pre_era_info.total_staked_amount(),
        "Total staked amount for the current era must remain the same."
    );
    assert_eq!(
        post_era_info.total_staked_amount_next_era(),
        pre_era_info.total_staked_amount_next_era() + amount
    );
    assert_eq!(
        post_era_info.staked_amount_next_era(stake_period_type),
        pre_era_info.staked_amount_next_era(stake_period_type) + amount
    );
}

/// Unstake some funds from the specified smart contract.
pub(crate) fn assert_unstake(
    account: AccountId,
    smart_contract: &MockSmartContract,
    amount: Balance,
) {
    let pre_snapshot = MemorySnapshot::new();
    let pre_ledger = pre_snapshot.ledger.get(&account).unwrap();
    let pre_staker_info = pre_snapshot
        .staker_info
        .get(&(account, smart_contract.clone()))
        .expect("Entry must exist since 'unstake' is being called.");
    let pre_contract_stake = pre_snapshot
        .contract_stake
        .get(&smart_contract)
        .expect("Entry must exist since 'unstake' is being called.");
    let pre_era_info = pre_snapshot.current_era_info;

    let _unstake_era = pre_snapshot.active_protocol_state.era;
    let unstake_period = pre_snapshot.active_protocol_state.period_number();
    let unstake_period_type = pre_snapshot.active_protocol_state.period_type();

    let minimum_stake_amount: Balance =
        <Test as pallet_dapp_staking::Config>::MinimumStakeAmount::get();
    let is_full_unstake =
        pre_staker_info.total_staked_amount().saturating_sub(amount) < minimum_stake_amount;

    // Unstake all if we expect to go below the minimum stake amount
    let amount = if is_full_unstake {
        pre_staker_info.total_staked_amount()
    } else {
        amount
    };

    // Unstake from smart contract & verify event
    assert_ok!(DappStaking::unstake(
        RuntimeOrigin::signed(account),
        smart_contract.clone(),
        amount
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::Unstake {
        account,
        smart_contract: smart_contract.clone(),
        amount,
    }));

    // Verify post-state
    let post_snapshot = MemorySnapshot::new();
    let post_ledger = post_snapshot.ledger.get(&account).unwrap();
    let post_contract_stake = post_snapshot
        .contract_stake
        .get(&smart_contract)
        .expect("Entry must exist since 'stake' operation was successfull.");
    let post_era_info = post_snapshot.current_era_info;

    // 1. verify ledger
    // =====================
    // =====================
    assert_eq!(
        post_ledger.staked_amount(unstake_period),
        pre_ledger.staked_amount(unstake_period) - amount,
        "Stake amount must decrease by the 'amount'"
    );
    assert_eq!(
        post_ledger.stakeable_amount(unstake_period),
        pre_ledger.stakeable_amount(unstake_period) + amount,
        "Stakeable amount must increase by the 'amount'"
    );
    // TODO: expand with more detailed checks of staked and staked_future

    // 2. verify staker info
    // =====================
    // =====================
    if is_full_unstake {
        assert!(
            !StakerInfo::<Test>::contains_key(&account, smart_contract),
            "Entry must be deleted since it was a full unstake."
        );
    } else {
        let post_staker_info = post_snapshot
        .staker_info
        .get(&(account, *smart_contract))
        .expect("Entry must exist since 'stake' operation was successfull and it wasn't a full unstake.");
        assert_eq!(post_staker_info.period_number(), unstake_period);
        assert_eq!(
            post_staker_info.total_staked_amount(),
            pre_staker_info.total_staked_amount() - amount,
            "Total staked amount must decrease by the 'amount'"
        );
        assert_eq!(
            post_staker_info.staked_amount(unstake_period_type),
            pre_staker_info
                .staked_amount(unstake_period_type)
                .saturating_sub(amount),
            "Staked amount must decrease by the 'amount'"
        );

        let is_loyal = pre_staker_info.is_loyal()
            && !(unstake_period_type == PeriodType::BuildAndEarn
                && post_staker_info.staked_amount(PeriodType::Voting)
                    < pre_staker_info.staked_amount(PeriodType::Voting));
        assert_eq!(
            post_staker_info.is_loyal(),
            is_loyal,
            "If 'Voting' stake amount is reduced in B&E period, loyalty flag must be set to false."
        );
    }

    // 3. verify contract stake
    // =========================
    // =========================
    assert_eq!(
        post_contract_stake.total_staked_amount(unstake_period),
        pre_contract_stake.total_staked_amount(unstake_period) - amount,
        "Staked amount must decreased by the 'amount'"
    );
    assert_eq!(
        post_contract_stake.staked_amount(unstake_period, unstake_period_type),
        pre_contract_stake
            .staked_amount(unstake_period, unstake_period_type)
            .saturating_sub(amount),
        "Staked amount must decreased by the 'amount'"
    );

    // 4. verify era info
    // =========================
    // =========================

    // It's possible next era has staked more than the current era. This is because 'stake' will always stake for the NEXT era.
    if pre_era_info.total_staked_amount() < amount {
        assert!(post_era_info.total_staked_amount().is_zero());
    } else {
        assert_eq!(
            post_era_info.total_staked_amount(),
            pre_era_info.total_staked_amount() - amount,
            "Total staked amount for the current era must decrease by 'amount'."
        );
    }
    assert_eq!(
        post_era_info.total_staked_amount_next_era(),
        pre_era_info.total_staked_amount_next_era() - amount,
        "Total staked amount for the next era must decrease by 'amount'. No overflow is allowed."
    );

    if unstake_period_type == PeriodType::BuildAndEarn
        && pre_era_info.staked_amount_next_era(PeriodType::BuildAndEarn) < amount
    {
        let overflow = amount - pre_era_info.staked_amount_next_era(PeriodType::BuildAndEarn);

        assert!(post_era_info
            .staked_amount_next_era(PeriodType::BuildAndEarn)
            .is_zero());
        assert_eq!(
            post_era_info.staked_amount_next_era(PeriodType::Voting),
            pre_era_info.staked_amount_next_era(PeriodType::Voting) - overflow
        );
    } else {
        assert_eq!(
            post_era_info.staked_amount_next_era(unstake_period_type),
            pre_era_info.staked_amount_next_era(unstake_period_type) - amount
        );
    }
}

/// Claim staker rewards.
pub(crate) fn assert_claim_staker_rewards(account: AccountId) {
    let pre_snapshot = MemorySnapshot::new();
    let pre_ledger = pre_snapshot.ledger.get(&account).unwrap();
    let pre_total_issuance = <Test as pallet_dapp_staking::Config>::Currency::total_issuance();
    let pre_free_balance = <Test as pallet_dapp_staking::Config>::Currency::free_balance(&account);

    // Get the first eligible era for claiming rewards
    let first_claim_era = pre_ledger
        .earliest_staked_era()
        .expect("Entry must exist, otherwise 'claim' is invalid.");

    // Get the apprropriate era rewards span for the 'first era'
    let era_span_length: EraNumber =
        <Test as pallet_dapp_staking::Config>::EraRewardSpanLength::get();
    let era_span_index = first_claim_era - (first_claim_era % era_span_length);
    let era_rewards_span = pre_snapshot
        .era_rewards
        .get(&era_span_index)
        .expect("Entry must exist, otherwise 'claim' is invalid.");

    // Calculate the final era for claiming rewards. Also determine if this will fully claim all staked period rewards.
    let claim_period_end = if pre_ledger.staked_period().unwrap()
        == pre_snapshot.active_protocol_state.period_number()
    {
        None
    } else {
        Some(
            pre_snapshot
                .period_end
                .get(&pre_ledger.staked_period().unwrap())
                .expect("Entry must exist, since it's the current period.")
                .final_era,
        )
    };

    let (last_claim_era, is_full_claim) = if claim_period_end.is_none() {
        (pre_snapshot.active_protocol_state.era - 1, false)
    } else {
        let claim_period = pre_ledger.staked_period().unwrap();
        let period_end = pre_snapshot
            .period_end
            .get(&claim_period)
            .expect("Entry must exist, since it's a past period.");

        let last_claim_era = era_rewards_span.last_era().min(period_end.final_era);
        let is_full_claim = last_claim_era == period_end.final_era;
        (last_claim_era, is_full_claim)
    };

    assert!(
        last_claim_era < pre_snapshot.active_protocol_state.era,
        "Sanity check."
    );

    // Calculate the expected rewards
    let mut rewards = Vec::new();
    for (era, amount) in pre_ledger
        .clone()
        .claim_up_to_era(last_claim_era, claim_period_end)
        .unwrap()
    {
        let era_reward_info = era_rewards_span
            .get(era)
            .expect("Entry must exist, otherwise 'claim' is invalid.");

        let reward = Perbill::from_rational(amount, era_reward_info.staked())
            * era_reward_info.staker_reward_pool();
        if reward.is_zero() {
            continue;
        }

        rewards.push((era, reward));
    }
    let total_reward = rewards
        .iter()
        .fold(Balance::zero(), |acc, (_, reward)| acc + reward);

    //clean up possible leftover events
    System::reset_events();

    // Unstake from smart contract & verify event(s)
    assert_ok!(DappStaking::claim_staker_rewards(RuntimeOrigin::signed(
        account
    ),));

    let events = dapp_staking_events();
    assert_eq!(events.len(), rewards.len());
    for (event, (era, reward)) in events.iter().zip(rewards.iter()) {
        assert_eq!(
            event,
            &Event::<Test>::Reward {
                account,
                era: *era,
                amount: *reward,
            }
        );
    }

    // Verify post state

    let post_total_issuance = <Test as pallet_dapp_staking::Config>::Currency::total_issuance();
    assert_eq!(
        post_total_issuance,
        pre_total_issuance + total_reward,
        "Total issuance must increase by the total reward amount."
    );

    let post_free_balance = <Test as pallet_dapp_staking::Config>::Currency::free_balance(&account);
    assert_eq!(
        post_free_balance,
        pre_free_balance + total_reward,
        "Free balance must increase by the total reward amount."
    );

    let post_snapshot = MemorySnapshot::new();
    let post_ledger = post_snapshot.ledger.get(&account).unwrap();

    if is_full_claim {
        assert!(post_ledger.staked.is_empty());
        assert!(post_ledger.staked_future.is_none());
    } else {
        assert_eq!(post_ledger.staked.era, last_claim_era + 1);
        // TODO: expand check?
    }
}

/// Returns from which starting era to which ending era can rewards be claimed for the specified account.
///
/// If `None` is returned, there is nothing to claim.
/// Doesn't consider reward expiration.
pub(crate) fn claimable_reward_range(account: AccountId) -> Option<(EraNumber, EraNumber)> {
    let ledger = Ledger::<Test>::get(&account);
    let protocol_state = ActiveProtocolState::<Test>::get();

    let earliest_stake_era = if let Some(era) = ledger.earliest_staked_era() {
        era
    } else {
        return None;
    };

    let last_claim_era = if ledger.staked_period() == Some(protocol_state.period_number()) {
        protocol_state.era - 1
    } else {
        // Period finished, we can claim up to its final era
        let period_end = PeriodEnd::<Test>::get(ledger.staked_period().unwrap()).unwrap();
        period_end.final_era
    };

    Some((earliest_stake_era, last_claim_era))
}

/// Number of times it's required to call `claim_staker_rewards` to claim all pending rewards.
///
/// In case no rewards are pending, return **zero**.
pub(crate) fn required_number_of_reward_claims(account: AccountId) -> u32 {
    let range = if let Some(range) = claimable_reward_range(account) {
        range
    } else {
        return 0;
    };

    let era_span_length: EraNumber =
        <Test as pallet_dapp_staking::Config>::EraRewardSpanLength::get();
    let first = DappStaking::era_reward_span_index(range.0)
        .checked_div(era_span_length)
        .unwrap();
    let second = DappStaking::era_reward_span_index(range.1)
        .checked_div(era_span_length)
        .unwrap();

    second - first + 1
}
