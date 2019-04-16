mod mvp;

use sr_primitives::traits::{Verify, Zero, CheckedAdd, CheckedSub, Hash};
use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, Parameter};
use system::ensure_signed;

use rstd::prelude::*;
use rstd::marker::PhantomData;

#[cfg(feature = "std")]
pub use std::fmt;
pub use std::collections::HashMap;
// use Encode, Decode
use parity_codec::{Encode, Decode, Codec};
use std::ops::Div;

// mvp
pub use mvp::MVPValue;

pub trait Trait: consensus::Trait + Default {
	type Signature: Parameter + Verify<Signer=Self::SessionKey>;
	type Value: Parameter + Zero + CheckedAdd + CheckedSub + Div<usize, Output=Self::Value> + Default;
	type TimeLock: Parameter + Zero + Default;

	type TransactionInput: Parameter + TransactionInputTrait<Self> + Default;
	type TransactionOutput: Parameter + TransactionOutputTrait<Self> + Default;
	type Transaction: Parameter + TransactionTrait<Self> + Default;
	type SignedTransaction: Parameter + SignedTransactionTrait<Self> + Default;

	type Inserter: Inserter<Self>;
	type Remover: Remover<Self>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

type CheckResult<T> = std::result::Result<T, &'static str>;

pub trait Inserter<T: Trait> {
	fn insert(tx: &T::Transaction) {
		// new output is inserted to UTXO.
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, out) in tx.outputs()
			.iter()
			.enumerate() {
			let identify = (hash.clone(), i);
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

impl<T: Trait> Inserter<T> for DefaultInserter<T> {}

pub trait Remover<T: Trait> {
	fn remove(tx: &T::Transaction) {
		for inp in tx.inputs().iter() {
			for key in inp.output().unwrap_or(Default::default()).keys().iter() {
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
				});
			}
			<UnspentOutputs<T>>::remove((inp.tx_hash(), inp.out_index()));
		}
	}
}

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct DefaultRemover<T: Trait>(PhantomData<T>);

impl<T: Trait> Remover<T> for DefaultRemover<T> {}

pub trait TransactionInputTrait<T: Trait> {
	fn new(tx_hash: T::Hash, out_index: usize) -> Self;
	fn tx_hash(&self) -> T::Hash;
	fn out_index(&self) -> usize;

	fn output(&self) -> Option<T::TransactionOutput> {
		<UnspentOutputs<T>>::get((self.tx_hash(), self.out_index()))
	}

	fn value(&self) -> T::Value {
		match self.output() {
			Some(tx_out) => tx_out.value(),
			None => T::Value::zero(),
		}
	}
}

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


impl<T: Trait> TransactionInputTrait<T> for TransactionInput<T> {
	fn new(tx_hash: T::Hash, out_index: usize) -> Self {
		Self { tx_hash, out_index }
	}
	fn tx_hash(&self) -> T::Hash {
		self.tx_hash
	}
	fn out_index(&self) -> usize {
		self.out_index
	}
}

pub trait TransactionOutputTrait<T: Trait> {
	fn new(value: T::Value, keys: Vec<<T as consensus::Trait>::SessionKey>, quorum: u32) -> Self;
	fn value(&self) -> T::Value;
	fn keys(&self) -> &Vec<<T as consensus::Trait>::SessionKey>;
	fn quorum(&self) -> u32;
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TransactionOutput<T: Trait> {
	///#[codec(compact)]
	pub value: T::Value,
	///#[codec(compact)]
	pub keys: Vec<<T as consensus::Trait>::SessionKey>,
	///#[codec(compact)]
	pub quorum: u32,
}

impl<T: Trait> TransactionOutputTrait<T> for TransactionOutput<T> {
	fn new(value: T::Value, keys: Vec<<T as consensus::Trait>::SessionKey>, quorum: u32) -> Self {
		Self { value, keys, quorum }
	}
	fn value(&self) -> T::Value {
		self.value.clone()
	}
	fn keys(&self) -> &Vec<<T as consensus::Trait>::SessionKey> {
		&self.keys
	}
	fn quorum(&self) -> u32 {
		self.quorum
	}
}

pub trait TransactionTrait<T: Trait>: Codec {
	fn new(inputs: Vec<T::TransactionInput>, outputs: Vec<T::TransactionOutput>, lock_time: T::TimeLock) -> Self;
	fn inputs(&self) -> &Vec<T::TransactionInput>;
	fn outputs(&self) -> &Vec<T::TransactionOutput>;
	fn lock_time(&self) -> T::TimeLock;

