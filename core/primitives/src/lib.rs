#![cfg_attr(not(feature = "std"), no_std)]

// primitives data structures.
pub mod mvp;

use parity_codec::Encode;
use rstd::prelude::*;

pub fn concat_bytes<T: Encode, U: Encode>(a: &T, b: &U) -> Vec<u8> {
	a.encode().iter().chain(b.encode().iter()).map(|x| *x).collect::<Vec<_>>()
}
