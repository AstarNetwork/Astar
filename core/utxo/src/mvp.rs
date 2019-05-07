use super::*;

#[cfg(feature = "std")]
use serde_derive::{Serialize, Deserialize};

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, Parameter, dispatch::Result};
use system::ensure_signed;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash, SimpleArithmetic, Verify, As, Zero, CheckedAdd, CheckedSub};

use parity_codec::{Encode, Decode, Codec};


/// H: Hash
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct TransactionInput<H> {
	///#[codec(compact)]
	pub tx_hash: H,
	///#[codec(compact)]
	pub out_index: u32,
}

type TxIn<T: Trait> = TransactionInput<T::Hash>;

impl<H: Copy> TransactionInput<H> {
	fn output_or_default<T: Trait>(&self) -> TransactionOutput<T::Value, T::AccountId>
		where (H, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		match <UnspentOutputs<T>>::get((self.tx_hash.clone(), self.out_index)) {
			Some(tx_out) => tx_out,
			None => Default::default(),
		}
	}
	fn output<T: Trait>(&self) -> Option<TransactionOutput<T::Value, T::AccountId>>
		where (H, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		<UnspentOutputs<T>>::get((self.tx_hash.clone(), self.out_index))
	}
}

/// V: Value, K: Key
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct TransactionOutput<V, K> {
	///#[codec(compact)]
	pub value: V,
	///#[codec(compact)]
	pub keys: Vec<K>,
	///#[codec(compact)]
	pub quorum: u32,
}

type TxOut<T: Trait> = TransactionOutput<T::Value, T::AccountId>;

/// V: Value, K: Key, H: Hash, L: TimeLock
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct Transaction<V, K, H, L> {
	///#[codec(compact)]
	pub inputs: Vec<TransactionInput<H>>,
	///#[codec(compact)]
	pub outputs: Vec<TransactionOutput<V, K>>,
	///#[codec(compact)]
	pub lock_time: L,
}

type Tx<T: Trait> = Transaction<T::Value, T::AccountId, T::Hash, T::TimeLock>;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SignedTransaction<V, K, H, S, L> {
	///#[codec(compact)]
	pub payload: Transaction<V, K, H, L>,
	///#[codec(compact)]
	pub signatures: Vec<S>,
	///#[codec(compact)]
	pub public_keys: Vec<K>,
}

type SignedTx<T: Trait> = SignedTransaction<T::Value, T::AccountId, T::Hash, T::Signature, T::TimeLock>;

pub fn hash_of<Hashing, H>(tx_hash: &H, i: &u32) -> H
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	Hashing::hash(&plasm_primitives::concat_bytes(tx_hash, i))
}

pub trait Trait: system::Trait {
	type Signature: Parameter + Default + Verify<Signer=Self::AccountId>;
	type TimeLock: Parameter + Zero + Default;
	type Value: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + As<usize> + As<u64> + MaybeSerializeDebug;

