#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde_derive::{Serialize, Deserialize};

use sr_primitives::traits::{Verify, Zero, CheckedAdd, CheckedSub, Hash};
use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, Parameter};

use system::ensure_signed;

use rstd::prelude::*;
use rstd::marker::PhantomData;

#[cfg(feature = "std")]
pub use std::fmt;

// use Encode, Decode
use parity_codec::{Encode, Decode};
use rstd::ops::Div;

// plasm pritmitives uses mvp::Value
pub mod mvp;

pub trait Trait: system::Trait {
	type Signature: Verify<Signer=Self::AccountId>;
	type TimeLock: Parameter + Zero + Default;
	type Value: Parameter + From<Self::Value> + Zero + CheckedAdd + CheckedSub + Div<usize, Output=Self::Value> + Default;

	type Input: TransactionInputTrait<Self::Hash> + From<Self::Input>;
	type Output: Parameter + TransactionOutputTrait<Self::Value, Self::AccountId> + From<Self::Output> + Default;

	type Transaction: Parameter + TransactionTrait<Self::Input, Self::Output, Self::TimeLock> + Default;
	type SignedTransaction: Parameter + SignedTransactionTrait<Self>;

	type Inserter: InserterTrait<Self>;
	type Remover: RemoverTrait<Self>;
	type Finalizer: FinalizerTrait<Self>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

type CheckResult<T> = rstd::result::Result<T, &'static str>;

pub trait InserterTrait<T: Trait> {
	fn insert(tx: &T::Transaction) {
		Self::default_insert(tx);
	}
	fn default_insert(tx: &T::Transaction) {
		// new output is inserted to UTXO.
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, out) in tx.outputs()
			.iter()
			.enumerate() {
			let identify = (hash.clone().into(), i as u32);
			<UnspentOutputs<T>>::insert(identify.clone(), out.clone());
			for key in out.keys() {
				<UnspentOutputsFinder<T>>::mutate(key, |v| {
					match v.as_mut() {
						Some(vc) => vc.push(identify.clone()),
						None => *v = Some(vec! {identify.clone()}),
					}
				});
			}
		}
	}
}

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DefaultInserter<T: Trait>(PhantomData<T>);

impl<T: Trait> InserterTrait<T> for DefaultInserter<T> {}

pub trait RemoverTrait<T: Trait> {
	fn remove(tx: &T::Transaction) {
		Self::default_remove(tx);
	}

	fn default_remove(tx: &T::Transaction) {
		for inp in tx.inputs().iter() {
			for key in inp
				.output_or_default::<T>()
				.keys()
				.iter() {
				<UnspentOutputsFinder<T>>::mutate(key, |v| {
					*v = match
						v.as_ref()
							.unwrap_or(&vec! {})
							.iter()
							.filter(|e| **e != (inp.tx_hash(), inp.out_index()))
							.map(|e| *e)
							.collect::<Vec<_>>()
							.as_slice() {
						[] => None,
						s => Some(s.to_vec()),
					}
				})
			}
			<UnspentOutputs<T>>::remove((inp.tx_hash(), inp.out_index()));
		}
	}
}

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DefaultRemover<T: Trait>(PhantomData<T>);

impl<T: Trait> RemoverTrait<T> for DefaultRemover<T> {}

pub trait FinalizerTrait<T: Trait> {
	fn finalize(n: T::BlockNumber) {
		Self::default_finalize(n);
	}

	fn default_finalize(n: T::BlockNumber) {
		// TODO authorty SessionKey applied sr25519 or related AccountId.
//		let authorities = consensus::Module::<T>::authorities();
//		let leftover = <LeftoverTotal<T>>::take();
//
//		// send leftover to all authorities.
//		if authorities.len() == 0 { return; }
//		let shared_value = leftover / (authorities.len());
//		if shared_value == T::Value::zero() { return; }
//
//		// create UnspentTransactionOutput
//		let outs: Vec<_> = authorities.iter()
//			.map(|key|
//				T::Output::new(shared_value.clone(), vec! {<T as system::Trait>::AccountId::from(key.clone()), }, 1))
//			.collect();
//
//		// crate Transaction.
//		let tx = T::Transaction::new(vec! {}, outs.clone(), T::TimeLock::zero());
//		T::Inserter::insert(&tx);
	}
}

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DefaultFinalizer<T: Trait>(PhantomData<T>);

impl<T: Trait> FinalizerTrait<T> for DefaultFinalizer<T> {}

