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
		return None;
	}
}

pub struct DirectMerkleDb;

impl<Id: Encode, Key: Encode, O: Codec> MerkleDb<Id, Key, O> for DirectMerkleDb {}

#[cfg(test)]
mod tests {
	use super::*;
	use runtime_io::with_externalities;

	use support::{impl_outer_origin, assert_ok};
	use sr_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header},
	};
	use primitives::{ed25519, Pair, Blake2Hasher, H256};
	use std::clone::Clone;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct Test;

	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
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

	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	fn mock_verify(proofs: MockProofs<H256>) -> H256 {
		match proofs {
			MockProofs::<H256>::Leaf(leaf) => leaf,
			MockProofs::<H256>::Node(left, right) => MockConcatHasher::concat::<H256>(
				mock_verify(*left),
				mock_verify(*right))
		}
	}

	fn test_db_push(key: u64, value: H256) {
		MerkleDb::<&'static str, u64, H256>::push(&DirectMerkleDb, &"test_db", &key, value);
	}

	fn test_db_get(key: u64) -> Option<H256> {
		MerkleDb::<&'static str, u64, H256>::get(&DirectMerkleDb, &"test_db", &key)
	}

	#[test]
	fn merkle_mock_db() {
		with_externalities(&mut new_test_ext(), || {
			for i in (0..100) {
				let k = i as u64;
				let v = H256::random();
				test_db_push(k, v.clone());
				assert_eq!(Some(v), test_db_get(k));
			}
			// nothing key 114514
			assert_eq!(None, test_db_get(114514));
		});
	}

	#[test]
	fn merkle_mock_test() {
		with_externalities(&mut new_test_ext(), || {
			type MerkleTree = MockMerkleTree<H256>;
			let hashes = (0..10).map(|_| H256::random()).collect::<Vec<_>>();
			assert_eq!(10, hashes.len());

			hashes.iter()
				.inspect(|h| MerkleTree::push(*h.clone()))
				.count();

			// verify
			let root_hash = MerkleTree::root();
			for i in 0..10 {
				println!("{}", i);
				let proofs = MerkleTree::proofs(&hashes[i]);
				assert_eq!(root_hash, mock_verify(proofs));
			}
		});
	}
}
