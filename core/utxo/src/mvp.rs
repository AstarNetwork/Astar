use super::*;
// use Encode, Decode
use plasm_merkle::MerkleTreeTrait;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash};
use parity_codec::Codec;

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Inserter<T, Tree>(PhantomData<(T, Tree)>);

pub fn utxo_hash<Hashing, H>(tx_hash: &H, i: &usize) -> H
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	Hashing::hash(&plasm_primitives::concat_bytes(tx_hash, i))
}

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash, T::Hashing>> InserterTrait<T> for Inserter<T, Tree> {
	fn insert(tx: &T::Transaction) {
		Self::default_insert(tx);
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, _) in tx.outputs().iter().enumerate() {
			Tree::push(utxo_hash::<T::Hashing, T::Hash>(&hash, &i))
		}
	}
}

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Finalizer<T, Tree> (PhantomData<(T, Tree)>);

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash, T::Hashing>> FinalizerTrait<T> for Finalizer<T, Tree> {
	fn default_finalize(n: T::BlockNumber) {
		Tree::commit();
	}
}
