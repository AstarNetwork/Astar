use super::*;
use sp_arithmetic::traits::BaseArithmetic;
use sp_std::marker::PhantomData;

/// Get the amount of staking per Era in a module in the Plasm Network.
pub trait ComputeEraWithParam<EraIndex> {
    type Param;
    fn compute(era: &EraIndex) -> Self::Param;
}

pub struct DefaultForDappsStaking<T: Trait> {
    _phantom: PhantomData<T>,
}
impl<T: Trait> ComputeEraWithParam<EraIndex> for DefaultForDappsStaking<T> {
    type Param = BalanceOf<T>;
    fn compute(era: &EraIndex) -> BalanceOf<T> {
        0.into()
    }
}

/// The reward is allocated from the total supply of tokens,
/// the time for Era, the amount of staking for Security, and the amount of staking for Dapps.
pub trait ComputeTotalPayout<ValidatorParam, DappsParam> {
    fn compute<N, M>(
        total_tokens: N,
        era_duration: M,
        for_security_parm: ValidatorParam,
        for_dapps_param: DappsParam,
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

/// Get the history depth
pub trait HistoryDepthFinder {
    fn get() -> u32;
}
