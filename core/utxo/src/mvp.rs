use super::*;
// use Encode, Decode
use plasm_merkle::MerkleTreeTrait;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash, SimpleArithmetic};
use parity_codec::Codec;

pub use plasm_primitives::mvp::Value;

/// H: Hash
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct TransactionInput<H> {
	///#[codec(compact)]
	pub tx_hash: H,
	///#[codec(compact)]
	pub out_index: u32,
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

pub trait Trait: system::Trait {
	type Signature: Parameter + Verify<Signer=Self::AccountId>;
	type TimeLock: Parameter + Zero + Default;
	type Value: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + As<usize> + As<u64> + MaybeSerializeDebug;

	type Utxo: UtxoTriat<SignedTransaction<Self::Value, Self::AccountId, Self::Hash, Self::Signature>>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}


#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Inserter<T, Tree>(PhantomData<(T, Tree)>);

pub fn utxo_hash<Hashing, H>(tx_hash: &H, i: &u32) -> H
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	Hashing::hash(&plasm_primitives::concat_bytes(tx_hash, i))
}

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash, T::Hashing>> InserterTrait<T> for Inserter<T, Tree> {
	fn insert(tx: &T::Transaction) {
		Self::default_insert(tx);
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, _) in tx.outputs().iter().enumerate() {
			Tree::new().push(utxo_hash::<T::Hashing, T::Hash>(&hash, &(i as u32)))
		}
	}
}

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Finalizer<T, Tree> (PhantomData<(T, Tree)>);

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash, T::Hashing>> FinalizerTrait<T> for Finalizer<T, Tree> {
	fn default_finalize(n: T::BlockNumber) {
		Tree::new().commit();
	}
}