pub trait TransactionInputTrait<Hash> {
	fn new(tx_hash: Hash, out_index: u32) -> Self;
	fn output_or_default<T: Trait>(&self) -> T::Output
		where (Hash, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		match <UnspentOutputs<T>>::get((self.tx_hash(), self.out_index())) {
			Some(tx_out) => tx_out,
			None => Default::default(),
		}
	}
	fn output<T: Trait>(&self) -> Option<T::Output>
		where (Hash, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		<UnspentOutputs<T>>::get((self.tx_hash(), self.out_index()))
	}
	fn tx_hash(&self) -> Hash;
	fn out_index(&self) -> u32;
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct TransactionInput<Hash> {
	///#[codec(compact)]
	pub tx_hash: Hash,
	///#[codec(compact)]
	pub out_index: u32,
}

impl<Hash> TransactionInputTrait<Hash> for TransactionInput<Hash>
	where Hash: Clone {
	fn new(tx_hash: Hash, out_index: u32) -> Self {
		Self { tx_hash, out_index }
	}
	fn tx_hash(&self) -> Hash {
		self.tx_hash.clone()
	}
	fn out_index(&self) -> u32 {
		self.out_index
	}
}

pub trait TransactionOutputTrait<Value, Key> {
	fn new(value: Value, keys: Vec<Key>, quorum: u32) -> Self;
	fn value(&self) -> Value;
	fn keys(&self) -> &Vec<Key>;
	fn quorum(&self) -> u32;
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct TransactionOutput<Value, Key> {
	///#[codec(compact)]
	pub value: Value,
	///#[codec(compact)]
	pub keys: Vec<Key>,
	///#[codec(compact)]
	pub quorum: u32,
}

impl<Value: Clone, Key> TransactionOutputTrait<Value, Key> for TransactionOutput<Value, Key>
	where Value: Clone {
	fn new(value: Value, keys: Vec<Key>, quorum: u32) -> Self {
		Self { value, keys, quorum }
	}
	fn value(&self) -> Value {
		self.value.clone()
	}
	fn keys(&self) -> &Vec<Key> {
		&self.keys
	}
	fn quorum(&self) -> u32 {
		self.quorum
	}
}

pub trait TransactionTrait<Input, Output, TimeLock> {
	fn new(inputs: Vec<Input>, outputs: Vec<Output>, lock_time: TimeLock) -> Self;
	fn inputs(&self) -> &Vec<Input>;
	fn outputs(&self) -> &Vec<Output>;
	fn lock_time(&self) -> TimeLock;
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct Transaction<Input, Output, TimeLock> {
	///#[codec(compact)]
	pub inputs: Vec<Input>,
	///#[codec(compact)]
	pub outputs: Vec<Output>,
	///#[codec(compact)]
	pub lock_time: TimeLock,
}

impl<Input, Output, TimeLock> TransactionTrait<Input, Output, TimeLock> for Transaction<Input, Output, TimeLock>
	where TimeLock: Clone {
	fn new(inputs: Vec<Input>, outputs: Vec<Output>, lock_time: TimeLock) -> Self {
		Self { inputs, outputs, lock_time }
	}
	fn inputs(&self) -> &Vec<Input> {
		&self.inputs
	}
	fn outputs(&self) -> &Vec<Output> {
		&self.outputs
	}
	fn lock_time(&self) -> TimeLock {
		self.lock_time.clone()
	}
}

pub trait SignedTransactionTrait<T: Trait> {
	fn payload(&self) -> &Option<T::Transaction>;
	fn signatures(&self) -> &Vec<T::Signature>;
	fn public_keys(&self) -> &Vec<<T as system::Trait>::AccountId>;

	/// spent transaction
	fn spent(&self) {
		T::Remover::remove(self.payload().as_ref().expect("must be payload when spent."));
		T::Inserter::insert(self.payload().as_ref().expect("must be payload when spent."));
	}

	/// verify signatures
	fn verify(&self) -> Result {
		if let Some(tx) = self.payload() {
			let hash = <T as system::Trait>::Hashing::hash_of(tx);
			for (sign, key) in self.signatures().iter().zip(self.public_keys().iter()) {
				if !sign.verify(hash.clone().as_mut() as &[u8], key) {
					return Err("signature is unverified.");
				}
			}
			return Ok(());
		}
		Err("payload is None")
	}

	/// unlock inputs
	fn unlock(&self) -> Result {
		let keys: Vec<_> = self.public_keys().iter().collect();
		for input in self.payload()
			.as_ref()
			.expect("payload expects not None")
			.inputs()
			.iter() {
			let output = input.output::<T>()
				.ok_or("specified utxo by input is not found.")?;
			if output.quorum() > output
				.keys()
				.iter()
				.filter(|key|
					keys.contains(key))
				.count() as u32 {
				return Err("not enough public_keys to unlock all specified utxo.");
			}
		}
		Ok(())
	}

