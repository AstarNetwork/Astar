use rstd::marker::PhantomData;
use rstd::prelude::*;
use sr_primitives::traits::{SimpleBitOps, Zero};
use support::storage::child;

pub trait ConcatHash {
	fn concat<T>(a: T, b: T) -> T where T: SimpleBitOps + Zero + Eq;
}

pub struct MockConcatHasher;

impl ConcatHash for MockConcatHasher {
	fn concat<T>(a: T, b: T) -> T where T: SimpleBitOps + Zero + Eq {
		if a == T::zero() { return b; }
		if b == T::zero() { return a; }
		a ^ b
	}
}

// H: Hash, O: Outpoint(Hashable)
pub trait MerkleTreeTrait<H, O> {
	type Proofs;
	fn root() -> H;
	fn proofs(leaf: O) -> Self::Proofs;
	fn push(leaf: O);
}

pub struct MerkleTree<H, O>(PhantomData<(H, O)>);

impl<H, O> MerkleTreeTrait<H, O> for MerkleTree<H, O> {
	type Proofs = MockProofs<H>;
	fn root() -> H {
		H()
	}
	fn proofs(leaf: O) -> Self::Proofs { Self::Proofs() }
	fn push(leaf: O) {}
}

enum MockProofs<T> {
	Leaf(T),
	Node(Box<MockProofs<T>>, Box<MockProofs<T>>),
}

pub struct MerkleDb<O, Id, Key>(PhantomData<O, Id, Key>);

impl<O, Id, Key> MerkleDb<O, Id, Key>
	where
		O: Into<&[u8]>,
		Id: Into<&[u8]>,
		key: Into<&[u8]>,
{
	fn push(o: O, trie_id: Id, key: Key) {
		child::put_raw(o.into(), trie_id.into(), key.into());
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn merkle_mock_test() {
		assert!(true)
	}
}
// 完全二分木による実装。
// SegmentTree っぽく予め 2*n のデータをとっておく。
// 利点：実装が楽。
// other: UTXO の最大プール数(n)が決まっている。nを超えると証明不可能になる。（一巡するので）
// [index] =

//pub trait MerkleProof<H> {
//	type Her;
//	fn left() -> Option<Self>;
//	fn right() -> Option<Self>;
//	fn hash() -> H;
//	fn concat() -> H;
//}
//
//pub struct HeapLikeMerkleProof<Her, H> {
//	pub left: HeapLikeMerkleProof;
//	pub right: HeapLikeMerkleProof;
//	pub hash: Option<H>
//}
//
//pub trait MerkleStorage {
//	type MerkleProof;
//	fn push(index: u64);
//	fn get(index: u64) -> Self::MerkleProof;
//}
//
