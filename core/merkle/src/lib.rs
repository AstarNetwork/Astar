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
	hash(&plasm_primitives::concat_bytes(a, b))
}

pub trait ReadOnlyMerkleTreeTrait<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	/// get root Hash of MerkleTree.
	fn root(&self) -> H;
	/// get proofs of leaf.
	fn proofs(&self, leaf: &H) -> Option<MerkleProof<H>>;
}

// H: Hash, O: Outpoint(Hashable)
pub trait MerkleTreeTrait<H, Hashing>: ReadOnlyMerkleTreeTrait<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	fn new() -> Self;
	/// push Hash to MerkleTree.
	fn push(&self, leaf: H);
	// commit to MerkleTree
	fn commit(&self);
}


pub trait RecoverableMerkleTreeTrait<H, Hashing>: MerkleTreeTrait<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	type Out: ReadOnlyMerkleTreeTrait<H, Hashing> + PartialEq;
	fn load(root: &H) -> Option<Self::Out>;
	fn save(&self);
}

pub trait MerkleDb<Key: Encode, O: Codec> {
	fn push(&self, trie_id: &[u8], key: &Key, o: O) {
		child::put_raw(trie_id, &key.encode()[..], &o.encode()[..]);
	}
	fn get(&self, trie_id: &[u8], key: &Key) -> Option<O> {
		if let Some(ret) = child::get_raw(trie_id, &key.encode()[..]) {
			return O::decode(&mut &ret[..]);
		}
		return None;
	}
}

pub struct DirectMerkleDb;

impl<Key: Encode, O: Codec> MerkleDb<Key, O> for DirectMerkleDb {}

pub trait ProofTrait<H>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default {
	fn root<Hashing>(&self) -> H where Hashing: Hash<Output=H>;
	fn leaf(&self) -> &H;
	fn proofs(&self) -> &Vec<H>;
	fn depth(&self) -> u32;
	fn index(&self) -> u64;
}

#[derive(PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MerkleProof<H> {
	pub proofs: Vec<H>,
	pub depth: u32,
	pub index: u64,
}

impl<H> MerkleProof<H>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default
{
	fn re_root<Hashing>(&self, mid: u64, now_l: u32, now_r: u32) -> H
		where Hashing: Hash<Output=H>
	{
		if now_r - now_l == 1 {
			return concat_hash(&self.proofs[now_l as usize], &self.proofs[now_r as usize], Hashing::hash);
		}
		let now_depth = self.proofs.len() as u64 - now_r as u64 + now_l as u64;
		let new_mid = (1u64 << self.depth >> now_depth) + mid;
		if new_mid <= self.index {
			return concat_hash(&self.proofs[now_l as usize],
							   &self.re_root::<Hashing>(new_mid, now_l + 1, now_r),
							   Hashing::hash);
		} else {
			return concat_hash(&self.re_root::<Hashing>(mid, now_l, now_r - 1),
							   &self.proofs[now_r as usize],
							   Hashing::hash);
		}
	}

	fn re_leaf(&self, mid: u64, now_l: u32, now_r: u32) -> &H {
		if now_r == now_l {
			return &self.proofs[now_r as usize];
		}
		let now_depth = self.proofs.len() as u64 - now_r as u64 + now_l as u64;
		let new_mid = (1u64 << self.depth >> now_depth) + mid;
		if new_mid <= self.index {
			return &self.re_leaf(new_mid, now_l + 1, now_r);
		} else {
			return &self.re_leaf(mid, now_l, now_r - 1);
		}
	}
}

impl<H> ProofTrait<H> for MerkleProof<H>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default {
	fn root<Hashing>(&self) -> H
		where Hashing: Hash<Output=H>
	{
		self.re_root::<Hashing>(0, 0, self.proofs.len() as u32 - 1)
	}

	fn leaf(&self) -> &H { self.re_leaf(0, 0, self.proofs.len() as u32 - 1) }

	fn proofs(&self) -> &Vec<H> { &self.proofs }
	fn depth(&self) -> u32 { self.depth }
	fn index(&self) -> u64 { self.index }
}

#[cfg(test)]
mod tests;
