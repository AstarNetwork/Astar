//! This is explains plasm inflation models.
//! The staking has 2 kinds.
//!
//! 1. Validator Staking
//! 2. Dapps(Operator) Staking
//!
//! About each staking, this module computes issuing new tokens.

use super::*;
use sp_runtime::PerThing;
use sp_arithmetic::traits::BaseArithmetic;

/// The total payout to all operators and validators and their nominators per era.
///
/// Testnet(Until migrate NPoS) defined as such:
///     20% of total issue tokens per a year.
///
/// `era_duration` is expressed in millisecond.
pub fn compute_total_payout_test<N>(total_tokens: N, era_duration: u64) -> (N, N)
where
    N: BaseArithmetic + Clone + From<u32>,
{
    // Milliseconds per year for the Julian year (365.25 days).
    const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
    let portion =
        Perbill::from_rational_approximation(era_duration as u64, MILLISECONDS_PER_YEAR * 5);
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
        assert_eq!(
            super::compute_total_payout_test(100_000_000u64, YEAR).0,
            19_986_311
        );

        const DAY: u64 = 24 * 60 * 60 * 1000;
        assert_eq!(
            super::compute_total_payout_test(100_000_000u64, DAY).0,
            54_757
        );

        const SIX_HOURS: u64 = 6 * 60 * 60 * 1000;
        assert_eq!(
            super::compute_total_payout_test(100_000_000u64, SIX_HOURS).0,
            13_689
        );

        const HOUR: u64 = 60 * 60 * 1000;
        assert_eq!(
            super::compute_total_payout_test(100_000_000u64, HOUR).0,
            2_281
        );
    }
}
