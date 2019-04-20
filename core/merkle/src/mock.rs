#![cfg_attr(not(feature = "std"), no_std)]

use super::*;
use rstd::marker::PhantomData;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash};
use parity_codec::Codec;

// mock merkle tree trie id name. no conflict.
const MOCK_MERKLE_TREE_TRIE_ID: &'static str = "mock_merkle_tree_trie_id";
const MOCK_MERKLE_TREE_DEPTH: u8 = 20;
/// must be 2^n.
const MOCK_MERKLE_TREE_LIMIT: u64 = (1 << MOCK_MERKLE_TREE_DEPTH as u64);

/// MerkleTree measn
/// 		0
/// 1	2		3	4
/// 5 6 7 8	  9 10 11 12
///
/// Alike SegmentTree. So fixed number of data.
pub struct MerkleTree<H, Hashing>(PhantomData<(H, Hashing)>);

impl<H: Codec + Default, Hashing> MerkleTree<H, Hashing> where {
	pub fn get_hash(index: u64) -> H {
		MerkleDb::<&'static str, u64, H>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, &index).unwrap_or(Default::default())
	}
	pub fn get_index(h: &H) -> u64 {
		MerkleDb::<&'static str, H, u64>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h).unwrap_or(0)
	}
	pub fn push_hash(index: u64, h: H) {
		MerkleDb::<&'static str, u64, H>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, &index, h);
	}

	pub fn push_index(h: &H, index: u64) {
		MerkleDb::<&'static str, H, u64>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h, index);
	}
}

impl<H, Hashing> MerkleTreeTrait<H, Hashing> for MerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	fn root() -> H {
		Self::get_hash(0)
	}

	fn proofs(leaf: &H) -> MerkleProof<H> {
		let mut index: u64 = Self::get_index(leaf);
		let ret_index = index;
		let mut proofs = vec! {leaf.clone()};

		index += MOCK_MERKLE_TREE_LIMIT - 1;
		while index > 0 {
			let lr: bool = (index & 1) == 1;
			index = (index - 1) / 2;
			match lr {
				true => proofs.push(Self::get_hash(2 * index + 2)),    // left leafs.
				false => proofs = vec! {Self::get_hash(2 * index + 1)}
					.iter().chain(proofs.iter()).map(|x| *x).collect::<Vec<_>>(), // right leafs.
			}
		}
		MerkleProof {
			proofs: proofs,
			depth: MOCK_MERKLE_TREE_DEPTH,
			index: ret_index,
		}
	}

	fn push(leaf: H) {
		let h: H = Default::default();
		let x = Hashing::hash_of(&h);
		let cnt = Self::get_index(&x);
		Self::push_index(&x, cnt + 1);
		Self::push_hash(MOCK_MERKLE_TREE_LIMIT << 1 + cnt, leaf);
	}

	fn commit() {
		let h: H = Default::default();
		let x = Hashing::hash_of(&h);
		let cnt = Self::get_index(&x);
		Self::push_index(&x, 0);
		for i in 0..cnt {
			let leaf = Self::get_hash(MOCK_MERKLE_TREE_LIMIT << 1 + i);
			let mut index: u64 = Self::get_index(&h);
			Self::push_index(&Default::default(), index + 1); // increments...
			Self::push_index(&leaf, index);

			index += MOCK_MERKLE_TREE_LIMIT - 1;
			Self::push_hash(index, leaf);
			while index > 0 {
				index = (index - 1) / 2;
				Self::push_hash(index,
								concat_hash(&Self::get_hash(2 * index + 1),
											&Self::get_hash(2 * index + 2),
											Hashing::hash));
			}
		}
	}
}
