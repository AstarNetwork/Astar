use super::*;
use runtime_io::with_externalities;

use support::{impl_outer_origin};
use sr_primitives::{
	BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
	testing::{Digest, DigestItem, Header},
};
use primitives::{Blake2Hasher, H256};
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
	merkle_cocnat_hash::<H256, mock::ConcatHasher, fn() -> H256>(|| { H256::random() });
}

fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
	system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
}

fn mock_verify(proofs: mock::Proofs<H256>) -> H256 {
	match proofs {
		mock::Proofs::<H256>::Leaf(leaf) => leaf,
		mock::Proofs::<H256>::Node(left, right) => mock::ConcatHasher::concat::<H256>(
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
		for i in 0..100 {
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
fn merkle_mock_onece_test() {
	with_externalities(&mut new_test_ext(), || {
		type MerkleTree = mock::MerkleTree<H256>;
		let a = H256::random();
		let b = H256::random();
		let c = H256::random();

		MerkleTree::push(a);
		assert_eq!(a, MerkleTree::root());
		assert_eq!(a, mock_verify(MerkleTree::proofs(&a)));

		MerkleTree::push(b);
		assert_eq!(mock::ConcatHasher::concat(a, b), MerkleTree::root());
		assert_eq!(MerkleTree::root(), mock_verify(MerkleTree::proofs(&a)));
		assert_eq!(MerkleTree::root(), mock_verify(MerkleTree::proofs(&b)));

		MerkleTree::push(c);
		assert_eq!(mock::ConcatHasher::concat(mock::ConcatHasher::concat(a, b), c), MerkleTree::root());
		assert_eq!(MerkleTree::root(), mock_verify(MerkleTree::proofs(&a)));
		assert_eq!(MerkleTree::root(), mock_verify(MerkleTree::proofs(&b)));
		assert_eq!(MerkleTree::root(), mock_verify(MerkleTree::proofs(&c)));
	});
}

#[test]
fn merkle_mock_test() {
	with_externalities(&mut new_test_ext(), || {
		type MerkleTree = mock::MerkleTree<H256>;
		let hashes = (0..10).map(|_| H256::random()).collect::<Vec<_>>();
		assert_eq!(10, hashes.len());

		for h in hashes.iter() {
			MerkleTree::push(h.clone())
		}

		// verify
		let root_hash = MerkleTree::root();
		for i in 0..10 {
			let proofs = MerkleTree::proofs(&hashes[i]);
			assert_eq!(root_hash, mock_verify(proofs));
		}
	});
}
