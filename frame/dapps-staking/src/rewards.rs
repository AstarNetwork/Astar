//! This is explains plasm inflation models.
//! The staking has 2 kinds.
//!
//! 1. Validator Staking
//! 2. Dapps(Operator) Staking
//!
//! About each staking, this module computes issuing new tokens.

use super::*;
use log::log2;
use num_traits::sign::Unsigned;
use sp_arithmetic::traits::{BaseArithmetic, SaturatedConversion};

/// Compute reards for dapps from total dapps rewards to operators and nominators.
pub trait ComputeRewardsForDapps {
    fn compute_rewards_for_dapps<N>(total_dapps_rewards: N) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>;

    fn compute_reward_for_nominator<N>(
        nominate_total: N,
        total_staked: N,
        nominators_reward: N,
        staked_values: Vec<(N, N)>,
    ) -> N
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>;

    fn compute_reward_for_operator<N>(
        staked_operator: N,
        total_staked: N,
        operators_reward: N,
    ) -> N
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>;
}

/// The based compute rewards for dapps.
/// Following of https://docs.plasmnet.io/learn/token-economy#inflation-model, `t = 4`.
pub struct BasedComputeRewardsForDapps;

impl ComputeRewardsForDapps for BasedComputeRewardsForDapps {
    fn compute_rewards_for_dapps<N>(total_dapps_rewards: N) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        let operators_reward =
            Perbill::from_rational_approximation(N::from(4 as u32), N::from(5 as u32))
                * total_dapps_rewards.clone();
        let nominators_reward = total_dapps_rewards
            .checked_sub(&operators_reward)
            .unwrap_or(N::zero());
        (operators_reward, nominators_reward)
    }

    fn compute_reward_for_nominator<N>(
        nominate_total: N,
        total_staked: N,
        nominators_reward: N,
        _: Vec<(N, N)>,
    ) -> N
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        Perbill::from_rational_approximation(nominate_total, total_staked) * nominators_reward
    }

    fn compute_reward_for_operator<N>(staked_operator: N, total_staked: N, operators_reward: N) -> N
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        Perbill::from_rational_approximation(staked_operator, total_staked) * operators_reward
    }
}

pub struct VoidableRewardsForDapps;

impl ComputeRewardsForDapps for VoidableRewardsForDapps {
    /// distribute dapps rewards into 50% to operators and the other 50% to nominators
    fn compute_rewards_for_dapps<N>(total_dapps_rewards: N) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        let operators_reward =
            Perbill::from_rational_approximation(N::from(1 as u32), N::from(2 as u32))
                * total_dapps_rewards.clone();
        let nominators_reward = total_dapps_rewards
            .checked_sub(&operators_reward)
            .unwrap_or(N::zero());
        (operators_reward, nominators_reward)
    }

    /// Stakings that are less than 3% of total staking are ignored.
    /// Nominators get paid according to the value of each staking value multiplied by the scaling value
    /// Each scaling value is decided like the more staked contract becomes the fewer value.
    ///
    /// If you stake against contract A, which accounts for 10% of the total staking volume,
    /// the scaling value alpha will be -1 * log(10/100)
    /// In addition, multiply the staking value by the coefficient beta (= 0.197).
    /// This is necessary to make alpha*beta closer to 1.0 when the ratio is 3%.
    fn compute_reward_for_nominator<N>(
        _nominate_total: N,
        total_staked: N,
        nominators_reward: N,
        staked_values: Vec<(N, N)>,
    ) -> N
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        let threshold = total_staked.clone() * N::from(3 as u32) / N::from(100 as u32);

        let weighted_staking_total = staked_values
            .iter()
            .filter(|(total, _value)| threshold <= total.clone())
            .fold(N::from(0 as u32), |sum, (total, value)| {
                // -1 * log2(p/q) = log2(q/p)
                let alpha = N::from(log2(
                    total_staked.clone().saturated_into::<u32>(),
                    total.clone().saturated_into::<u32>(),
                ));
                sum + (value.clone().saturating_mul(alpha) / N::from(1_000 as u32))
                    .saturating_mul(N::from(197 as u32))
            });
        Perbill::from_rational_approximation(weighted_staking_total, total_staked)
            * N::from(1 as u32)
            * nominators_reward
    }

    fn compute_reward_for_operator<N>(staked_operator: N, total_staked: N, operators_reward: N) -> N
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        Perbill::from_rational_approximation(staked_operator, total_staked) * operators_reward
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn compute_payout_test<N>(total_dapps_tokens: N) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        BasedComputeRewardsForDapps::compute_rewards_for_dapps(total_dapps_tokens)
    }

    fn compute_voidable_rewards_payout<N>(total_dapps_tokens: N) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        VoidableRewardsForDapps::compute_rewards_for_dapps(total_dapps_tokens)
    }

    #[test]
    fn test_compute_payout_test() {
        assert_eq!(
            compute_payout_test(100_000_000u64),
            (80_000_000, 20_000_000)
        );

        assert_eq!(compute_payout_test(10_000_000u64), (8_000_000, 2_000_000));

        assert_eq!(compute_payout_test(11_111_111u64), (8_888_889, 2_222_222));
    }

    #[test]
    fn test_compute_voidable_rewards_payout() {
        assert_eq!(
            compute_voidable_rewards_payout(100_000_000u64),
            (50_000_000, 50_000_000)
        );

        assert_eq!(
            compute_voidable_rewards_payout(10_000_000u64),
            (5_000_000, 5_000_000)
        );

        assert_eq!(
            compute_voidable_rewards_payout(11_111_111u64),
            (5_555_555, 5_555_556)
        );
    }
}
