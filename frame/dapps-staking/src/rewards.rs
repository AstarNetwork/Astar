//! This is explains plasm inflation models.
//! The staking has 2 kinds.
//!
//! 1. Validator Staking
//! 2. Dapps(Operator) Staking
//!
//! About each staking, this module computes issuing new tokens.

use super::*;
use sp_arithmetic::traits::BaseArithmetic;

/// Compute reards for dapps from total dapps rewards to operators and nominators.
pub trait ComputeRewardsForDapps {
    fn compute_rewards_for_dapps<N>(total_dapps_rewards: N) -> (N, N)
    where
        N: BaseArithmetic + Clone + From<u32>;
}

/// The based compute rewards for dapps.
/// Following of https://docs.plasmnet.io/learn/token-economy#inflation-model, `t = 4`.
pub struct BasedComputeRewardsForDapps;

impl ComputeRewardsForDapps for BasedComputeRewardsForDapps {
    fn compute_rewards_for_dapps<N>(total_dapps_rewards: N) -> (N, N)
    where
        N: BaseArithmetic + Clone + From<u32>,
    {
        let operators_reward =
            Perbill::from_rational_approximation(N::from(4 as u32), N::from(5 as u32))
                * total_dapps_rewards.clone();
        let nominators_reward = total_dapps_rewards
            .checked_sub(&operators_reward)
            .unwrap_or(N::zero());
        (operators_reward, nominators_reward)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn compute_payout_test<N>(total_dapps_tokens: N) -> (N, N)
    where
        N: BaseArithmetic + Clone + From<u32>,
    {
        BasedComputeRewardsForDapps::compute_rewards_for_dapps(total_dapps_tokens)
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
}
