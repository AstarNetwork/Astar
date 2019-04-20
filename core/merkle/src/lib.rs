use rstd::prelude::*;
use sr_primitives::traits::SimpleBitOps;
use support::storage::child;
use parity_codec::{Encode, Codec};

pub mod mock;

pub trait ConcatHash {
	fn concat<T>(a: T, b: T) -> T
		where T: SimpleBitOps + Eq + Default + Copy;
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

pub trait MerkleDb<Id: Encode, Key: Encode, O: Codec> {
	fn push(&self, trie_id: &Id, key: &Key, o: O) {
		child::put_raw(&trie_id.encode()[..], &key.encode()[..], &o.encode()[..]);
	}
	fn get(&self, trie_id: &Id, key: &Key) -> Option<O> {
		if let Some(ret) = child::get_raw(&trie_id.encode()[..], &key.encode()[..]) {
			return O::decode(&mut &ret[..]);
		}
		return None;
	}
}

pub struct DirectMerkleDb;

impl<Id: Encode, Key: Encode, O: Codec> MerkleDb<Id, Key, O> for DirectMerkleDb {}

#[cfg(test)]
mod tests;
