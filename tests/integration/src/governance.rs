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

use crate::{propose_vote_and_close, setup::*};

use frame_support::{
    dispatch::GetDispatchInfo,
    traits::{Currency, StorePreimage},
};
use pallet_democracy::{AccountVote, Conviction, Vote};
use pallet_tx_pause::RuntimeCallNameOf;
use parity_scale_codec::Encode;
use sp_runtime::traits::{BlakeTwo256, Dispatchable, Hash};

#[test]
fn external_proposals_work() {
    new_test_ext().execute_with(|| {
        let remark_call = RuntimeCall::System(frame_system::Call::remark {
            remark: b"1337".to_vec(),
        });
        let remark_call_bounded = Preimage::bound(remark_call).unwrap();

        let external_propose_call =
            RuntimeCall::Democracy(pallet_democracy::Call::external_propose_majority {
                proposal: remark_call_bounded.clone(),
            });

        // Main council should be able to make external proposals
        propose_vote_and_close!(Council, external_propose_call, 0);

        let next_external_proposal = pallet_democracy::NextExternal::<Runtime>::get().unwrap();
        assert_eq!(
            next_external_proposal.0, remark_call_bounded,
            "Call should have been put as the next external proposal."
        );

        // Fast-track the proposal
        let (voting_period, delay) = (13, 17);
        let fast_track_call = RuntimeCall::Democracy(pallet_democracy::Call::fast_track {
            proposal_hash: next_external_proposal.0.hash(),
            voting_period,
            delay,
        });

        // Tech committee should be able to fast-track external proposals
        propose_vote_and_close!(TechnicalCommittee, fast_track_call, 0);

        // Basic check that a new (first) referendum was created
        let referendum_index = 0;
        let created_referendum =
            pallet_democracy::ReferendumInfoOf::<Runtime>::get(referendum_index).unwrap();
        matches!(
            created_referendum,
            pallet_democracy::ReferendumInfo::Ongoing(_)
        );
    })
}

#[test]
fn community_council_can_execute_dapp_staking_calls() {
    new_test_ext().execute_with(|| {
        // Fund the proxy account
        let proxy_account = <Runtime as pallet_collective_proxy::Config>::ProxyAccountId::get();
        let lock_amount = 10_000_000_000_000_000_000_000;
        Balances::make_free_balance_be(&proxy_account, lock_amount);

        // Prepare the wrapped dApp staking lock call
        let lock_call = RuntimeCall::DappStaking(pallet_dapp_staking::Call::lock {
            amount: lock_amount,
        });
        let collective_proxy_call =
            RuntimeCall::CollectiveProxy(pallet_collective_proxy::Call::execute_call {
                call: Box::new(lock_call),
            });

        // Community council should be able to execute dApp staking calls
        propose_vote_and_close!(CommunityCouncil, collective_proxy_call, 0);

        // Check that the lock was successful
        assert_eq!(
            pallet_dapp_staking::Ledger::<Runtime>::get(&proxy_account).locked(),
            lock_amount
        );
    })
}

#[test]
fn main_council_and_tech_committee_can_tx_pause() {
    new_test_ext().execute_with(|| {
        // Prepare calls.
        let call_full_name = full_name::<Runtime>(b"Balances", b"transfer_allow_death");
        let transfer_call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB.clone().into(),
            value: 1,
        });

        let tx_pause_proposal = RuntimeCall::TxPause(pallet_tx_pause::Call::pause {
            full_name: call_full_name.clone(),
        });

        // Sanity check - ensure transfer_allow_death isn't filtered.
        assert_ok!(transfer_call
            .clone()
            .dispatch(RuntimeOrigin::signed(ALICE.clone()),));

        // Main council should be able to propose a tx pause.
        propose_vote_and_close!(Council, tx_pause_proposal, 0);

        // Now ensure transfer_allow_death is filtered.
        assert_noop!(
            transfer_call
                .clone()
                .dispatch(RuntimeOrigin::signed(ALICE.clone())),
            frame_system::Error::<Runtime>::CallFiltered
        );

        // Now use tech committee to unpause the call
        let tx_unpause_proposal = RuntimeCall::TxPause(pallet_tx_pause::Call::unpause {
            ident: call_full_name.clone(),
        });
        propose_vote_and_close!(TechnicalCommittee, tx_unpause_proposal, 0);

        // Call should once again work.
        assert_ok!(transfer_call.dispatch(RuntimeOrigin::signed(ALICE.clone()),));
    })
}

