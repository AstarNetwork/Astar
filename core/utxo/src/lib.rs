use sr_primitives::traits::{Verify, MaybeSerializeDebug, Member, MaybeDisplay, Zero, CheckedAdd, CheckedSub, Hash};
use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, Parameter};
use system::{ensure_signed, ensure_inherent};
use primitives::{Blake2Hasher, H256};

// use Vec<>
use rstd::prelude::*;
#[cfg(feature = "std")]
pub use std::fmt;
pub use std::collections::HashSet;
// use Encode, Decode
use parity_codec::{Encode, Decode, Compact};
use serde::{Serialize, Deserialize};

pub trait Trait: consensus::Trait {
	type HashableSessionKey: Parameter + Default + MaybeSerializeDebug + std::hash::Hash;
	type Signature: Parameter + Verify;
	type Value: Parameter + Zero + CheckedAdd + CheckedSub + Sized + Div<Self::Value>;
	//加法について交換則、結合則、単位元(0)、逆元(-a)：可換群(加法群)
	type TimeLock: Parameter + Zero;
}

pub trait Div<T> {
	fn div(self, rhs: usize) -> T;
}

impl Div<u64> for u64 {
	fn div(self, rhs: usize) -> u64 {
		self / (rhs as u64)
	}
}

type CheckResult<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TransactionInput<T: Trait> {
	///#[codec(compact)]
	pub tx_hash: T::Hash,
	///#[codec(compact)]
	pub out_index: usize,
	// optional temp saved transaction output (save_tmp_out, get_tmp_out)
	//_temp_out: Option<TransactionOutput<Value, HashableSessionKey>> // TODO
}

impl<T: Trait> TransactionInput<T> {
	pub fn tx_hash(&self) -> T::Hash {
		self.tx_hash
	}
	pub fn out_index(&self) -> usize {
		self.out_index
	}
	pub fn value(&self) -> T::Value {
		match self.output() {
			Some(tx_out) => tx_out.value(),
			None => T::Value::zero(),
		}
	}

	pub fn output(&self) -> Option<TransactionOutput<T>> {
		<UnspentOutputs<T>>::get((self.tx_hash(), self.out_index()))
	}
	pub fn spent(&self) {
		<UnspentOutputs<T>>::remove((self.tx_hash(), self.out_index()))
	}
}


#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TransactionOutput<T: Trait> {
	///#[codec(compact)]
	pub value: T::Value,
	///#[codec(compact)]
	pub keys: Vec<T::HashableSessionKey>,
	///#[codec(compact)]
	pub quorum: u32,
}

impl<T: Trait> TransactionOutput<T> {
	pub fn value(&self) -> T::Value {
		self.value.clone()
	}
	pub fn keys(&self) -> &Vec<T::HashableSessionKey> {
		&self.keys
	}
	pub fn quorum(&self) -> u32 {
		self.quorum
	}
}

#[derive(Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Transaction<T: Trait> {
	///#[codec(compact)]
	pub inputs: Vec<TransactionInput<T>>,
	///#[codec(compact)]
	pub outputs: Vec<TransactionOutput<T>>,
	///#[codec(compact)]
	pub lock_time: T::TimeLock,
}

impl<T: Trait> Transaction<T> {
	pub fn inputs(&self) -> &Vec<TransactionInput<T>> {
		&self.inputs
	}
	pub fn outputs(&self) -> &Vec<TransactionOutput<T>> {
		&self.outputs
	}

	// calculate leftover.
	pub fn leftover(&self) -> CheckResult<T::Value> {
		let sum_in: T::Value = self
			.inputs()
			.iter()
			.try_fold(T::Value::zero(), |sum, inp| sum.checked_add(&inp.value()))
			.ok_or("sum of inputs value is overflow")?;
		let sum_out: T::Value = self
			.outputs()
			.iter()
			.try_fold(T::Value::zero(), |sum, out| sum.checked_add(&out.value()))
			.ok_or("sum of outputs value is overflow")?;
		let leftover = sum_in.checked_sub(&sum_out).ok_or("leftover invalid (sum of input) - (sum of output)")?;
		Ok(leftover)
	}

	// spent means changes UTXOs.
	pub fn spent(&self) {
		// output that is specified by input remove from UTXO.
		self.inputs()
			.iter()
			.inspect(|inp| inp.spent());

		// new output is inserted to UTXO.
		let hash = <T as system::Trait>::Hashing::hash_of(self);
		self.outputs()
			.iter()
			.enumerate()
			.inspect(|(i, out)|
				<UnspentOutputs<T>>::insert((hash.clone(), i.clone()), out.clone()));
	}
}

#[derive(Clone, Encode, Decode, Default)]
pub struct SignedTransaction<T: Trait> {
	///#[codec(compact)]
	pub payload: Option<Transaction<T>>,
	///#[codec(compact)]
	pub signatures: Vec<T::Signature>,
	///#[codec(compact)]
	pub public_keys: Vec<T::HashableSessionKey>,
}

