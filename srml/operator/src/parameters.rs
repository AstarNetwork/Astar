use super::*;
use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub trait Verifiable {
	fn verify(&self) -> Result;
}

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
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
