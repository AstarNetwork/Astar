pub mod mvp;

use srml_support::impl_outer_origin;
use sr_primitives::traits::{Verify, MaybeSerializeDebug};
use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result};
use system::ensure_signed;
use primitives::{Blake2Hasher, H256, Zero};

// use Vec<>
use rstd::prelude::*;
#[cfg(feature = "std")]
pub use std::fmt;
// use Encode, Decode
use parity_codec::{Encode, Decode};
use serde::{Serialize, Deserialize};
use crate::mvp::CheckInfo;

pub trait Trait: system::Trait {
	type SessionKey: Parameter + Default + MaybeSerializeDebug;
	type Signature: Verify;
	type Script;
	type Value: Zero;
	//加法について交換則、結合則、単位元(0)、逆元(-a)：可換群(加法群)
	type Outpoint: From<[u8; 36]>;
	type TimeLock;
	type CheckInfo;
	type TxIn: TransactionInput<T>;
	type TxOut: TransactionOutput<T>;
}

type CheckResult<T> = result::Result<T, &'static src>;

pub trait UTXOFinder<T: Trait> {
	find(T::OutPoint) -> Option < & T::TxOut >;
}

pub trait TransactionInput<T: Trait> {
	fn outpoint() -> T::OutPoint;
	fn output() -> Option<&T::TxOut> {
		<UnspentTransaction<T>>::get(self.outpoint);
	}
	fn value(&self) -> T::Value;
	fn verify(&self, T::Hash) -> CheckResult<T>;
}

pub trait TransactionOutput<T: Trait> {
	fn unlock(&self) -> CheckResult<T>;
	fn value(&self) -> T::Value;
}

pub trait Transaction<T: Trait> {
	fn inputs(&self) -> &Option<Vec<T::TxIn>>;
	fn outputs(&self) -> &Option<Vec<T::TxOut>>;
}