impl<T: Trait> SignedTransaction<T> {
	fn payload(&self) -> Option<Transaction<T>> {
		self.payload
	}
	fn signatures(&self) -> &Vec<T::Signature> {
		&self.signatures
	}
	fn public_keys(&self) -> &Vec<T::HashableSessionKey> {
		&self.public_keys
	}

	// verify signatures
	pub fn verify(&self) -> Result {
		if let Some(tx) = self.payload() {
			let hash = <T as system::Trait>::Hashing::hash_of(&tx);
			for (sign, key) in self.signatures().iter().zip(self.public_keys().iter()) {
				// TODO
				//	let pubkey = <<C as Crypto>::Pair as Pair>::Public::from_string(uri).ok().or_else(||
				//				<C as Crypto>::Pair::from_string(uri, password).ok().map(|p| p.public())
				if !sign.verify(hash.clone().as_mut() as &[u8], key as &<T::Signature as Verify>::Signer) { Err("signature is unverified.") }
			}
			Ok(())
		}
		Err("payload is None")
	}

	// unlock inputs
	pub fn unlock(&self) -> Result {
		let keys: HashSet<_> = self.public_keys().iter().collect();
		for input in self.payload()
			.expect("payload is not None")
			.inputs() {
			let output = <UnspentOutputs<T>>::get((input.tx_hash(), input.out_index()))
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
		Ok(())
	}
}


/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Utxo {
		/// All valid unspent transaction outputs are stored in this map.
		/// Initial set of UTXO is populated from the list stored in genesis.
		pub UnspentOutputs get(unspent_outputs): map (<T as system::Trait>::Hash, usize) => Option<TransactionOutput<T>>;

		/// Total leftover value to be redistributed among authorities.
		/// It is accumulated during block execution and then drained
		/// on block finalization.
		pub LeftoverTotal get(leftover_total): T::Value;

		/// Outputs that are locked
		pub LockedOutputs get(locked_outputs): map T::BlockNumber => Option<Vec<TransactionOutput<T>>>;
	}

	add_extra_genesis {
		config(initial_utxo): Vec<TransactionOutput<T>>;
	}
}

decl_module! {
	// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		//fn deposit_event<T>() = default;

		// Dispatch a single transaction and update UTXO set accordingly
		pub fn execute(origin, signed_tx: Vec<u8>) -> Result {
			ensure_inherent(origin)?;

			// all signature checking Signature.Verify(HashableSessionKey, hash(transaction.payload)).
			signed_tx.verify()?;
			// UTXO unlocked checking.
			signed_tx.unlock()?;
			// LeftOver(Fee) calclate.
			let leftover = signed_tx
				.payload()
				.unwrap()
				.leftover()?;

			// Calculate new leftover total
			let new_total = <LeftoverTotal<T>>::get()
				.checked_add(leftover)
				.unwrap_or("leftover overflow")?;

			Self::update_storage(&signed_tx.payload(), leftover, new_total);
			//Self::deposit_event(Event::TransactionExecuted(signed_tx));
			Ok(())
		}

		// Handler called by the system on block finalization
		fn on_finalize(n: T::BlockNumber) {
			let authorities: Vec<_> = consensus::Module::<T>::authorities()
				.iter().map(|&a| a.into()).collect();
			Self::spend_leftover(&authorities);
		}
	}
}
//
//decl_event!(
//	/// An event in this module.
//	pub enum Event<T> where T = Trait {
//		/// Transaction was executed successfully
//		TransactionExecuted(SignedTransaction<T>),
//	}
//);

/// Not callable external
impl<T: Trait> Module<T> {
	/// Update storage to reflect changes made by transaction
	fn update_storage(transaction: &Transaction<T>, leftover: T::Value, new_total: T::Value) {
		/// Storing updated leftover value
		<LeftoverTotal<T>>::put(new_total);

		/// Remove all used UTXO since they are now spent
		transaction.spent();
	}

	/// Redistribute combined leftover value evenly among authorities
	fn spend_leftover(authorities: &[T::HashableSessionKey]) {
		let leftover = <LeftoverTotal<T>>::take();

		// send leftover to all authorities.
		let shared_value = leftover.div(authorities.len());
		if shared_value == 0 { return; }

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
			<UnspentOutputs<T>>::insert((hash.clone(), i.clone()), out.clone());
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use support::{impl_outer_origin, assert_ok};
	use sr_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header},
	};
	use primitives::ed25519;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;

	pub type Signature = ed25519::Signature;
	pub type SessionKey = <Signature as Verify>::Signer;

	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}

	impl consensus::Trait for Test {
		type Log = DigestItem;
		type SessionKey = SessionKey;
		type InherentOfflineReport = consensus::InstantFinalityReportVec<()>;
	}

	impl Trait for Test {
		type HashableSessionKey = SessionKey;
		type Signature = Signature;
		type Value = u64;
		type TimeLock = Self::BlockNumber;
	}

	type TemplateModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn it_works() {
		with_externalities(&mut new_test_ext(), || {
			unimplemented!()
		});
	}
}
