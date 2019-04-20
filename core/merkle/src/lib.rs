#![cfg_attr(not(feature = "std"), no_std)]

use rstd::prelude::*;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash};
use support::storage::child;
use parity_codec::{Encode, Codec};

pub mod mock;

pub fn concat_hash<H, F>(a: &H, b: &H, hash: F) -> H
	where H: Encode + Default + Eq + Copy,
		  F: FnOnce(&[u8]) -> H {
	if *a == Default::default() { return *b; }
	if *b == Default::default() { return *a; }
	hash(&a.encode().iter().chain(b.encode().iter()).map(|x| *x).collect::<Vec<_>>())
}

// H: Hash, O: Outpoint(Hashable)
pub trait MerkleTreeTrait<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	/// get root Hash of MerkleTree.
	fn root() -> H;
	/// get proofs of leaf.
	fn proofs(leaf: &H) -> MerkleProof<H>;
	/// push Hash to MerkleTree.
	fn push(leaf: H);
	// commit to MerkleTree
	fn commit();
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

#[derive(Debug)]
pub struct MerkleProof<H> {
	proofs: Vec<H>,
	depth: u8,
	index: u64,
}

impl<H> MerkleProof<H>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default
{
	pub fn verify<Hashing>(&self) -> H
		where Hashing: Hash<Output=H>
	{
		self.re_verify::<Hashing>(0, 0, self.proofs.len() - 1)
	}

	fn re_verify<Hashing>(&self, mid: u64, now_l: usize, now_r: usize) -> H
		where Hashing: Hash<Output=H>
	{
		if now_r - now_l == 1 {
			return concat_hash(&self.proofs[now_l], &self.proofs[now_r], Hashing::hash);
		}
		let now_depth = self.proofs.len() as u64 - now_r as u64 + now_l as u64;
		let new_mid = (1u64 << self.depth >> now_depth) + mid;
		if new_mid <= self.index {
			return concat_hash(&self.proofs[now_l],
							   &self.re_verify::<Hashing>(new_mid, now_l + 1, now_r),
							   Hashing::hash);
		} else {
			return concat_hash(&self.re_verify::<Hashing>(mid, now_l, now_r - 1),
							   &self.proofs[now_r],
							   Hashing::hash);
		}
	}
}

#[cfg(test)]
mod tests;
