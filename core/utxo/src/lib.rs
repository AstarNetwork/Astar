pub mod mvp;

use srml_support::impl_outer_origin;
use sr_primitives::traits::{Verify, MaybeSerializeDebug, Member, MaybeDisplay};
use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, StorageDoubleMap, dispatch::Result};
use system::ensure_signed;
use primitives::{Blake2Hasher, H256, Zero};

// use Vec<>
use rstd::prelude::*;
#[cfg(feature = "std")]
pub use std::fmt;
pub use std::collections::HashSet;
// use Encode, Decode
use parity_codec::{Encode, Decode};
use serde::{Serialize, Deserialize};
use crate::mvp::CheckInfo;

pub trait Trait: consensus::Trait {
	type Signature: Verify;
	type Value: Zero;
	//加法について交換則、結合則、単位元(0)、逆元(-a)：可換群(加法群)
	type TimeLock: Zero;
}

type CheckResult<T> = std::result::Result<T, &'static src>;

#[derive(Clone, Encode, Decode, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TransactionInput<Hash> {
	#[codec(compact)]
	pub tx_hash: Hash,
	#[codec(compact)]
	pub out_index: usize,
	// optional temp saved transaction output (save_tmp_out, get_tmp_out)
	//_temp_out: Option<TransactionOutput<Value, SessionKey>> // TODO
}

type TxIn<T: Trait> = TransactionInput<T::Hash>;

impl<T: Trait> TxIn<T> {
	pub fn tx_hash(&self) -> T::Hash {
		self.tx_hash
	}
	pub fn out_index(&self) -> usize {
		self.out_index
	}
	pub fn value(&self) -> value {
		match self.output() {
			Some(tx_out) => tx_out.value(),
			None => T::Value::zero(),
		}
	}

	pub fn output(&self) -> Option<TxOut<T>> {
		<UnspentOutputs<T>>::get(input.tx_hash(), input.out_index())
	}
	pub fn spent(&self) {
		<UnspentOutputs<T>>::remove(input.tx_hash(), input.out_index())
	}
}


#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TransactionOutput<Value, SessionKey> {
	#[codec(compact)]
	pub value: Value,
	#[codec(compact)]
	pub keys: Vec<SessionKey>,
	#[codec(compact)]
	pub quorum: u32,
}

type TxOut<T: Trait> = TransactionOutput<T::Value, T::SessionKey>;