	type Utxo: UtxoTrait<SignedTx<Self>, Self::Value>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

fn generate_tx_from_conf<T: Trait>(genesis: &Vec<(T::Value, <T as system::Trait>::AccountId)>) -> Tx<T> {
	Transaction {
		inputs: vec! {},
		outputs: genesis
			.clone()
			.iter()
			.map(|e| TransactionOutput { value: e.0.clone(), keys: vec! {e.1.clone()}, quorum: 1 })
			.collect::<Vec<_>>(),
		lock_time: T::TimeLock::zero(),
	}
}

decl_storage! {
	trait Store for Module <T: Trait> as Utxo {
		/// All valid unspent transaction outputs are stored in this map.
		/// Initial set of UTXO is populated from the list stored in genesis.
		pub UnspentOutputs get(unspent_outputs) build( |config: &GenesisConfig<T>| {
			let tx = generate_tx_from_conf::<T>(&config.genesis_tx);
			tx.clone()
				.outputs
				.iter()
				.enumerate()
				.map(|(i, u)| ((<T as system::Trait>::Hashing::hash_of(&tx), i as u32), u.clone()))
				.collect::<Vec<_>>()
		}): map (<T as system::Trait>::Hash, u32) => Option<TxOut<T>>;

		/// [AccountId] = reference of UTXO.
		pub UnspentOutputsFinder get(unspent_outputs_finder) build( |config: &GenesisConfig<T> | {
			let tx = generate_tx_from_conf::<T>(&config.genesis_tx);
			config.genesis_tx
				.clone()
				.iter()
				.enumerate()
				.map(|(i, e)| (e.1.clone(), vec!{(<T as system::Trait>::Hashing::hash_of(&tx), i as u32)}))
				.collect::<Vec<_>>()
		}): map <T as system::Trait>::AccountId => Option<Vec<(<T as system::Trait>::Hash, u32)>>; //TODO HashSet<>

		pub TxList get(tx_list) build( |config: &GenesisConfig<T> | {
			let tx = generate_tx_from_conf::<T>(&config.genesis_tx);
			vec!{(<T as system::Trait>::Hashing::hash_of(&tx),
				SignedTx::<T> {
					payload: tx,
					signatures: vec!{},
					public_keys: vec!{},
				})}
		}): map <T as system::Trait>::Hash => SignedTx<T>;

		/// Total leftover value to be redistributed among authorities.
		/// It is accumulated during block execution and then drained
		/// on block finalization.
		pub LeftoverTotal get(leftover_total): T::Value;

		// TODO Outputs that are locked
		//	pub LockedOutputs get(locked_outputs): map T::BlockNumber => Option <Vec<TxOut<T>>>;
	}

	add_extra_genesis {
		config(genesis_tx): Vec<(T::Value, <T as system::Trait>::AccountId)>;
	}
}

decl_module! {
	// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T> () = default;

		// Dispatch a single transaction and update UTXO set accordingly
		pub fn execute(origin, signed_tx: SignedTx<T> ) -> Result {
			ensure_signed(origin)?;
			Self::exec(signed_tx)
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event<T> where SignedTransaction = SignedTx<T> {
		/// Transaction was executed successfully
		TransactionExecuted(SignedTransaction),
	}
);

impl<T: Trait> WritableUtxoTrait<SignedTx<T>> for Module<T> {
	/// push: updated UnspentOutputs, UnspentOutputsFinder and TxList.
	fn push(signed_tx: SignedTx<T>) {
		let tx = &signed_tx.payload;
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, out) in tx.outputs
			.iter()
			.enumerate() {
			let identify = (hash.clone(), i as u32);
			<UnspentOutputs<T>>::insert(identify.clone(), out.clone());
			for key in out.keys.iter() {
				<UnspentOutputsFinder<T>>::mutate(key, |v| {
					match v.as_mut() {
						Some(vc) => vc.push(identify.clone()),
						None => *v = Some(vec! {identify.clone()}),
					}
				});
			}
		}
		<TxList<T>>::insert(&hash, signed_tx);
	}
}

impl<T: Trait> ReadbleUtxoTrait<SignedTx<T>, T::Value> for Module<T> {
	fn verify(signed_tx: &SignedTx<T>) -> Result {
		let mut hash = <T as system::Trait>::Hashing::hash_of(&signed_tx.payload);
		for (sign, key) in signed_tx.signatures.iter().zip(signed_tx.public_keys.iter()) {
			if !sign.verify(hash.as_mut() as &[u8], key) {
				return Err("signature is unverified.");
			}
		}
		Ok(())
	}

	fn unlock(signed_tx: &SignedTx<T>) -> Result {
		let keys: Vec<_> = signed_tx.public_keys.iter().collect();
		for input in signed_tx.payload
			.inputs
			.iter() {
			let output = input.output::<T>()
				.ok_or("specified utxo by input is not found.")?;
			if output.quorum > output
				.keys
				.iter()
				.filter(|key|
					keys.contains(key))
				.count() as u32 {
				return Err("not enough public_keys to unlock all specified utxo.");
			}
		}
		Ok(())
	}

	fn leftover(signed_tx: &SignedTx<T>) -> CheckResult<T::Value> {
		let sum_in: T::Value = signed_tx
			.payload
			.inputs
			.iter()
			.try_fold(T::Value::zero(), |sum, inp| sum.checked_add(&inp.output_or_default::<T>().value))
			.ok_or("sum of inputs value is overflow")?;
		let sum_out: T::Value = signed_tx
			.payload
			.outputs
			.iter()
			.try_fold(T::Value::zero(), |sum, out| sum.checked_add(&out.value))
			.ok_or("sum of outputs value is overflow")?;
		let leftover = sum_in.checked_sub(&sum_out).ok_or("leftover invalid (sum of input) - (sum of output)")?;
		Ok(leftover)
	}
}

impl<T: Trait> UtxoTrait<SignedTx<T>, T::Value> for Module<T> {
	fn exec(signed_tx: SignedTx<T>) -> Result {
		// all signature checking Signature.Verify(HashableAccountId, hash(transaction.payload)).
		Self::verify(&signed_tx)?;
		// UTXO unlocked checking.
		Self::unlock(&signed_tx)?;
		// LeftOver(Fee) calclate.
		let leftover = Self::leftover(&signed_tx)?;

		// Calculate new leftover total
		let new_total = <LeftoverTotal<T>>::get()
			.checked_add(&leftover)
			.ok_or("leftover overflow")?;

		<LeftoverTotal<T>>::put(new_total);
		Self::push(signed_tx.clone());
		Self::deposit_event(RawEvent::TransactionExecuted(signed_tx));
		Ok(())
	}
}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;
	use sr_primitives::{
		traits::BlakeTwo256,
	};
	use primitives::{sr25519, Pair, H256};

