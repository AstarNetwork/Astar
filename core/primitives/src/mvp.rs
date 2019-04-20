#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde_derive::{Serialize, Deserialize};

use sr_primitives::traits::{Zero, CheckedAdd, CheckedSub};
// use Encode, Decode
use parity_codec::{Encode, Decode};
use rstd::ops::{Deref, Div, Add, Sub};

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct Value(u64);

impl Value {
	pub fn new(a: u64) -> Self {
		Value(a)
	}
}

impl Div<usize> for Value {
	type Output = Value;
	fn div(self, rhs: usize) -> Self::Output {
		Value(*self / (rhs as u64))
	}
}

impl Zero for Value {
	fn zero() -> Self {
		Value(0)
	}

	fn is_zero(&self) -> bool {
		**self == 0
	}
}

impl Add for Value {
	type Output = Self;
	fn add(self, rhs: Value) -> Self::Output {
		Value(*self + *rhs)
	}
}

impl CheckedAdd for Value {
	fn checked_add(&self, v: &Self) -> Option<Self> {
		if let Some(v) = (**self).checked_add(**v) {
			return Some(Value(v));
		}
		None
	}
}

impl Sub for Value {
	type Output = Self;
	fn sub(self, rhs: Value) -> Self::Output {
		Value(*self - *rhs)
	}
}

impl CheckedSub for Value {
	fn checked_sub(&self, v: &Self) -> Option<Self> {
		if let Some(v) = (**self).checked_sub(**v) {
			return Some(Value(v));
		}
		None
	}
}

impl Deref for Value {
	type Target = u64;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mvp_value_add() {
		assert_eq!(300, *(Value(100) + Value(200)));
		assert_eq!((1 << 60), *(Value(1 << 59) + Value(1 << 59)));
	}

	#[test]
	fn mvp_value_checked_add() {
		assert_eq!(Some(Value(1000)), Value(500).checked_add(&(Value(300) + Value(200))));
		assert_eq!(None, Value(1 << 63).checked_add(&Value(1 << 63)));
	}

	#[test]
	fn mvp_value_sub() {
		assert_eq!(100, *(Value(200) - Value(100)));
		assert_eq!((1 << 58), *(Value(1 << 59) - Value(1 << 58)));
	}

	#[test]
	fn mvp_value_checked_sub() {
		assert_eq!(Some(Value(1000)), Value(2000).checked_sub(&(Value(300) + Value(700))));
		assert_eq!(None, Value(1 << 62).checked_sub(&Value(1 << 63)));
	}

	#[test]
	fn mvp_value_div() {
		assert_eq!(Value(5), Value(200) / (40 as usize));
		assert_eq!(Value(100), Value(2019) / (20 as usize));
	}

	#[test]
	fn mvp_value_zero() {
		assert_eq!(0, *Value::zero());
	}
}
