use super::*;

/// Get the era for validator and dapps staking module.
pub trait EraFinder<EraIndex, SessionIndex> {
    /// The current era index.
    ///
    /// This is the latest planned era, depending on how session module queues the validator
    /// set, it might be active or not.
    fn current() -> Option<EraIndex>;

    /// The active era information, it holds index and start.
    ///
    /// The active era is the era currently rewarded.
    /// Validator set of this era must be equal to `SessionInterface::validators`.
    fn active() -> Option<ActiveEraInfo>;

    /// The session index at which the era start for the last `HISTORY_DEPTH` eras
    fn start_session_index(era: &EraIndex) -> Option<SessionIndex>;
}

/// Get the security rewards for validator module.
pub trait ForSecurityEraRewardFinder<Balance> {
    fn get(era: &EraIndex) -> Option<Balance>;

    fn validator_count() -> u32;
}

/// Get the dapps rewards for dapps staking module.
pub trait ForDappsEraRewardFinder<Balance> {
    fn get(era: &EraIndex) -> Option<Balance>;
}

/// Get the history depth
pub trait HistoryDepthFinder {
    fn get() -> u32;
}