#[test]
fn main_council_and_tech_committee_can_trigger_safe_mode() {
    new_test_ext().execute_with(|| {
        // Prepare calls.
        let safe_mode_enter_call = RuntimeCall::SafeMode(pallet_safe_mode::Call::force_enter {});
        let transfer_call = RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
            dest: BOB.clone().into(),
            value: 1,
        });

        // Sanity check - ensure transfer_allow_death isn't filtered.
        assert_ok!(transfer_call
            .clone()
            .dispatch(RuntimeOrigin::signed(ALICE.clone()),));

        // Tech committee should be able to propose a tx pause.
        propose_vote_and_close!(TechnicalCommittee, safe_mode_enter_call, 0);

        // Now ensure transfer_allow_death is filtered.
        assert_noop!(
            transfer_call
                .clone()
                .dispatch(RuntimeOrigin::signed(ALICE.clone())),
            frame_system::Error::<Runtime>::CallFiltered
        );

        // Now use main council to extend safe mode
        let safe_mode_extend_call = RuntimeCall::SafeMode(pallet_safe_mode::Call::force_extend {});
        propose_vote_and_close!(Council, safe_mode_extend_call, 0);

        // And use it again to exit safe mode
        let safe_mode_exit_call = RuntimeCall::SafeMode(pallet_safe_mode::Call::force_exit {});
        propose_vote_and_close!(Council, safe_mode_exit_call, 1);

        // Call should once again work.
        assert_ok!(transfer_call
            .clone()
            .dispatch(RuntimeOrigin::signed(ALICE.clone()),));
    })
}

#[test]
fn ensure_lockout_not_possible() {
    new_test_ext().execute_with(|| {
        // Enable safe mode.
        let safe_mode_enter_call = RuntimeCall::SafeMode(pallet_safe_mode::Call::force_enter {});
        propose_vote_and_close!(TechnicalCommittee, safe_mode_enter_call, 0);

        // Sanity check that e.g. transfer doesn't work anymore.
        assert_noop!(
            RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
                dest: BOB.clone().into(),
                value: 1,
            })
            .dispatch(RuntimeOrigin::signed(ALICE.clone())),
            frame_system::Error::<Runtime>::CallFiltered
        );

        // However, calls related to council & tech committee must still work.
        let call = RuntimeCall::System(frame_system::Call::remark {
            remark: b"abc".to_vec(),
        });
        assert_ok!(RuntimeCall::Council(pallet_collective::Call::propose {
            threshold: 3,
            proposal: Box::new(call.clone()),
            length_bound: call.encode().len() as u32,
        })
        .dispatch(RuntimeOrigin::signed(ALICE.clone())));

        assert_ok!(
            RuntimeCall::TechnicalCommittee(pallet_collective::Call::propose {
                threshold: 3,
                proposal: Box::new(call.clone()),
                length_bound: call.encode().len() as u32,
            })
            .dispatch(RuntimeOrigin::signed(ALICE.clone()))
        );
    })
}

