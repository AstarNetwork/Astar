//! This is explains plasm inflation models.
//! The staking has 2 kinds.
//!
//! 1. Validator Staking
//! 2. Dapps(Operator) Staking
//!
//! About each staking, this module computes issuing new tokens.

use super::*;
use num_traits::sign::Unsigned;
use sp_arithmetic::traits::BaseArithmetic;
use sp_std::marker::PhantomData;
use traits::ComputeTotalPayout;

pub struct FirstPlasmIncentive<N: BaseArithmetic + Clone + From<u32>> {
    _phantom: PhantomData<N>,
}

impl<L, D> ComputeTotalPayout<L, D> for FirstPlasmIncentive<L>
where
    L: BaseArithmetic + Clone + From<u32>,
{
    fn compute<N, M>(
        total_tokens: N,
        era_duration: M,
        number_of_validator: L,
        _dapps_staking: D,
    ) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
        M: BaseArithmetic + Clone + From<u32>,
    {
        const TARGETS_NUMBER: u128 = 100;
        const MILLISECONDS_PER_YEAR: u128 = 1000 * 3600 * 24 * 36525 / 100;
        // I_0 = 2.5%.
        const I_0_DENOMINATOR: u128 = 25;
        const I_0_NUMERATOR: u128 = 1000;
        let number_of_validator_clone: u128 = number_of_validator.clone().unique_saturated_into();
        let era_duration_clone: u128 = era_duration.clone().unique_saturated_into();
        let number_of_validator: u128 = number_of_validator.unique_saturated_into();
        let portion = if TARGETS_NUMBER < number_of_validator_clone {
            // TotalForSecurityRewards
            // = TotalAmountOfIssue * I_0% * (EraDuration / 1year)

            // denominator: I_0_DENOMINATOR * EraDuration
            // numerator: 1year * I_0_NUMERATOR
            Perbill::from_rational_approximation(
                I_0_DENOMINATOR * era_duration_clone,
                MILLISECONDS_PER_YEAR * I_0_NUMERATOR,
            )
        } else {
            // TotalForSecurityRewards
            // = TotalAmountOfIssue * I_0% * (NumberOfValidators/TargetsNumber) * (EraDuration/1year)

            // denominator: I_0_DENOMINATOR * NumberOfValidators * EraDuration
            // numerator: 1year * I_0_NUMERATOR * TargetsNumber
            Perbill::from_rational_approximation(
                I_0_DENOMINATOR * number_of_validator * era_duration_clone,
                MILLISECONDS_PER_YEAR * I_0_NUMERATOR * TARGETS_NUMBER,
            )
        };
        let payout = portion * total_tokens;
        (payout, N::zero())
    }
}

pub struct SimpleComputeTotalPayout;

/// The total payout to all operators and validators and their nominators per era.
///
/// Testnet(Until migrate NPoS) defined as such:
///     20% of total issue tokens per a year.
///
/// `era_duration` is expressed in millisecond.
impl<V, D> ComputeTotalPayout<V, D> for SimpleComputeTotalPayout {
    fn compute<N, M>(
        total_tokens: N,
        era_duration: M,
        _validator_staking: V,
        _dapps_staking: D,
    ) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
        M: BaseArithmetic + Clone + From<u32>,
    {
        // Milliseconds per year for the Julian year (365.25 days).
        const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
        let portion = Perbill::from_rational_approximation(
            era_duration.unique_saturated_into(),
            MILLISECONDS_PER_YEAR * 5,
        );
        let payout = portion * total_tokens;
        (payout.clone(), payout)
    }
}

pub struct MaintainRatioComputeTotalPayout<Balance>
where
    Balance: BaseArithmetic + Clone + From<u32>,
{
    _phantom: PhantomData<Balance>,
}