	// calculate leftover.
	fn leftover(&self) -> CheckResult<T::Value> {
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
}


#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Transaction<T: Trait> {
	///#[codec(compact)]
	pub inputs: Vec<T::TransactionInput>,
	///#[codec(compact)]
	pub outputs: Vec<T::TransactionOutput>,
	///#[codec(compact)]
	pub lock_time: T::TimeLock,
}

impl<T: Trait> TransactionTrait<T> for Transaction<T> {
	fn new(inputs: Vec<T::TransactionInput>, outputs: Vec<T::TransactionOutput>, lock_time: T::TimeLock) -> Self {
		Self { inputs, outputs, lock_time }
	}
	fn inputs(&self) -> &Vec<T::TransactionInput> {
		&self.inputs
	}
	fn outputs(&self) -> &Vec<T::TransactionOutput> {
		&self.outputs
	}
	fn lock_time(&self) -> T::TimeLock {
		self.lock_time.clone()
	}
}

pub trait SignedTransactionTrait<T: Trait> {
	fn new(payload: Option<T::Transaction>, signatures: Vec<T::Signature>, public_keys: Vec<<T as consensus::Trait>::SessionKey>) -> Self;
	fn payload(&self) -> &Option<T::Transaction>;
	fn signatures(&self) -> &Vec<T::Signature>;
	fn public_keys(&self) -> &Vec<<T as consensus::Trait>::SessionKey>;

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
			.inputs() {
			let output = <UnspentOutputs<T>>::get((input.tx_hash(), input.out_index()))
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
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SignedTransaction<T: Trait> {
	///#[codec(compact)]
	pub payload: Option<T::Transaction>,
	///#[codec(compact)]
	pub signatures: Vec<T::Signature>,
	///#[codec(compact)]
	pub public_keys: Vec<<T as consensus::Trait>::SessionKey>,
}

impl<T: Trait> SignedTransactionTrait<T> for SignedTransaction<T> {
	fn new(payload: Option<T::Transaction>, signatures: Vec<T::Signature>, public_keys: Vec<<T as consensus::Trait>::SessionKey>) -> Self {
		Self { payload, signatures, public_keys }
	}
	fn payload(&self) -> &Option<T::Transaction> {
		&self.payload
	}
	fn signatures(&self) -> &Vec<T::Signature> {
		&self.signatures
	}
	fn public_keys(&self) -> &Vec<<T as consensus::Trait>::SessionKey> {
		&self.public_keys
	}
}

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Utxo {
		/// All valid unspent transaction outputs are stored in this map.
		/// Initial set of UTXO is populated from the list stored in genesis.
		pub UnspentOutputs get(unspent_outputs) build(|config: &GenesisConfig<T>| {
			config.initial_tx
				.clone()
				.outputs()
				.iter()
				.enumerate()
				.map(|(i, u)| ((T::Hashing::hash_of(&config.initial_tx), i), u.clone()))
				.collect::<Vec<_>>()
		}): map (<T as system::Trait>::Hash, usize) => Option<T::TransactionOutput>;

		/// [SessionKey] = reference of UTXO.
		pub UnspentOutputsFinder get(unspent_outputs_finder) build(|config: &GenesisConfig<T>| { // TODO more clearly
			let mut finder: HashMap<<T as system::Trait>::Hash,Vec<(<T as system::Trait>::Hash, usize)>> = Default::default();
			let mut vc: Vec<(<T as consensus::Trait>::SessionKey,(Vec<(<T as system::Trait>::Hash, usize)>))> = vec!{};
			let mut keys: Vec<<T as consensus::Trait>::SessionKey> = vec!{};
			let _ = config.initial_tx
				.clone()
				.outputs()
				.iter()
				.enumerate()
				.inspect(|(i, u)| {
					let _ = u.keys()
						.iter()
						.inspect(|key|{
							let hash = <T as system::Trait>::Hashing::hash_of(*key);
							let inh = (<T as system::Trait>::Hashing::hash_of(&config.initial_tx), *i);
							if let Some(f) = finder.get_mut(&hash) {
								f.push(inh);
							} else {
								finder.insert(hash, vec!{inh});
							}
							keys.push((**key).clone());
						})
						.count();
				})
				.count();
			for key in keys.iter() {
				let hash = <T as system::Trait>::Hashing::hash_of(key);
				if let Some(e) = finder.get(&hash) {
					vc.push((key.clone(), e.to_vec()));
					finder.remove(&hash);
				}
			}
			vc
		}): map <T as consensus::Trait>::SessionKey => Option<Vec<(<T as system::Trait>::Hash, usize)>>; //TODO HashSet<>

		/// Total leftover value to be redistributed among authorities.
		/// It is accumulated during block execution and then drained
		/// on block finalization.
		pub LeftoverTotal get(leftover_total): T::Value;

		/// Outputs that are locked
		pub LockedOutputs get(locked_outputs): map T::BlockNumber => Option<Vec<T::TransactionOutput>>;
	}

	add_extra_genesis {
		config(initial_tx): T::Transaction;
	}
}

decl_module! {
	// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		// Dispatch a single transaction and update UTXO set accordingly
		pub fn execute(origin, signed_tx: Vec<u8>) -> Result {
			ensure_signed(origin)?;

			let signed_tx = T::SignedTransaction::decode(&mut &signed_tx[..]).ok_or("signed_tx is undecoded bytes.")?;
			// all signature checking Signature.Verify(HashableSessionKey, hash(transaction.payload)).
			signed_tx.verify()?;
			// UTXO unlocked checking.
			signed_tx.unlock()?;
			// LeftOver(Fee) calclate.
			let leftover = signed_tx
				.payload()
				.as_ref()
				.unwrap()
				.leftover()?;

			// Calculate new leftover total
			let new_total = <LeftoverTotal<T>>::get()
				.checked_add(&leftover)
				.ok_or("leftover overflow")?;

			Self::update_storage(&signed_tx, new_total);
			Self::deposit_event(RawEvent::TransactionExecuted(signed_tx));
			Ok(())
		}

		// Handler called by the system on block finalization
		pub fn on_finalize(_n: T::BlockNumber) {
			Self::spend_leftover(&consensus::Module::<T>::authorities());
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event<T> where SignedTransaction = <T as Trait>::SignedTransaction {
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

	/// Redistribute combined leftover value evenly among authorities
	fn spend_leftover(authorities: &[<T as consensus::Trait>::SessionKey]) {
		let leftover = <LeftoverTotal<T>>::take();

		// send leftover to all authorities.
		if authorities.len() == 0 { return; }
		let shared_value = leftover / (authorities.len());
		if shared_value == <T as Trait>::Value::zero() { return; }

		// create UnspentTransactionOutput
		let outs: Vec<_> = authorities.iter()
			.map(|key|
				T::TransactionOutput::new(
					shared_value.clone(),
					vec! {key.clone(), },
					1))
			.collect();

		// crate Transaction for calc hash
		let tx = T::Transaction::new(
			vec! {},
			outs.clone(),
			T::TimeLock::zero());
		let hash = T::Hashing::hash_of(&tx);

		// UnspentOutputs[hash][i] = unspentOutput
		for (i, out) in outs.iter().enumerate() {
			let identify = (hash.clone(), i);
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
	use primitives::{ed25519, Pair, H256, Blake2Hasher};
	use std::clone::Clone;

	impl_outer_origin! {
	pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq, Default)]
	#[cfg_attr(feature = "std", derive(Debug))]
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
		type Signature = Signature;
		type Value = MVPValue;
		type TimeLock = Self::BlockNumber;

		type TransactionInput = TransactionInput<Test>;
		type TransactionOutput = TransactionOutput<Test>;
		type Transaction = Transaction<Test>;
		type SignedTransaction = SignedTransaction<Test>;

		type Inserter = DefaultInserter<Test>;
		type Remover = DefaultRemover<Test>;

		type Event = ();
	}

	fn authority_key_pair(s: &str) -> ed25519::Pair {
		ed25519::Pair::from_string(&format!("//{}", s), None)
			.expect("static values are valid; qed")
	}

	fn gen_normal_tx(in_hash: <Test as system::Trait>::Hash, in_index: usize,
					 out_value: <Test as Trait>::Value, out_key: <Test as consensus::Trait>::SessionKey) -> <Test as Trait>::Transaction {
		<Test as Trait>::Transaction::new(
			vec! {<Test as Trait>::TransactionInput::new(in_hash, in_index)},
			vec! {<Test as Trait>::TransactionOutput::new(out_value, vec! {out_key}, 1)},
			0)
	}

	fn hash(tx: &<Test as Trait>::Transaction) -> <Test as system::Trait>::Hash {
		<Test as system::Trait>::Hashing::hash_of(tx)
	}

	fn sign(tx: &<Test as Trait>::Transaction, key_pair: &ed25519::Pair) -> <Test as Trait>::SignedTransaction {
		let signature = key_pair.sign(&hash(tx)[..]);
		<Test as Trait>::SignedTransaction::new(Some(tx.clone()), vec! {signature}, vec! {key_pair.public()})
	}

	fn genesis_tx(root: &ed25519::Pair) -> <Test as Trait>::Transaction {
		<Test as Trait>::Transaction::new(
			vec! {},
			vec! {
				<Test as Trait>::TransactionOutput::new(
					MVPValue::new(1 << 60),
					vec! {root.public()},
					1),
			},
			0)
	}

	// This function basically just builds ax genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext(root: &ed25519::Pair) -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(GenesisConfig::<Test> {
			initial_tx: genesis_tx(root),
		}.build_storage().unwrap().0);
		t.into()
	}

