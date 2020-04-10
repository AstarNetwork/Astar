use super::*;
use sp_arithmetic::traits::BaseArithmetic;

/// Get the amount of staking per Era in a module in the Plasm Network.
pub trait GetEraStakingAmount<EraIndex, Balance> {
    fn compute(era: &EraIndex) -> Balance;
}

/// The reward is allocated from the total supply of tokens,
/// the time for Era, the amount of staking for Security, and the amount of staking for Dapps.
pub trait ComputeTotalPayout {
    fn compute<N, M>(
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
    fn compute(current_era: EraIndex) -> Option<Vec<AccountId>>;
}

/// Get the era for validator and dapps staking module.
pub trait EraFinder<EraIndex, SessionIndex, Moment> {
    /// The current era index.
    ///
    /// This is the latest planned era, depending on how session module queues the validator
    /// set, it might be active or not.
    fn current() -> Option<EraIndex>;

    /// The active era information, it holds index and start.
    ///
    /// The active era is the era currently rewarded.
    /// Validator set of this era must be equal to `SessionInterface::validators`.
    fn active() -> Option<ActiveEraInfo<Moment>>;

    /// The session index at which the era start for the last `HISTORY_DEPTH` eras
    fn start_session_index(era: &EraIndex) -> Option<SessionIndex>;
}

/// Get the security rewards for validator module.
pub trait ForSecurityEraRewardFinder<Balance> {
    fn get(era: &EraIndex) -> Option<Balance>;
}

/// Get the dapps rewards for dapps staking module.
pub trait ForDappsEraRewardFinder<Balance> {
    fn get(era: &EraIndex) -> Option<Balance>;
}
