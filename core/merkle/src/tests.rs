use super::*;
use runtime_io::with_externalities;

use support::impl_outer_origin;
use sr_primitives::{
	BuildStorage,
	traits::{BlakeTwo256, IdentityLookup, Hash},
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


#[test]
fn concat_test() {
	let a = H256::random();
	let b = H256::random();
	let c = H256::random();
	assert_eq!(concat_hash(&concat_hash(&a, &b, BlakeTwo256::hash), &c, BlakeTwo256::hash),
			   concat_hash(&concat_hash(&a, &b, BlakeTwo256::hash), &c, BlakeTwo256::hash));

	assert_eq!(a, concat_hash(&a, &Default::default(), BlakeTwo256::hash));
	assert_eq!(b, concat_hash(&Default::default(), &b, BlakeTwo256::hash));
}

fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
	system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
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

fn merkle_test<Tree, H, Hashing, F>(rnd: F)
	where Tree: MerkleTreeTrait<H, Hashing>,
		  H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>,
		  F: Fn() -> H
{
	let hashes = (0..10).map(|_| rnd()).collect::<Vec<_>>();
	assert_eq!(10, hashes.len());

	for i in 0..10 {
		Tree::push(hashes[i].clone());
		Tree::commit();
		for j in 0..(i + 1) {
			let proofs = Tree::proofs(&hashes[j]);
			println!("{:?}", proofs);
			assert_eq!(&hashes[j], Tree::proofs(&hashes[j]).leaf());
			assert_eq!(Tree::root(), proofs.root::<Hashing>())
		}
	}

	let new_hashes = (0..10).map(|_| rnd()).collect::<Vec<_>>();
	for i in 0..10 {
		Tree::push(new_hashes[i]);
	}
	Tree::commit();
	for i in 0..10 {
		assert_eq!(&hashes[i], Tree::proofs(&hashes[i]).leaf());
		assert_eq!(Tree::root(), Tree::proofs(&hashes[i]).root::<Hashing>());
		assert_eq!(Tree::root(), Tree::proofs(&new_hashes[i]).root::<Hashing>());
	}
}

#[test]
fn merkle_mock_test() {
	with_externalities(&mut new_test_ext(), || {
		type MerkleTree = mock::MerkleTree<H256, BlakeTwo256>;
		merkle_test::<MerkleTree, H256, BlakeTwo256, fn() -> H256>(H256::random);
	});
}
