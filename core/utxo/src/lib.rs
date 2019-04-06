pub mod mvp;

use srml_support::impl_outer_origin;
use sr_primitives::traits::{Verify,MaybeSerializeDebug};

pub trait UTXOValidator {
	fn validate<Key, Sig as Verify>(keys: &Vec<Key>, sigs: &Vec<Sig>) -> bool;
}

pub trait Trait: system::Trait {
	type SessionKey: Parameter + Default + MaybeSerializeDebug;
	type Signature: Verify;
	type Script: UTXOValidator + Default;
	type SequenceNumber;
	type Outpoint;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TxInput<T: Trait> {
	pub signatures: Vec<T::Signature>,
	pub sequence: Option<T::SequenceNumber>,
	pub outpoint: T::Outpoint,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TxOutput<T: Trait> {
	pub balance: T::Balance,
	pub script: Option<T::Script>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tx<T: Trait> {
	pub inputs: Vec<TxInput<T>>,
	pub outputs: Vec<TxOutout<T>>,
}

impl_outer_origin! {
	pub enum Origin for MockTrait {}
}

impl Trait for MockTrait {
	type SessionKey = node_primitives::AuthorityId;
	type Signature = node_primitives::Signature;
	type Script = ();
	type SequenceNumber = ();
	type Outpoint = [u8; 36];
}
