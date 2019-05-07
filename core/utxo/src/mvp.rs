use super::*;
// use Encode, Decode
use plasm_merkle::MerkleTreeTrait;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash, SimpleArithmetic};
use parity_codec::Codec;

pub use plasm_primitives::mvp::Value;
use std::marker::PhantomData;

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

impl<H> TransactionInput<H> {
	fn output_or_default<T: Trait>(&self) -> TransactionOutput<T::Value, T::AccountId>
		where (H, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		match <UnspentOutputs<T>>::get(&(self.tx_hash, self.out_index)) {
			Some(tx_out) => tx_out,
			None => Default::default(),
		}
	}
	fn output<T: Trait>(&self) -> Option<TransactionOutput<T::Value, T::AccountId>>
		where (H, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		<UnspentOutputs<T>>::get(&(self.tx_hash, self.out_index))
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

/// V: Value, K: Key, H: Hash
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct Transaction<V, K, H> {
	///#[codec(compact)]
	pub inputs: Vec<TransactionInput<H>>,
	///#[codec(compact)]
	pub outputs: Vec<TransactionOutput<V, K>>,
	///#[codec(compact)]
	pub lock_time: TimeLock,
}

type Tx<T: Trait> = Transaction<T::Value, T::AccountId, T::Hash>;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SignedTransaction<V, K, H, S> {
	///#[codec(compact)]
	pub payload: Transaction<V, K, H>,
	///#[codec(compact)]
	pub signatures: Vec<S>,
	///#[codec(compact)]
	pub public_keys: Vec<K>,
}

type SignedTx<T: Trait> = SignedTransaction<T::Value, T::AccountId, T::Hash, T::Signature>;

pub fn hash_of<Hashing, H>(tx_hash: &H, i: &u32) -> H
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	Hashing::hash(&plasm_primitives::concat_bytes(tx_hash, i))
}

pub trait Trait: system::Trait {
	type Signature: Parameter + Verify<Signer=Self::AccountId>;
	type TimeLock: Parameter + Zero + Default;
	type Value: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + As<usize> + As<u64> + MaybeSerializeDebug;

	type Utxo: UtxoTriat<SignedTx<Self>>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

fn generate_tx_from_conf<T: Trait>(genesis: Vec<(T::Value, <T as system::Trait>::AccountId)>) -> Tx<T> {
	Transaction {
		inputs: vec! {},
		outputs: config.genesis_tx
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
			let tx = generate_tx_from_conf(config.genesis_tx);
			tx.clone()
				.outputs
				.iter()
				.enumerate()
				.map(|(i, u)| ((<T as system::Trait>::Hashing::hash_of(&tx), i as u32), u.clone()))
				.collect::<Vec<_>>()
		}): map (<T as system::Trait>::Hash, u32) => Option<TxOut<T>>;

		/// [AccountId] = reference of UTXO.
		pub UnspentOutputsFinder get(unspent_outputs_finder) build( |config: &GenesisConfig<T> | {
			let tx = generate_tx_from_conf(config.genesis_tx);
			config.genesis_tx
				.clone()
				.iter()
				.enumerate()
				.map(|(i, e)| (e.1.clone(), vec!{(<T as system::Trait>::Hashing::hash_of(&tx), i as u32)}))
				.collect::<Vec<_>>()
		}): map <T as system::Trait>::AccountId => Option<Vec<(<T as system::Trait>::Hash, u32)>>; //TODO HashSet<>

		pub TxList get(tx_list) build( |config: &GenesisConfig<T> | {
			let tx = generate_tx_from_conf(config.genesis_tx);
			vec!{(<T as system::Trait>::Hashing::hash_of(&tx), tx)}
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
			Self::execute(signed_tx);
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event {
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
			for key in out.keys() {
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
		let hash = <T as system::Trait>::Hashing::hash_of(&signed_tx.payload);
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
