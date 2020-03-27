use super::*;
use sp_arithmetic::traits::BaseArithmetic;

/// Get the amount of staking per Era in a module in the Plasm Network.
pub trait GetEraStakingAmount<EraIndex, Balance> {
    fn get_era_staking_amount(era: EraIndex) -> Balance;
}

/// The reward is allocated from the total supply of tokens,
/// the time for Era, the amount of staking for Security, and the amount of staking for Dapps.
pub trait ComputeTotalPayout {
    fn compute_total_payout<N, M>(
        total_tokens: N,
        era_duration: M,
        validator_staking: N,
        dapps_staking: N,
    ) -> (N, N)
    where
        N: BaseArithmetic + Clone + From<u32>,
        M: BaseArithmetic + Clone + From<u32>;
}

/// Returns the next validator candidate.
pub trait MaybeValidators<EraIndex, AccountId> {
    fn maybe_validators(current_era: EraIndex) -> Option<Vec<AccountId>>;
}