	// calculate leftover.
	fn leftover(&self) -> CheckResult<T::Value> {
		let sum_in: T::Value = self
			.payload()
			.as_ref()
			.expect("paylaod expected not None")
			.inputs()
			.iter()
			.try_fold(T::Value::zero(), |sum, inp| sum.checked_add(&T::Value::from(inp.output_or_default::<T>().value())))
			.ok_or("sum of inputs value is overflow")?;
		let sum_out: T::Value = self
			.payload()
			.as_ref()
			.expect("paylaod expected not None")
			.outputs()
			.iter()
			.try_fold(T::Value::zero(), |sum, out| sum.checked_add(&T::Value::from(out.value())))
			.ok_or("sum of outputs value is overflow")?;
		let leftover = sum_in.checked_sub(&sum_out).ok_or("leftover invalid (sum of input) - (sum of output)")?;
		Ok(leftover)
	}
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SignedTransaction<T: Trait> {
	///#[codec(compact)]
	pub payload: Option<T::Transaction>,
	///#[codec(compact)]
	pub signatures: Vec<T::Signature>,
	///#[codec(compact)]
	pub public_keys: Vec<<T as system::Trait>::AccountId>,
}

impl<T: Trait> SignedTransactionTrait<T> for SignedTransaction<T> {
	fn payload(&self) -> &Option<T::Transaction> {
		&self.payload
	}
	fn signatures(&self) -> &Vec<T::Signature> {
		&self.signatures
	}
	fn public_keys(&self) -> &Vec<<T as system::Trait>::AccountId> {
		&self.public_keys
	}
}

/// This module's storage items.
decl_storage! {
	trait Store for Module <T: Trait> as Utxo {
	/// All valid unspent transaction outputs are stored in this map.
	/// Initial set of UTXO is populated from the list stored in genesis.
		pub UnspentOutputs get(unspent_outputs) build( |config: &GenesisConfig<T>| {
			let tx = T::Transaction::new(vec!{},
				config.genesis_tx
					.clone()
					.iter()
					.map(|e| T::Output::new(e.0.clone(),vec!{e.1.clone()},1))
					.collect::<Vec<_>>(), T::TimeLock::zero());
			tx.clone()
				.outputs()
				.iter()
				.enumerate()
				.map(|(i, u)| ((<T as system::Trait>::Hashing::hash_of(&tx), i as u32), u.clone()))
				.collect::<Vec<_>>()
		}): map (<T as system::Trait>::Hash, u32) => Option<T::Output>;
		
		/// [AccountId] = reference of UTXO.
		pub UnspentOutputsFinder get(unspent_outputs_finder) build( |config: &GenesisConfig<T> | { // TODO more clearly
			let tx = T::Transaction::new(vec!{},
				config.genesis_tx
					.clone()
					.iter()
					.map(|e| T::Output::new(e.0.clone(),vec!{e.1.clone()},1))
					.collect::<Vec<_>>(), T::TimeLock::zero());
			config.genesis_tx
				.clone()
				.iter()
				.enumerate()
				.map(|(i, e)| (e.1.clone(), vec!{(<T as system::Trait>::Hashing::hash_of(&tx), i as u32)}))
				.collect::<Vec<_>>()
		}): map <T as system::Trait>::AccountId => Option<Vec<(<T as system::Trait>::Hash, u32)>>; //TODO HashSet<>
		
		/// Total leftover value to be redistributed among authorities.
		/// It is accumulated during block execution and then drained
		/// on block finalization.
		pub LeftoverTotal get(leftover_total): T::Value;
		
		/// Outputs that are locked
		pub LockedOutputs get(locked_outputs): map T::BlockNumber => Option <Vec<T::Output>>;
	}
	
	add_extra_genesis {
		config(genesis_tx): Vec<(T::Value, <T as system::Trait>::AccountId)>; // TODO Genesis should only use primitive.
	}
}

decl_module! {
	// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T> () = default;

		// Dispatch a single transaction and update UTXO set accordingly
		pub fn execute(origin, signed_tx: Vec<u8> ) -> Result {
			ensure_signed(origin)?;

			let signed_tx = T::SignedTransaction::decode( &mut &signed_tx[..]).ok_or("signed_tx is undecoded bytes.")?;
			// all signature checking Signature.Verify(HashableAccountId, hash(transaction.payload)).
			signed_tx.verify()?;
			// UTXO unlocked checking.
			signed_tx.unlock()?;
			// LeftOver(Fee) calclate.
			let leftover = signed_tx.leftover()?;

			// Calculate new leftover total
			let new_total = <LeftoverTotal<T>>::get()
			.checked_add(&leftover)
			.ok_or("leftover overflow")?;

			Self::update_storage(&signed_tx, new_total);
			Self::deposit_event(RawEvent::TransactionExecuted(signed_tx));
			Ok(())
		}

		// Handler called by the system on block finalization
		pub fn on_finalize(n: T::BlockNumber) {
			T::Finalizer::finalize(n);
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event <T> where SignedTransaction = <T as Trait>::SignedTransaction {
		/// Transaction was executed successfully
		TransactionExecuted(SignedTransaction),

	}
);

/// Not callable external
impl<T: Trait> Module<T> {
	/// Update storage to reflect changes made by transaction
	fn update_storage(signed_tx: &T::SignedTransaction, new_total: T::Value) {
		/// Storing updated leftover value
		<LeftoverTotal<T>>::put(new_total);

		/// Remove all used UTXO since they are now spent
		signed_tx.spent();
	}
}

#[cfg(test)]
mod tests;