#[test]
fn simulate_chain_recovery() {
    new_test_ext().execute_with(|| {
        // 1. First put the runtime into safe mode.
        let safe_mode_enter_call = RuntimeCall::SafeMode(pallet_safe_mode::Call::force_enter {});
        propose_vote_and_close!(TechnicalCommittee, safe_mode_enter_call, 0);

        // 2. Now prepare a call that will "save" the chain.
        let transfer_amount = 1337;
        let recovery_call = RuntimeCall::Balances(pallet_balances::Call::force_transfer {
            source: BOB.clone().into(),
            dest: ALICE.clone().into(),
            value: transfer_amount,
        });
        let recovery_call_bounded = Preimage::bound(recovery_call).unwrap();

        // 3. Use the council to prepare an external proposal
        let external_propose_call =
            RuntimeCall::Democracy(pallet_democracy::Call::external_propose_majority {
                proposal: recovery_call_bounded.clone(),
            });
        propose_vote_and_close!(Council, external_propose_call, 0);

        let next_external_proposal = pallet_democracy::NextExternal::<Runtime>::get().unwrap();
        assert_eq!(
            next_external_proposal.0, recovery_call_bounded,
            "Call should have been put as the next external proposal."
        );

        // 4. Use tech committee to fast-track it
        let (voting_period, delay) = (1, 1);
        let fast_track_call = RuntimeCall::Democracy(pallet_democracy::Call::fast_track {
            proposal_hash: next_external_proposal.0.hash(),
            voting_period,
            delay,
        });
        propose_vote_and_close!(TechnicalCommittee, fast_track_call, 1);

        // 5. Vote for it, it must succeed
        assert_ok!(RuntimeCall::Democracy(pallet_democracy::Call::vote {
            ref_index: 0,
            vote: AccountVote::Standard {
                vote: Vote {
                    aye: true,
                    conviction: Conviction::Locked1x
                },
                balance: 1337
            },
        })
        .dispatch(RuntimeOrigin::signed(ALICE.clone())));
        let init_alice_balance = Balances::free_balance(&ALICE);

        // 6. Demonstrate it's possible to prevent votes using tx-pause.
        let tx_pause_proposal = RuntimeCall::TxPause(pallet_tx_pause::Call::pause {
            full_name: full_name::<Runtime>(b"Democracy", b"vote"),
        });
        propose_vote_and_close!(Council, tx_pause_proposal, 1);
        assert_noop!(
            RuntimeCall::Democracy(pallet_democracy::Call::vote {
                ref_index: 0,
                vote: AccountVote::Standard {
                    vote: Vote {
                        aye: true,
                        conviction: Conviction::Locked1x
                    },
                    balance: 1337
                },
            })
            .dispatch(RuntimeOrigin::signed(BOB.clone())),
            frame_system::Error::<Runtime>::CallFiltered
        );

        // 7. Complete the referendum, schedule the call, and have it executed.
        run_for_blocks(2);
        assert_eq!(
            Balances::free_balance(&ALICE),
            init_alice_balance + transfer_amount,
            "Alice should have received the transfer amount."
        );

        // 8. Exit safe mode.
        let safe_mode_exit_call = RuntimeCall::SafeMode(pallet_safe_mode::Call::force_exit {});
        propose_vote_and_close!(Council, safe_mode_exit_call, 2);
        assert_ok!(
            RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
                dest: BOB.clone().into(),
                value: 1,
            })
            .dispatch(RuntimeOrigin::signed(ALICE.clone())),
        );
    })
}

/// Helper function to create a full name for a call in the format needed by the TxPause pallet.
fn full_name<T: pallet_tx_pause::Config>(
    pallet_name: &[u8],
    call_name: &[u8],
) -> RuntimeCallNameOf<T> {
    <RuntimeCallNameOf<T>>::from((
        pallet_name.to_vec().try_into().unwrap(),
        call_name.to_vec().try_into().unwrap(),
    ))
}

/// Macro to propose a call, vote on it & close it.
/// The following parameters are expected:
/// - `$collective`: The collective pallet to use (e.g., `Council`, `TechnicalCommittee`, `CommunityCouncil`).
/// - `$call`: The call to propose.
/// - `$index`: The index of the proposal (e.g., `0` for the first proposal).
#[macro_export]
macro_rules! propose_vote_and_close {
    ($collective:ident, $call:expr, $index:expr) => {{
        {
            let call = $call.clone();
            let call_hash = BlakeTwo256::hash_of(&call);

            // Propose the call as Alice
            assert_ok!(RuntimeCall::$collective(pallet_collective::Call::propose {
                threshold: 3,
                proposal: Box::new(call.clone()),
                length_bound: call.encode().len() as u32
            })
            .dispatch(RuntimeOrigin::signed(ALICE.clone())));

            // Vote 'aye'
            for signer in &[ALICE, BOB, CAT] {
                assert_ok!(RuntimeCall::$collective(pallet_collective::Call::vote {
                    proposal: call_hash,
                    index: $index,
                    approve: true
                })
                .dispatch(RuntimeOrigin::signed(signer.clone())));
            }

            // Close the proposal & execute it
            assert_ok!(RuntimeCall::$collective(pallet_collective::Call::close {
                proposal_hash: call_hash,
                index: $index,
                proposal_weight_bound: call.get_dispatch_info().total_weight(),
                length_bound: call.encode().len() as u32,
            })
            .dispatch(RuntimeOrigin::signed(ALICE.clone())));
        }
    }};
}
