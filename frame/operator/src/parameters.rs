use super::*;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub trait Verifiable {
	fn verify(&self) -> Result;
}

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct DefaultParameters {
	pub can_be_nominated: bool,
	pub option_expired: u128,
	pub option_p: u128,
}

impl Verifiable for DefaultParameters {
	fn verify(&self) -> Result {
		Ok(())
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for DefaultParameters {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{:?}", self)
	}
}

impl sp_std::fmt::Debug for DefaultParameters {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "{:?}", self)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}
