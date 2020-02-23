//! *Plasm Staking Parameters*
//!
//! Rewards_{option_{i,j}^{old}}=Rewards_{opeartor_{i}}\times \frac{stake_{i,j}^{old}}{\sum^{n^{old}}_{i,j}\sum^{m_i^{old}}_jstake_{i,j}^{old}}\times p^{old}_{operator_i}
//!
//! Used Perbil other parameters.
use codec::{Decode, Encode};
use operator::parameters::Verifiable;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{DispatchError, Perbill, PerThing};

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StakingParameters {
    /// If true, the operated contracts can be nominated else is can't.
    pub can_be_nominated: bool,
    /// Expired of that **option** can be exercised.
    pub option_expired: u128,
    /// For calculating option, **p**.
    pub option_p: u32,
}

impl Verifiable for StakingParameters {
    fn verify(&self) -> Result<(), DispatchError> {
        if self.option_p > Perbill::from_percent(20).deconstruct() {
            Err("**p** of option's parameters must be lower than 20%(0_200_000_000)")?
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::fmt::Display for StakingParameters {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl sp_std::fmt::Debug for StakingParameters {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "{:?}", self)
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}
