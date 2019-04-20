use super::*;
// use Encode, Decode
use parity_codec::{Encode, Decode};
use plasm_merkle::MerkleTreeTrait;

#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MVPInserter<T: Trait, Tree: MerkleTreeTrait<T::Hash>>(PhantomData<(T, Tree)>);

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash>> Inserter<T> for MVPInserter<T, Tree> {
	fn insert(tx: &T::Transaction) {
		Self::using_insert(tx);
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, _) in tx.outputs().iter().enumerate() {
			Tree::push(<T as system::Trait>::Hashing::hash_of(&(hash, i)));
		}
	}
}

//#[derive(Clone, Eq, PartialEq, Default)]
//#[cfg_attr(feature = "std", derive(Debug))]
//pub struct MVPRemover<T: Trait>(PhantomData<T>);
//
//impl<T: Trait> Remover<T> for MVPRemover<T> {
//	fn remove(tx: &T::Transaction) {
//		Self::using_remove(tx);
//	}
//}
//
#[derive(Clone, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MVPFinalizer<T: Trait, Tree: MerkleTreeTrait<T::Hash>> (PhantomData<T>);

impl<T: Trait, Tree: MerkleTreeTrait<T::Hash>> Finalizer<T> for MVPFinalizer<T, Tree> {
	fn finalize(authorities: &[<T as consensus::Trait>::SessionKey]) {
		Self::using_finalize(authorities);
		Tree::comit();
	}
}