/// The total payout to all operators and validators and their nominators per era.
///
/// Testnet(Until migrate NPoS) defined as such:
///     20% of total issue tokens per a year.
/// Maintainn is Distribute rewards while maintaining a ratio of validator and dapps-compatible staking amounts.
///
/// `era_duration` is expressed in millisecond.
impl<Balance> ComputeTotalPayout<Balance, Balance> for MaintainRatioComputeTotalPayout<Balance>
where
    Balance: BaseArithmetic + Unsigned + Clone + From<u32>,
{
    fn compute<N, M>(
        total_tokens: N,
        era_duration: M,
        validator_staking: Balance,
        dapps_staking: Balance,
    ) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
        M: BaseArithmetic + Clone + From<u32>,
    {
        // Milliseconds per year for the Julian year (365.25 days).
        const MILLISECONDS_PER_YEAR: u64 = 1000 * 60 * 60 * 24 * 36525 / 100;
        let portion = Perbill::from_rational_approximation(
            era_duration.unique_saturated_into(),
            MILLISECONDS_PER_YEAR * 5,
        );
        let payout = portion * total_tokens;
        if validator_staking == Balance::zero() {
            if dapps_staking == Balance::zero() {
                return (N::zero(), N::zero());
            }
            return (N::zero(), payout);
        }
        let validator_portion = Perbill::from_rational_approximation(
            validator_staking.clone(),
            validator_staking + dapps_staking,
        );
        let validator_payout = validator_portion * payout.clone();
        let dapps_payout = payout - validator_payout.clone();
        (validator_payout, dapps_payout)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn compute_test<N>(
        total_tokens: N,
        era_duration: u64,
        validator_staking: N,
        dapps_staking: N,
    ) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        SimpleComputeTotalPayout::compute(
            total_tokens,
            era_duration,
            validator_staking,
            dapps_staking,
        )
    }

    fn compute_maintain_total_payout_test<N>(
        total_tokens: N,
        era_duration: u64,
        validator_staking: N,
        dapps_staking: N,
    ) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        MaintainRatioComputeTotalPayout::<N>::compute(
            total_tokens,
            era_duration,
            validator_staking,
            dapps_staking,
        )
    }

    fn compute_first_rewards_test<N>(
        total_tokens: N,
        era_duration: u64,
        number_of_validator: u32,
    ) -> (N, N)
    where
        N: BaseArithmetic + Unsigned + Clone + From<u32>,
    {
        FirstPlasmIncentive::<u32>::compute(total_tokens, era_duration, number_of_validator, 0)
    }

    #[test]
    fn test_compute_test() {
        const YEAR: u64 = 365 * 24 * 60 * 60 * 1000;
        // check maximum inflation.
        // not 10_000 due to rounding error.
        assert_eq!(compute_test(100_000_000u64, YEAR, 0, 0).0, 19_986_311);

        const DAY: u64 = 24 * 60 * 60 * 1000;
        assert_eq!(compute_test(100_000_000u64, DAY, 0, 0).0, 54_757);

        const SIX_HOURS: u64 = 6 * 60 * 60 * 1000;
        assert_eq!(compute_test(100_000_000u64, SIX_HOURS, 0, 0).0, 13_689);

        const HOUR: u64 = 60 * 60 * 1000;
        assert_eq!(compute_test(100_000_000u64, HOUR, 0, 0).0, 2_281);
    }

    #[test]
    fn test_maintain_compute_test() {
        const YEAR: u64 = 365 * 24 * 60 * 60 * 1000;
        // check maximum inflation.
        // not 10_000 due to rounding error.
        assert_eq!(
            compute_maintain_total_payout_test(100_000_000u64, YEAR, 90, 10),
            (17_987_680, 1_998_631)
        );

        assert_eq!(
            compute_maintain_total_payout_test(100_000_000u64, YEAR, 70, 30),
            (13_990_418, 599_5893)
        );

        assert_eq!(
            compute_maintain_total_payout_test(100_000_000u64, YEAR, 50, 50),
            (9_993_155, 9_993_156)
        );

        assert_eq!(
            compute_maintain_total_payout_test(100_000_000u64, YEAR, 10, 90),
            (1_998_631, 17_987_680)
        );

        assert_eq!(
            compute_maintain_total_payout_test(100_000_000u64, YEAR, 0, 100),
            (0, 19_986_311)
        );

        assert_eq!(
            compute_maintain_total_payout_test(100_000_000u64, YEAR, 100, 0),
            (19_986_311, 0)
        );

        assert_eq!(
            compute_maintain_total_payout_test(100_000_000u64, YEAR, 0, 0),
            (0, 0)
        );
    }

    #[test]
    fn test_first_rewards_compute() {
        const YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;
        assert_eq!(
            compute_first_rewards_test(100_000_000u64, YEAR, 100),
            (2_500_000, 0)
        );

        assert_eq!(
            compute_first_rewards_test(100_000_000u64, YEAR, 150),
            (2_500_000, 0)
        );

        assert_eq!(
            compute_first_rewards_test(100_000_000u64, YEAR, 50),
            (1_250_000, 0)
        );

        assert_eq!(
            compute_first_rewards_test(100_000_000u64, YEAR / 365, 100),
            (2_500_000 / 365, 0)
        );
    }
}
