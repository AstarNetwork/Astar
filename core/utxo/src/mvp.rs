use super::*;
// use Encode, Decode
use plasm_merkle::MerkleTreeTrait;

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Inserter<T, Tree>(PhantomData<(T, Tree)>);

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash, T::Hashing>> InserterTrait<T> for Inserter<T, Tree> {
	fn insert(tx: &T::Transaction) {
		Self::standart_insert(tx);
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, _) in tx.outputs().iter().enumerate() {
			Tree::push(<T as system::Trait>::Hashing::hash_of(&(hash, i)));
		}
	}
}

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Finalizer<T, Tree> (PhantomData<(T, Tree)>);

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash, T::Hashing>> FinalizerTrait<T> for Finalizer<T, Tree> {
	fn standart_finalize(n: T::BlockNumber) {
		Tree::commit();
	}
}
