// TODO: details implementations for Pallet.

use super::{pallet::*, STAKING_ID};

impl<T: Config> Pallet<T> {
    // // MUTABLES (DANGEROUS)

    // /// Update the ledger for a controller. This will also update the stash lock. The lock will
    // /// will lock the entire funds except paying for further transactions.
    // fn update_ledger(
    //     controller: &T::AccountId,
    //     ledger: &StakingLedger<T::AccountId, BalanceOf<T>>,
    // ) {
    //     T::Currency::set_lock(
    //         STAKING_ID,
    //         &ledger.stash,
    //         ledger.total,
    //         WithdrawReasons::all(),
    //     );
    //     <Ledger<T>>::insert(controller, ledger);
    // }

    // /// Remove all associated data of a stash account from the staking system.
    // ///
    // /// Assumes storage is upgraded before calling.
    // ///
    // /// This is called :
    // /// - Immediately when an account's balance falls below existential deposit.
    // /// - after a `withdraw_unbond()` call that frees all of a stash's bonded balance.
    // fn kill_stash(stash: &T::AccountId) -> DispatchResult {
    //     let controller = Bonded::<T>::take(stash).ok_or(Error::<T>::NotStash)?;
    //     <Ledger<T>>::remove(&controller);

    //     <Payee<T>>::remove(stash);
    //     if let Some(nominations) = Self::dapps_nominations(stash) {
    //         Self::remove_nominations(stash, nominations);
    //     }

    //     system::Module::<T>::dec_consumers(stash);
    //     Ok(())
    // }

    // /// Chill a stash account.
    // fn chill_stash(stash: &T::AccountId) {
    //     if let Some(nominations) = Self::dapps_nominations(stash) {
    //         Self::remove_nominations(stash, nominations);
    //     }
    // }

    // fn propagate_nominate_totals(nominator: &T::AccountId, src_era: &EraIndex, dst_era: &EraIndex) {
    //     if <ErasNominateTotals<T>>::contains_key(src_era, nominator) {
    //         let untreated_nootate_total = <ErasNominateTotals<T>>::get(src_era, nominator);

    //         <ErasNominateTotals<T>>::mutate(dst_era, nominator, |total| {
    //             *total += untreated_nootate_total;
    //         })
    //     }
    // }

    // fn reward_nominator(
    //     era: &EraIndex,
    //     max_payout: BalanceOf<T>,
    //     nominator: &T::AccountId,
    // ) -> BalanceOf<T> {
    //     let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
    //     let (_, nominators_reward) =
    //         T::ComputeRewardsForDapps::compute_rewards_for_dapps(max_payout);

    //     let total_staked = Self::eras_total_stake(era);

    //     let mut nominate_values: Vec<_> = Vec::new();

    //     let mut each_points: Vec<_> = Vec::new();
    //     for e in era.saturating_sub(T::HistoryDepthFinder::get())..=*era {
    //         for (contract, points) in <ErasStakingPoints<T>>::iter_prefix(&e) {
    //             if Self::is_rewardable(&contract, &e) {
    //                 each_points.push((
    //                     Self::eras_staking_points(era, contract).total,
    //                     points.individual,
    //                 ));
    //             }
    //         }
    //     }

    //     for (total, individual) in each_points {
    //         for (account, value) in individual {
    //             if account == *nominator {
    //                 nominate_values.push((total, value));
    //             }
    //         }
    //     }

    //     let nominate_total = Self::eras_nominate_totals(era, nominator);
    //     let reward = T::ComputeRewardsForDapps::compute_reward_for_nominator(
    //         nominate_total,
    //         total_staked,
    //         nominators_reward,
    //         nominate_values,
    //     );
    //     total_imbalance.subsume(
    //         Self::make_payout(nominator, reward).unwrap_or(PositiveImbalanceOf::<T>::zero()),
    //     );

    //     let total_payout = total_imbalance.peek();

    //     let rest = max_payout.saturating_sub(total_payout.clone());

    //     T::Reward::on_unbalanced(total_imbalance);
    //     T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
    //     total_payout
    // }

    // fn reward_operator(
    //     era: &EraIndex,
    //     max_payout: BalanceOf<T>,
    //     operator: &T::AccountId,
    // ) -> BalanceOf<T> {
    //     let mut total_imbalance = <PositiveImbalanceOf<T>>::zero();
    //     let (operators_reward, _) =
    //         T::ComputeRewardsForDapps::compute_rewards_for_dapps(max_payout);

    //     let total_staked = Self::eras_total_stake(era);