impl<T: Trait> TxOut<T> {
	pub fn value(&self) -> T::Value {
		self.value
	}
	pub fn keys(&self) -> Vec<T::SessionKey> {
		self.keys
	}
	pub fn quorum(&self) -> u32 {
		self.quorum
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Transaction<TxIn, TxOut, Time> {
	#[codec(compact)]
	pub inputs: Vec<TxIn>,
	#[codec(compact)]
	pub outputs: Vec<TxOut>,
	#[codec(compact)]
	pub lock_time: Time,
}


type Tx<T: Trait> = Transaction<TxIn<T>, TxOut<T>, T::TimeLock>;

impl<T: Trait> Tx<T> {
	pub fn inputs(&self) -> Vec<TxIn<T>> {
		self.inputs
	}

	// calculate leftover.
	pub fn leftover(&self) -> CheckResult<T::Value> {
		let sum_in: T::Value = self
			.inputs()
			.iter()
			.try_fold(T::Value::zero(), |sum, inp| sum.checked_add(inp.value()))
			.ok_or("sum of inputs value is overflow");
		let sum_out: T::Value = transaction
			.outputs()
			.iter()
			.try_fold(T::Value::zero(), |sum, out| sum.checked_add(out.value()))
			.ok_or("sum of outputs value is overflow");
		let leftover = sum_in.checked_sub(sum_out).ok_or("leftover invalid (sum of input) - (sum of output)")?;
		OK(leftover)
	}

	// spent means changes UTXOs.
	pub fn spent(&self) {
		// output that is specified by input remove from UTXO.
		self.inputs()
			.iter()
			.inspect(|inp| inp.spent());

		// new output is inserted to UTXO.
		let hash = T::Hashing::hash_of(self);
		self.outputs()
			.iter()
			.enumerate()
			.inspect(|(i,out)|
				<UnspentOutputs<T>>::insert(hash, i, out))
	}
}

pub struct SignedTransaction<Tx, Signature, SessionKey> {
	#[codec(compact)]
	pub payload: Option<Tx>,
	#[codec(compact)]
	pub signatures: Vec<Signature>,
	#[codec(compact)]
	pub public_keys: Vec<SessionKey>,
}

type SignedTx<T: Trait> = SignedTransaction<Tx<T>, T::Signature, T::SessionKey>;

impl<T: Trait> SignedTx<T> {
	fn payload(&self) -> Option<Tx<T>> {
		self.payload
	}
	fn signatures(&self) -> Vec<T::Signature> {
		self.signatures
	}
	fn public_keys(&self) -> Vec<T::SessionKey> {
		self.public_keys
	}

	// verify signatures
	pub fn verify(&self) -> Result {
		if let Some(tx) = self.payload() {
			let hash = T::Hashing::hash_of(tx);
			for Some(sign, key) in self.signatures().iter().zip(self.public_keys().iter()) {
				sign.verify(hash, key)
			}
			OK(())
		}
		Err("payload is None")
	}

	// unlock inputs
	pub fn unlock(&self) -> Result {
		let keys: HashSet<_> = self.public_keys().iter().collect();
		for input in self.payload()
			.expect("payload is not None")
			.inputs() {
			let output = <UnspentOutputs<T>>::get(input.tx_hash(), input.out_index())
				.unwrap_or("specified utxo by input is not found.")?;
			if output.quorum() > output
				.keys()
				.iter()
				.filter(|key|
					keys.get(key).is_some())
				.count() {
				Err("not enough public_keys to unlock all specified utxo.")
			}
		}
		OK(())
	}
}


/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Utxo {
		/// All valid unspent transaction outputs are stored in this map.
		/// Initial set of UTXO is populated from the list stored in genesis.
		UnspentOutputs build(|config: &GenesisConfig<T>| {
			config.initial_utxo
				.iter()
				.cloned()
				.enumerate()
				.map(|(i,u)| (<T::Hashing as Hash>::hash_of(&u), i, u))
				.collect::<Vec<_>>()
		}): double_map T::Hash, usize => Option<TxOut<T>>;

		/// Total leftover value to be redistributed among authorities.
		/// It is accumulated during block execution and then drained
		/// on block finalization.
		LeftoverTotal: T::Value;

		/// Outputs that are locked
		LockedOutputs: map T::BlockNumber => Vec<TxOut<T>>;
	}

	add_extra_genesis {
		config(initial_utxo): Vec<TransactionOutput>;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		/// Dispatch a single transaction and update UTXO set accordingly
		pub fn execute(origin, #[codec(compact)] signed_tx: SignedTx<T>) -> Result {
			ensure_inherent(origin)?;

			// all signature checking Signature.Verify(SessionKey, hash(transaction.payload)).
			signed_tx.verify()?;
			// UTXO unlocked checking.
			signed_tx.unlock()?;
			// LeftOver(Fee) calclate.
			let leftover = signed_tx
				.payload()
				.unwrap()
				.leftover()?

			/// Calculate new leftover total
			let new_total = <LeftoverTotal<T>>::get()
				.checked_add(leftover)
				.ok_or("leftover overflow")?;

			Self::update_storage(&transaction.payload(), leftover, new_total)?;
			Self::deposit_event(Event::TransactionExecuted(transaction));
			Ok(())
		}

		/// Handler called by the system on block finalization
		pub fn on_finalise(n: T::BlockNumber) {
			let authorities: Vec<_> = consensus::authorities()
				.iter().map(|&a| a.into()).collect();
			Self::spend_leftover(&authorities);
		}
	}
}

/// Not callable external
impl<T: Trait> Module<T> {
	/// Update storage to reflect changes made by transaction
	fn update_storage(transaction: &Tx<T>, leftover: T::Value, new_total: T::Value) {
		/// Storing updated leftover value
		<LeftoverTotal<T>>::put(new_total);

		/// Remove all used UTXO since they are now spent
		transaction.spent();
	}

	/// Redistribute combined leftover value evenly among authorities
	fn spend_leftover(authorities: &[T::SessionKey]) {
		let leftover = <LeftoverTotal<T>>::take();

		// send leftover to all authorities.
		let share_value = leftover / authorities.len() as Value;
		if share_value == 0 { return; }

		// create UnspentTransactionOutput
		let outs: Vec<_> = authorities.iter()
			.map(|key|
				TransactionOutput {
					value: shared_value,
					keys: vec! {*authorities},
					quorum: 1,
				})
			.collect();

		// crate Transaction for calc hash
		let tx = Transaction {
			inputs: None,
			outputs: outs,
			lock_time: T::TimeLock::zero(),
		};
		let hash = T::Hashing::hash_of(&tx);

		// UnspentOutputs[hash][i] = unspentOutput
		for (i, out) in outs.iter().enumerate() {
			<UnspentOutputs<T>>::insert(hash, i, output);
		}
	}
}