pub trait SignedTransaction<T: Trait, Tx: Transaction> {
	fn signatures(&self) -> &Vec<T::Signature>;
	fn payload(&self) -> &Tx<T>;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TxInput<T: Trait> {
	#[codec(compact)]
	pub sequence: Option<T::SequenceNumber>,
	#[codec(compact)]
	pub outpoint: T::Outpoint,
}

impl<T: Trait> TransactionInput for TxInput<T> {
	fn output() -> Option<&T::TxOut> {
		let tx_hash = self.outpoint.split()
		let tx_out_ind = self.outpont.split()
			< UnspentTransaction < T >> ::get(tx_hash, tx_out_ind)
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TxOutput<T: Trait> {
	#[codec(compact)]
	pub value: T::Value,
	#[codec(compact)]
	pub script: Option<T::Script>,
}

impl<T: Trait> TransactionOutput for TxOutput<T> {
	fn unlock(&self) -> CheckResult<T> {
		OK(T::CheckInfo())
	}
	fn value(&self) -> T::Value {
		self.value
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Tx<T: Trait> {
	#[codec(compact)]
	pub inputs: Vec<TxInput<T>>,
	#[codec(compact)]
	pub outputs: Vec<TxOutout<T>>,
	#[codec(compact)]
	pub lock_time: T::SequenceNumber,
}

pub struct SignedTx<T: Trait, Tx: Transaction> {
	#[codec(compact)]
	pub payload: Option<Tx<T>>,
	#[codec(compact)]
	pub signatures: Vec<T::Signature>,
}

impl<T: Trait> Transaction for Tx<T> {
	fn inputs(&self) -> &Vec<T::TxIn> {
		&self.inputs
	}
	fn outputs(&self) -> &Vec<T::TxOut> {
		&self.outputs
	}
}

impl<T: Trait, Tx: Transaction> SignedTransaction for SignedTx<T, Tx> {
	fn signatures(&self) -> &Vec<T::Signature> {
		&self.signatures
	}
	fn payload(&self) -> &Option<Tx<T>> {
		&self.paylaod
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
				.map(|u| (<T::Hashing as Hash>::hash(&u), u))
				.collect::<Vec<_>>()
		}): map T::Outpoint => Option<T::TxOut>;

		/// Total leftover value to be redistributed among authorities.
		/// It is accumulated during block execution and then drained
		/// on block finalization.
		LeftoverTotal: T::Value;

		/// Outputs that are locked
		LockedOutputs: map T::BlockNumber => Vec<T::TxOut>;
	}

	add_extra_genesis {
		config(initial_utxo): Vec<T::TxOut>;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		/// Dispatch a single transaction and update UTXO set accordingly
		pub fn execute(origin, #[codec(compact)] transaction: SignedTransaction<T, Tx: Transaction>) -> Result {
			ensure_inherent(origin)?;

			let leftover = match Self::check_transaction(&transaction)?

			Self::update_storage(&transaction.payload(), leftover)?;
			Self::deposit_event(Event::TransactionExecuted(transaction));
			Ok(())
		}

		/// Handler called by the system on block finalization
		pub fn on_finalise(origin, ) {
			let authorities: Vec<_> = consensus::authorities()
				.iter().map(|&a| a.into()).collect();
			Self::spend_leftover(&authorities);
		}
	}
}

/// Not callable external
impl<T: Trait> Module<T> {
	fn check_tx_leftover(transaction: &Transaction<T>) -> CheckResult<T::Value> {
		let sum_in: T::Value = transaction
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

	fn check_tx_unlocked(transaction: &SignedTransaction<T, Tx>) -> CheckResult<()> {
		for input in transaction.inputs() {
			match input.output() {
				Some(src) => src.verify(transaction.signatures())?,
				None => return Err("not find unspent output on input"),
			}
		}
		OK(())
	}

	fn check_tx_signature(transaction: &SignedTransaction<T, Tx>) -> CheckResult<()> {
		for sig in transaction.signatures() {
			sig.verify().ok_or("signature verified")?;
		}
		OK(())
	}

	fn check_transaction(transaction: &SignedTransaction<T, Tx>) -> CheckResult<T::Value> {
		// all signature checking Signature.Verify(SessionKey, hash(transaction.payload)).
		Self::check_tx_signature(transaction)?;
		// UTXO unlocked checking.
		Self::check_tx_unlocked(transaction)?;
		// LeftOver(Fee) calclate.
		Self::check_tx_leftover(transaction.payload())
	}

	/// Update storage to reflect changes made by transaction
	fn update_storage(transaction: &Transaction<T>, leftover: T::Value) -> Result {
		/// Calculate new leftover total
		let new_total = <LeftoverTotal<T>>::get()
			.checked_add(leftover)
			.ok_or("leftover overflow")?;

		/// Storing updated leftover value
		<LeftoverTotal<T>>::put(new_total);

		/// Remove all used UTXO since they are now spent
		for input in &transaction.inputs() {
			<UnspentOutputs<T>>::remove(input.outpoint());
		}

		/// Add new UTXO to be used by future transactions
		let hash = T::Hasing::hash(transaction.payload());
		for (i, output) in transaction.outputs().iter().enumerate() {
			let outpoint = T::Outpoint::from([hash.to_fixed_bytes(), i.to_be_bytes()].concat());
			<UnspentOutputs<T>>::insert(outpoint, output);
		}
		Ok(())
	}

	/// Redistribute combined leftover value evenly among authorities
	fn spend_leftover(authorities: &[H256]) {
		let leftover = <LeftoverTotal<T>>::take();
		let share_value = leftover / authorities.len() as Value;
		if share_value == 0 { return; }

		let outs: TxOutput<T> = vec! {};
		for authority in authorities {
			outs.push(
				TxOutput < T > {
					value: shared_value,
					script: * authority,
				}
			);
		}
		let tx = Tx < T > {
			inputs: None,
			outputs: outs,
			lock_time: T::TimeLock::zero(),
		};
		let hash = T::Hashing::hash(&tx);
		for out in outs {
			let outpoint = T::Outpoint::from([hash.to_fixed_bytes(), i.to_be_bytes()].concat());
			<UnspentOutputs<T>>::insert(outpoint, output);
		}
	}
}