    //     let mut stakes = BalanceOf::<T>::zero();
    //     for e in era.saturating_sub(T::HistoryDepthFinder::get())..=*era {
    //         for (contract, _) in <ErasStakingPoints<T>>::iter_prefix(&e) {
    //             if let Some(o) = T::ContractFinder::operator(&contract) {
    //                 if o == *operator && Self::is_rewardable(&contract, &e) {
    //                     stakes += Self::eras_staking_points(era, contract).total;
    //                 }
    //             }
    //         }
    //     }

    //     let reward = T::ComputeRewardsForDapps::compute_reward_for_operator(
    //         stakes,
    //         total_staked,
    //         operators_reward,
    //     );
    //     total_imbalance.subsume(
    //         T::Currency::deposit_into_existing(operator, reward)
    //             .unwrap_or(PositiveImbalanceOf::<T>::zero()),
    //     );

    //     let total_payout = total_imbalance.peek();

    //     let rest = max_payout.saturating_sub(total_payout.clone());

    //     T::Reward::on_unbalanced(total_imbalance);
    //     T::RewardRemainder::on_unbalanced(T::Currency::issue(rest));
    //     total_payout
    // }

    // fn propagate_eras_staking_points_total(
    //     contract: &T::AccountId,
    //     src_era: &EraIndex,
    //     dst_era: &EraIndex,
    // ) {
    //     if <ErasStakingPoints<T>>::contains_key(src_era, contract) {
    //         let untreated_points = <ErasStakingPoints<T>>::get(src_era, contract);

    //         <ErasStakingPoints<T>>::mutate(&dst_era, &contract, |points| {
    //             (*points).total += untreated_points.total.clone();
    //         });
    //     }
    // }

    // fn compute_total_stake(era: &EraIndex) -> BalanceOf<T> {
    //     let mut untreated_era = Self::untreated_era();
    //     while *era > untreated_era {
    //         let total = Self::eras_total_stake(&untreated_era);
    //         <ErasTotalStake<T>>::mutate(&untreated_era + 1, |next_total| *next_total += total);
    //         untreated_era += 1;
    //     }
    //     UntreatedEra::put(untreated_era);
    //     let total_staked = Self::eras_total_stake(era);
    //     total_staked
    // }

    // fn make_payout(stash: &T::AccountId, amount: BalanceOf<T>) -> Option<PositiveImbalanceOf<T>> {
    //     let dest = Self::payee(stash);
    //     match dest {
    //         RewardDestination::Controller => Self::bonded(stash).and_then(|controller| {
    //             T::Currency::deposit_into_existing(&controller, amount).ok()
    //         }),
    //         RewardDestination::Stash => T::Currency::deposit_into_existing(stash, amount).ok(),
    //         RewardDestination::Staked => Self::bonded(stash)
    //             .and_then(|c| Self::ledger(&c).map(|l| (c, l)))
    //             .and_then(|(controller, mut l)| {
    //                 l.active += amount;
    //                 l.total += amount;
    //                 let r = T::Currency::deposit_into_existing(stash, amount).ok();
    //                 Self::update_ledger(&controller, &l);
    //                 r
    //             }),
    //         _ => None,
    //     }
    // }

    // fn take_in_nominations(
    //     stash: &T::AccountId,
    //     nominations: Nominations<T::AccountId, BalanceOf<T>>,
    // ) {
    //     if let Some(current_era) = T::EraFinder::current() {
    //         let next_era = current_era + 1;

    //         for (contract, value) in nominations.targets.iter() {
    //             if <ErasStakingPoints<T>>::contains_key(&next_era, &contract) {
    //                 <ErasStakingPoints<T>>::mutate(&next_era, &contract, |points| {
    //                     (*points).total += value.clone();
    //                     (*points).individual.insert(stash.clone(), value.clone());
    //                 });
    //             } else {
    //                 let points = EraStakingPoints {
    //                     total: value.clone(),
    //                     individual: vec![(stash.clone(), value.clone())]
    //                         .into_iter()
    //                         .collect::<BTreeMap<T::AccountId, BalanceOf<T>>>(),
    //                 };
    //                 <ErasStakingPoints<T>>::insert(&next_era, &contract, points);
    //             }

    //             <ErasNominateTotals<T>>::mutate(&next_era, stash, |total| {
    //                 *total += value.clone();
    //             });

    //             <ErasTotalStake<T>>::mutate(&next_era, |total| {
    //                 *total += value.clone();
    //             });
    //         }
    //     }
    //     <DappsNominations<T>>::insert(stash, nominations);
    // }

