use rstd::marker::PhantomData;
use rstd::prelude::*;
use sr_primitives::traits::SimpleBitOps;
use support::storage::child;
use parity_codec::{Encode, Codec};

pub trait ConcatHash {
	fn concat<T>(a: T, b: T) -> T
		where T: SimpleBitOps + Eq + Default + Copy;
}

pub struct MockConcatHasher;

impl ConcatHash for MockConcatHasher {
	fn concat<T>(a: T, b: T) -> T
		where T: SimpleBitOps + Eq + Default + Copy
	{
		if a == Default::default() { return b; }
		if b == Default::default() { return a; }
		a ^ b
	}
}

// H: Hash, O: Outpoint(Hashable)
pub trait MerkleTreeTrait<H: Codec> {
	type ConcatHasher: ConcatHash;
	type Proofs;
	/// get root Hash of MerkleTree.
	fn root() -> H;
	/// get proofs of leaf.
	fn proofs(leaf: &H) -> Self::Proofs;
	/// push Hash to MerkleTree.
	fn push(leaf: H);
}

// mock merkle tree trie id name. no conflict.
const MOCK_MERKLE_TREE_TRIE_ID: &'static str = "mock_merkle_tree_trie_id";
/// must be 2^n.
const MOCK_MERKLE_TREE_LIMIT: u64 = (1 << 5);

/// MockMerkleTree measn
/// 		0
/// 1	2		3	4
/// 5 6 7 8	  9 10 11 12
///
/// Alike SegmentTree. So fixed number of data.
pub struct MockMerkleTree<H>(PhantomData<(H)>);

impl<H: Codec + Default> MockMerkleTree<H> {
	pub fn get_hash(index: &u64) -> H {
		match MerkleDb::<&'static str, u64, H>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, index) {
			Some(h) => h,
			None => Default::default(),
		}
	}
	pub fn get_index(h: &H) -> u64 {
		match MerkleDb::<&'static str, H, u64>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h) {
			Some(index) => index,
			None => MOCK_MERKLE_TREE_LIMIT << 1,
		}
	}
	pub fn push_hash(index: &u64, h: H) {
		MerkleDb::<&'static str, u64, H>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, index, h);
	}

	pub fn push_index(h: &H, index: u64) {
		MerkleDb::<&'static str, H, u64>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h, index);
	}
}

impl<H: Codec + Default + Clone + Copy + SimpleBitOps + Eq> MerkleTreeTrait<H> for MockMerkleTree<H> {
	type ConcatHasher = MockConcatHasher;
	type Proofs = MockProofs<H>;
	fn root() -> H {
		Self::get_hash(&0)
	}

	fn proofs(leaf: &H) -> Self::Proofs {
		let mut index: u64 = Self::get_index(leaf);
		let mut proofs = MockProofs::Leaf(leaf.clone());

		index += MOCK_MERKLE_TREE_LIMIT - 1;
		while index > 0 {
			let lr: bool = (index & 1) == 0;
			index = (index - 1) / 2;
			proofs = match lr {
				/// left leafs.
				true => MockProofs::Node(Box::<MockProofs<H>>::new(proofs),
										 Box::<MockProofs<H>>::new(MockProofs::Leaf(Self::get_hash(&(2 * index + 2))))),
				/// right leafs.
				false => MockProofs::Node(Box::<MockProofs<H>>::new(MockProofs::Leaf(Self::get_hash(&(2 * index + 1_)))),
										  Box::<MockProofs<H>>::new(proofs)),
			}
		}
		proofs
	}

	fn push(leaf: H) {
		let mut index: u64 = Self::get_index(&Default::default());
		Self::push_index(&Default::default(), index + 1); /// increments...
		Self::push_index(&leaf, index.clone());

		index += MOCK_MERKLE_TREE_LIMIT - 1;
		Self::push_hash(&index, leaf);
		while index > 0 {
			index = (index - 1) / 2;
			Self::push_hash(&index,
							Self::ConcatHasher::concat(
								Self::get_hash(&(2 * index + 1)),
								Self::get_hash(&(2 * index + 2))));
		}
	}
}

// For Mock Proofs.
pub enum MockProofs<T> {
	Leaf(T),
	Node(Box<MockProofs<T>>, Box<MockProofs<T>>),
}

pub trait MerkleDb<Id: Encode, Key: Encode, O: Codec> {
	fn push(&self, trie_id: &Id, key: &Key, o: O) {
		child::put_raw(&trie_id.encode()[..], &key.encode()[..], &o.encode()[..]);
	}
	fn get(&self, trie_id: &Id, key: &Key) -> Option<O> {
		if let Some(ret) = child::get_raw(&trie_id.encode()[..], &key.encode()[..]) {
			return O::decode(&mut &ret[..]);
		}
		None
	}
}

pub struct DirectMerkleDb;

impl<Id: Encode, Key: Encode, O: Codec> MerkleDb<Id, Key, O> for DirectMerkleDb {}

#[cfg(test)]
mod tests {
	use super::*;
	use primitives::H256;

	fn mock_verify(proofs: MockProofs<H256>) -> H256 {
		match proofs {
			MockProofs::<H256>::Leaf(leaf) => leaf,
			MockProofs::<H256>::Node(left, right) => MockConcatHasher::concat::<H256>(
				mock_verify(*left),
				mock_verify(*right))
		}
	}

	fn merkle_cocnat_hash<H, Hasher, F>(rnd: F) where
		H: SimpleBitOps + Eq + Default + Copy + std::fmt::Debug,
		Hasher: ConcatHash,
		F: Fn() -> H {
		let a: H = Default::default();
		assert_eq!(a, Hasher::concat(H::default(), a));
		assert_eq!(a, Hasher::concat(a, H::default()));

		let x = rnd();
		let y = rnd();
		let z = rnd();
		assert_eq!(Hasher::concat(x, Hasher::concat(y, z)),
				   Hasher::concat(Hasher::concat(x, y), z));
	}

	#[test]
	fn merkle_mock_concat_hash() {
		merkle_cocnat_hash::<H256, MockConcatHasher, fn() -> H256>(|| { H256::random() });
	}

//	#[test]
//	fn merkle_mock_test() {
//		type MerkleTree = MockMerkleTree<H256>;
//		let hashes = vec!{1..10}.iter().map(|_| H256::random()).collect::<Vec<_>>();
//		hashes.iter()
//			.inspect(|h| MerkleTree::push(*h.clone()))
//			.count();
//
//		// verify
//		let root_hash = MerkleTree::root();
//		for i in 0..10 {
//			let proofs = MerkleTree::proofs(&hashes[i]);
//			assert_eq!(root_hash, mock_verify(proofs));
//		}
//	}
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
