#![cfg_attr(not(feature = "std"), no_std)]

use support::dispatch::Result;
use rstd::prelude::*;

#[macro_use]
pub mod mvp;

type CheckResult<T> = rstd::result::Result<T, &'static str>;

pub trait WritableUtxoTrait<SignedTx, AccountId, OutPoint> {
	fn push(tx: SignedTx);
	fn spent(tx: &SignedTx);
	fn remove(out_point: &OutPoint);
	fn remove_finder(who: &AccountId, out_point: &OutPoint);
	fn deal(whoes: &Vec<AccountId>);
}

pub trait ReadbleUtxoTrait<SignedTx, V> {
	fn verify(tx: &SignedTx) -> Result;
	fn unlock(tx: &SignedTx) -> Result;
	fn leftover(signed_tx: &SignedTx) -> CheckResult<V>;
}

pub trait UtxoTrait<SignedTx, AccountId, OutPoint, V>: WritableUtxoTrait<SignedTx, AccountId, OutPoint> + ReadbleUtxoTrait<SignedTx, V> {
	fn exec(tx: SignedTx) -> Result;
}
