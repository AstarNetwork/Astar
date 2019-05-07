#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use support::dispatch::Result;

use rstd::prelude::*;

pub mod mvp;

type CheckResult<T> = rstd::result::Result<T, &'static str>;

pub trait WritableUtxoTrait<SignedTx> {
	fn push(tx: SignedTx);
}

pub trait ReadbleUtxoTrait<SignedTx, V> {
	fn verify(tx: &SignedTx) -> Result;
	fn unlock(tx: &SignedTx) -> Result;
	fn leftover(signed_tx: &SignedTx) -> CheckResult<V>;
}

pub trait UtxoTrait<SignedTx, V>: WritableUtxoTrait<SignedTx> + ReadbleUtxoTrait<SignedTx, V> {
	fn exec(tx: SignedTx) -> Result;
}

#[cfg(tests)]
pub mod tests;
