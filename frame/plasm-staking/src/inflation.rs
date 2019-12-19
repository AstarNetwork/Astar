//! This is explains plasm inflation models.
//! The staking has 2 kinds.
//!
//! 1. Validator Staking
//! 2. Dapps(Operator) Staking
//!
//! About each staking, this module computes issuing new tokens.

use super::*;
use sp_runtime::traits::SimpleArithmetic;


/// The total payout to all operators or validators (and their nominators) per era.
///
/// Testnet(Until migrate NPoS) defined as such:
///     10% of total issue tokens per a year.
///
/// `era_duration` is expressed in millisecond.
pub fn compute_total_payout_test<N>(
    total_tokens: N,
    era_duration: u64
) -> (N, N) where N: SimpleArithmetic + Clone {
    // Milliseconds per year for the Julian year (365.25 days).
    const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
    let portion = Perbill::from_rational_approximation(era_duration as u64, MILLISECONDS_PER_YEAR * 10);

    let payout = portion * total_tokens;
    (payout.clone(), payout)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_compute_total_payout_test() {
        const YEAR: u64 = 365 * 24 * 60 * 60 * 1000;
        // check maximum inflation.
        // not 10_000 due to rounding error.
        assert_eq!(super::compute_total_payout(100_000u64, YEAR).1, 10_000);

        const DAY: u64 = 24 * 60 * 60 * 1000;
        assert_eq!(super::compute_total_payout(100_000u64, DAY).0, 273);

        const SIX_HOURS: u64 = 6 * 60 * 60 * 1000;
        assert_eq!(super::compute_total_payout(100_000u64, SIX_HOURS).0, 68);

        const HOUR: u64 = 60 * 60 * 1000;
        assert_eq!(super::compute_total_payout(100_000u64, HOURS).0, 6);
    }
}
