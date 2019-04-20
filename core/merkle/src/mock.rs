use super::*;
use rstd::marker::PhantomData;
use sr_primitives::traits::SimpleBitOps;
use parity_codec::{Codec};

pub struct ConcatHasher;

impl ConcatHash for ConcatHasher {
	fn concat<T>(a: T, b: T) -> T
		where T: SimpleBitOps + Eq + Default + Copy
	{
		if a == Default::default() { return b; }
		if b == Default::default() { return a; }
		a ^ b
	}
}

// mock merkle tree trie id name. no conflict.
const MOCK_MERKLE_TREE_TRIE_ID: &'static str = "mock_merkle_tree_trie_id";
/// must be 2^n.
const MOCK_MERKLE_TREE_LIMIT: u64 = (1 << 5);

/// MerkleTree measn
/// 		0
/// 1	2		3	4
/// 5 6 7 8	  9 10 11 12
///
/// Alike SegmentTree. So fixed number of data.
pub struct MerkleTree<H>(PhantomData<(H)>);

impl<H: Codec + Default> MerkleTree<H> {
	pub fn get_hash(index: &u64) -> H {
		match MerkleDb::<&'static str, u64, H>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, index) {
			Some(h) => h,
			None => Default::default(),
		}
	}
	pub fn get_index(h: &H) -> u64 {
		match MerkleDb::<&'static str, H, u64>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h) {
			Some(index) => index,
			None => 0,
		}
	}
	pub fn push_hash(index: &u64, h: H) {
		MerkleDb::<&'static str, u64, H>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, index, h);
	}

	pub fn push_index(h: &H, index: u64) {
		MerkleDb::<&'static str, H, u64>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h, index);
	}
}

impl<H: Codec + Default + Clone + Copy + SimpleBitOps + Eq> MerkleTreeTrait<H> for MerkleTree<H> {
	type ConcatHasher = ConcatHasher;
	type Proofs = Proofs<H>;
	fn root() -> H {
		Self::get_hash(&0)
	}

	fn proofs(leaf: &H) -> Self::Proofs {
		let mut index: u64 = Self::get_index(leaf);
		let mut proofs = Proofs::Leaf(leaf.clone());

		index += MOCK_MERKLE_TREE_LIMIT - 1;
		while index > 0 {
			let lr: bool = (index & 1) == 1;
			index = (index - 1) / 2;
			proofs = match lr {
				// left leafs.
				true => Proofs::Node(Box::<Proofs<H>>::new(proofs),
									 Box::<Proofs<H>>::new(Proofs::Leaf(Self::get_hash(&(2 * index + 2))))),
				// right leafs.
				false => Proofs::Node(Box::<Proofs<H>>::new(Proofs::Leaf(Self::get_hash(&(2 * index + 1_)))),
									  Box::<Proofs<H>>::new(proofs)),
			}
		}
		proofs
	}

	fn push(leaf: H) {
		let mut index: u64 = Self::get_index(&Default::default());

		Self::push_index(&Default::default(), index + 1); // increments...
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

// For  Proofs.
#[derive(Debug)]
pub enum Proofs<T> {
	Leaf(T),
	Node(Box<Proofs<T>>, Box<Proofs<T>>),
}