    // fn remove_nominations(
    //     stash: &T::AccountId,
    //     nominations: Nominations<T::AccountId, BalanceOf<T>>,
    // ) {
    //     let era = nominations.submitted_in + 1;
    //     for (contract, value) in nominations.targets.iter() {
    //         <ErasStakingPoints<T>>::mutate(&era, &contract, |points| {
    //             (*points).total = points.total.saturating_sub(value.clone());
    //             (*points).individual.remove(stash);
    //         });

    //         <ErasNominateTotals<T>>::mutate(&era, stash, |total| {
    //             *total = total.saturating_sub(value.clone());
    //         });

    //         <ErasTotalStake<T>>::mutate(&era, |total| {
    //             *total = total.saturating_sub(value.clone());
    //         });
    //     }
    //     <DappsNominations<T>>::remove(stash);
    // }

    // fn propagate_eras_votes(contract: &T::AccountId, src_era: &EraIndex, dst_era: &EraIndex) {
    //     if <ErasVotes<T>>::contains_key(src_era, contract) {
    //         let untreated_votes = <ErasVotes<T>>::get(src_era, contract);

    //         <ErasVotes<T>>::mutate(&dst_era, &contract, |votes| {
    //             (*votes).bad += untreated_votes.bad.clone();
    //             (*votes).good += untreated_votes.good.clone();
    //         });
    //     }
    // }

    // // lazily update ErasVotes
    // fn update_vote_counts(contract: &T::AccountId, era: &EraIndex) {
    //     let current_era = T::EraFinder::current().unwrap_or(Zero::zero());
    //     if current_era < *era {
    //         return;
    //     }
    //     let mut untreated_era = Self::contracts_untreated_era(&contract);
    //     while *era > untreated_era {
    //         Self::propagate_eras_votes(&contract, &untreated_era, &(untreated_era + 1));
    //         untreated_era += 1;
    //     }
    //     <ContractsUntreatedEra<T>>::insert(&contract, untreated_era);
    // }

    // // convert Vote into VoteCounts
    // fn vote_counts(vote: Vote) -> VoteCounts {
    //     let mut counts = VoteCounts { bad: 0, good: 0 };
    //     if vote == Vote::Bad {
    //         counts.bad += 1
    //     } else {
    //         counts.good += 1
    //     };
    //     counts
    // }

    // // check the number of votes meets the required number of votes
    // fn has_votes_requirement(contract: &T::AccountId, era: &EraIndex) -> bool {
    //     let vote_counts = Self::eras_votes(era, contract);
    //     vote_counts.bad + vote_counts.good >= VOTES_REQUIREMENT
    // }

    // // If there are more than the required number of votes and
    // //   the good has been voted for four times the bad in two consecutive periods,
    // //   this function returns true.
    // fn is_rewardable(contract: &T::AccountId, era: &EraIndex) -> bool {
    //     Self::update_vote_counts(contract, era);
    //     if *era <= Zero::zero() {
    //         return false;
    //     }
    //     let prev_votes = Self::eras_votes(era - 1, contract);
    //     let votes = Self::eras_votes(era, contract);
    //     Self::has_votes_requirement(contract, &(era - 1))
    //         && Self::has_votes_requirement(contract, era)
    //         && prev_votes.good >= prev_votes.bad * 4
    //         && votes.good >= votes.bad * 4
    // }

    // // If there are more votes the required number of votes and
    // //   the good is less than twice the bad in the previous era,
    // //   this function returns true.
    // fn is_locked(contract: &T::AccountId, era: &EraIndex) -> bool {
    //     Self::update_vote_counts(contract, era);
    //     if *era <= Zero::zero() {
    //         return false;
    //     }
    //     let prev_votes = Self::eras_votes(era - 1, contract);
    //     Self::has_votes_requirement(contract, &(era - 1))
    //         && Self::has_votes_requirement(contract, era)
    //         && prev_votes.good < prev_votes.bad * 2
    // }

    // // If there are more votes than the required number of votes and
    // //   the good is less than the bad for two consecutive terms,
    // //   this function returns true.
    // fn is_slashable(contract: &T::AccountId, era: &EraIndex) -> bool {
    //     Self::update_vote_counts(contract, era);
    //     if *era <= Zero::zero() {
    //         return false;
    //     }
    //     let prev_votes = Self::eras_votes(era - 1, contract);
    //     let votes = Self::eras_votes(era, contract);
    //     Self::has_votes_requirement(contract, &(era - 1))
    //         && Self::has_votes_requirement(contract, era)
    //         && prev_votes.good < prev_votes.bad
    //         && votes.good < votes.bad
    // }
}