	pub type Signature = sr25519::Signature;

	pub type AccountId = <Signature as Verify>::Signer;

	pub type MerkleTree = plasm_merkle::mock::MerkleTree<H256, BlakeTwo256>;

	pub type Value = u64;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct Test;

	fn hash(tx: &Tx<Test>) -> H256 {
		BlakeTwo256::hash_of(tx)
	}

	pub fn genesis_tx(root: &sr25519::Pair) -> Vec<(u64, AccountId)> {
		vec! {(1000000000000000, &root.public()), }
	}

	// This function basically just builds ax genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext(root: &sr25519::Pair) -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(GenesisConfig::<Test> {
			genesis_tx: genesis_tx(root),
		}.build_storage().unwrap().0);
		t.into()
	}

	type UTXO = Module<Test>;

	pub fn account_key_pair(s: &str) -> sr25519::Pair {
		sr25519::Pair::from_string(&format!("//{}", s), None)
			.expect("static values are valid; qed")
	}

	pub fn verify(account_id: &AccountId, value: u64, num_of_utxo: usize) {
		let ref_utxo = <UnspentOutputsFinder<Test>>::get(account_id);
		assert_eq!(num_of_utxo, ref_utxo.as_ref().unwrap().len());
		let utxos = ref_utxo
			.iter()
			.map(|r| <UnspentOutputs<Test>>::get((r.0.clone(), r.1)))
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		let sum = utxos.fold(0, |sum, v| sum + v);
		assert_eq!(value, sum);
	}


	fn mvp_minimum_works() {
		let root_key_pair = account_key_pair("test_root");
		with_externalities(&mut new_test_ext(&root_key_pair), || {
			// check merkle root ============================== different default
			verify(root_key_pair.public(), 1000000000000000, 1);

			// TODO
			let root_hash = MerkleTree::new().root();
			// check reference of genesis tx.
			let ref_utxo = <UnspentOutputsFinder<Test>>::get(root_key_pair.public());
			assert_eq!(1, ref_utxo.as_ref().unwrap().len());
			assert_eq!(0, ref_utxo.as_ref().unwrap()[0].1);
			let exp_gen_outpoint = ref_utxo.as_ref().unwrap()[0];

			// check genesis tx.
			let exp_gen_tx = &genesis_tx::<Test>(&root_key_pair)[0];
			let act_gen_out = <UnspentOutputs<Test>>::get(exp_gen_outpoint);
			assert_eq!(exp_gen_tx.0, act_gen_out.as_ref().unwrap().value());
			assert_eq!(1, act_gen_out.as_ref().unwrap().keys().len());
			assert_eq!(exp_gen_tx.1, act_gen_out.as_ref().unwrap().keys()[0]);

			// check total leftover is 0
			let leftover_total = <LeftoverTotal<Test>>::get();
			assert_eq!(0, *leftover_total);

			let receiver_key_pair = account_key_pair("test_receiver");
			let new_signed_tx = sign::<Test>(
				&gen_normal_tx(exp_gen_outpoint.0,
							   exp_gen_outpoint.1, Value::new(1 << 59), receiver_key_pair.public()),
				&root_key_pair,
			);
			assert_ok!(UTXO::execute(Origin::signed(root_key_pair.public()), new_signed_tx.encode()));

			// already spent genesis utxo.
			let spent_utxo = <UnspentOutputs<Test>>::get(exp_gen_outpoint);
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

			// not yet change root hash ========================= different default TODO genesis tx is not exist (after that issue)
			assert_eq!(root_hash, MerkleTree::new().root());

			// on_finalize
			UTXO::on_finalize(1);

			// changed root hash ============================== different default VVV
			let new_root_hash = MerkleTree::new().root();
			assert_ne!(root_hash, new_root_hash);

			// proofs by ref utxo.
			let proofs = MerkleTree::new().proofs(&BlakeTwo256::hash_of(&ref_utxo.as_ref().unwrap()[0]));
			assert_eq!(new_root_hash, proofs.root::<BlakeTwo256>());
		});
	}
}