	type Consensus = consensus::Module<Test>;
	type UTXO = Module<Test>;

	#[test]
	fn minimum_works() {// TODO fix divided tests.
		let root_key_pair = authority_key_pair("test_root");
		let authorities = vec! {
			authority_key_pair("test_authority_1").public(),
			authority_key_pair("test_authority_2").public()};
		with_externalities(&mut new_test_ext(&root_key_pair), || {
			// consensus set_authorities. (leftover getter.)
			Consensus::set_authorities(authorities.as_slice());

			// check genesis tx.
			let exp_gen_tx = genesis_tx(&root_key_pair);
			let act_gen_out = <UnspentOutputs<Test>>::get((hash(&exp_gen_tx), 0));
			assert_eq!(exp_gen_tx.outputs()[0], act_gen_out.unwrap());

			// check reference of genesis tx.
			let ref_utxo = <UnspentOutputsFinder<Test>>::get(root_key_pair.public());
			assert_eq!(1, ref_utxo.as_ref().unwrap().len());
			assert_eq!(hash(&exp_gen_tx), ref_utxo.as_ref().unwrap()[0].0);
			assert_eq!(0, ref_utxo.as_ref().unwrap()[0].1);

			// check total leftover is 0
			let leftover_total = <LeftoverTotal<Test>>::get();
			assert_eq!(0, *leftover_total);

			let receiver_key_pair = authority_key_pair("test_receiver");
			let new_signed_tx = sign(
				&gen_normal_tx(hash(&exp_gen_tx),
							   0, MVPValue::new(1 << 59), receiver_key_pair.public()),
				&root_key_pair,
			);
			assert_ok!(UTXO::execute(Origin::signed(1), new_signed_tx.encode()));

			// already spent genesis utxo.
			let spent_utxo = <UnspentOutputs<Test>>::get((hash(&exp_gen_tx), 0));
			assert!(spent_utxo.is_none());
			// already spent reference of genesis utxo.
			let ref_utxo = <UnspentOutputsFinder<Test>>::get(root_key_pair.public());
			assert!(ref_utxo.is_none());

			// get new transaction.
			let act_gen_out2 = <UnspentOutputs<Test>>::get((hash(new_signed_tx.payload().as_ref().unwrap()), 0));
			assert!(act_gen_out2.is_some());
			assert_eq!(new_signed_tx.payload().as_ref().unwrap().outputs()[0],
					   act_gen_out2.unwrap());
			// get reference of new teranction.
			let ref_utxo = <UnspentOutputsFinder<Test>>::get(receiver_key_pair.public());
			assert!(ref_utxo.is_some());
			assert_eq!(1, ref_utxo.as_ref().unwrap().len());
			assert_eq!(hash(new_signed_tx.payload().as_ref().unwrap()), ref_utxo.as_ref().unwrap()[0].0);
			assert_eq!(0, ref_utxo.as_ref().unwrap()[0].1);

			// check total leftover is (1<<60) - (1<<59)
			let leftover_total = <LeftoverTotal<Test>>::get();
			assert_eq!((1 << 59), *leftover_total);

			// on_finalize
			UTXO::on_finalize(1);
			// get reference of getting authorities leftover and get utxo.
			for authority in &authorities {
				// ref utxo
				let ref_utxo_authority = <UnspentOutputsFinder<Test>>::get(authority);
				let ref_utxo_authority = ref_utxo_authority.unwrap();
				assert_eq!(1, ref_utxo_authority.len());

				// utxo
				let utxo_authority = <UnspentOutputs<Test>>::get(ref_utxo_authority[0]);
				let utxo_authority = utxo_authority.unwrap();
				// value is (1<<59)/2 = (1<<58);
				assert_eq!((1 << 58), *utxo_authority.value());
				// keys = {authority}
				assert_eq!(1, utxo_authority.keys().len());
				assert_eq!(authority, &utxo_authority.keys()[0]);
			}

			// check total leftover is 0 after finalize
			let leftover_total = <LeftoverTotal<Test>>::get();
			assert_eq!(0, *leftover_total);
		});
	}
}
