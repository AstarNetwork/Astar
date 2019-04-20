use sr_primitives::traits::{Zero, CheckedAdd, CheckedSub};
// use Encode, Decode
use parity_codec::{Encode, Decode};
use std::ops::{Deref, Div, Add, Sub};
use serde_derive::{Serialize, Deserialize};

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MVPValue(u64);

impl MVPValue {
	pub fn new(a: u64) -> Self {
		MVPValue(a)
	}
}

impl Div<usize> for MVPValue {
	type Output = MVPValue;
	fn div(self, rhs: usize) -> Self::Output {
		MVPValue(*self / (rhs as u64))
	}
}

impl Zero for MVPValue {
	fn zero() -> Self {
		MVPValue(0)
	}

	fn is_zero(&self) -> bool {
		**self == 0
	}
}

impl Add for MVPValue {
	type Output = Self;
	fn add(self, rhs: MVPValue) -> Self::Output {
		MVPValue(*self + *rhs)
	}
}

impl CheckedAdd for MVPValue {
	fn checked_add(&self, v: &Self) -> Option<Self> {
		if let Some(v) = (**self).checked_add(**v) {
			return Some(MVPValue(v));
		}
		None
	}
}

impl Sub for MVPValue {
	type Output = Self;
	fn sub(self, rhs: MVPValue) -> Self::Output {
		MVPValue(*self - *rhs)
	}
}

impl CheckedSub for MVPValue {
	fn checked_sub(&self, v: &Self) -> Option<Self> {
		if let Some(v) = (**self).checked_sub(**v) {
			return Some(MVPValue(v));
		}
		None
	}
}

impl Deref for MVPValue {
	type Target = u64;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
