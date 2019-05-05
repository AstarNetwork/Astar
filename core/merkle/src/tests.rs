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
	MerkleDb::<u64, H256>::push(&DirectMerkleDb, b":child_storage:default:test_db", &key, value);
}

fn test_db_get(key: u64) -> Option<H256> {
	MerkleDb::<u64, H256>::get(&DirectMerkleDb, b":child_storage:default:test_db", &key)
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

fn merkle_test<Tree, H, Hashing, F>(tree: Tree, rnd: F)
	where Tree: MerkleTreeTrait<H, Hashing>,
		  H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>,
		  F: Fn() -> H
{
	let hashes = (0..10).map(|_| rnd()).collect::<Vec<_>>();
	assert_eq!(10, hashes.len());

	for i in 0..10 {
		tree.push(hashes[i].clone());
		tree.commit();
		for j in 0..(i + 1) {
			let proofs = tree.proofs(&hashes[j]).unwrap();
			println!("{:?}", proofs);
			assert_eq!(&hashes[j], tree.proofs(&hashes[j]).unwrap().leaf());
			assert_eq!(tree.root(), proofs.root::<Hashing>())
		}
	}
	assert_eq!(None, tree.proofs(&rnd()));

	let new_hashes = (0..10).map(|_| rnd()).collect::<Vec<_>>();
	for i in 0..10 {
		tree.push(new_hashes[i]);
	}
	tree.commit();
	for i in 0..10 {
		assert_eq!(&hashes[i], tree.proofs(&hashes[i]).unwrap().leaf());
		assert_eq!(tree.root(), tree.proofs(&hashes[i]).unwrap().root::<Hashing>());
		assert_eq!(tree.root(), tree.proofs(&new_hashes[i]).unwrap().root::<Hashing>());
	}
}

#[test]
fn merkle_mock_test() {
	with_externalities(&mut new_test_ext(), || {
		type MerkleTree = mock::MerkleTree<H256, BlakeTwo256>;
		merkle_test::<MerkleTree, H256, BlakeTwo256, fn() -> H256>(MerkleTree::new(), H256::random);
	});
}

fn recover_merkle_test<Tree, H, Hashing, F>(tree: Tree, rnd: F)
	where Tree: RecoverableMerkleTreeTrait<H, Hashing>,
		  H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>,
		  F: Fn() -> H
{
	let hashes = (0..10).map(|_| rnd()).collect::<Vec<_>>();
	assert_eq!(10, hashes.len());

	for i in 0..10 {
		tree.push(hashes[i].clone());
		tree.commit();
		for j in 0..(i + 1) {
			let proofs = tree.proofs(&hashes[j]).unwrap();
			println!("{:?}", proofs);
			assert_eq!(&hashes[j], tree.proofs(&hashes[j]).unwrap().leaf());
			assert_eq!(tree.root(), proofs.root::<Hashing>())
		}
	}
	let proofs_a = (0..10).map(|i| tree.proofs(&hashes[i]).unwrap()).collect::<Vec<_>>();
	tree.save();
	let root_a = tree.root();

	let new_hashes = (0..10).map(|_| rnd()).collect::<Vec<_>>();
	for i in 0..10 {
		tree.push(new_hashes[i]);
	}
	tree.commit();
	for i in 0..10 {
		assert_eq!(&hashes[i], tree.proofs(&hashes[i]).unwrap().leaf());
		assert_eq!(tree.root(), tree.proofs(&hashes[i]).unwrap().root::<Hashing>());
		assert_eq!(tree.root(), tree.proofs(&new_hashes[i]).unwrap().root::<Hashing>());
	}
	let root_b = tree.root();
	assert!(Tree::load(&root_b).is_none());
	assert_eq!(None, tree.proofs(&rnd()));
	tree.save();

	let tree_a = Tree::load(&root_a).unwrap();
	assert_eq!(root_a, tree_a.root());
	let proofs_a_act = (0..10).map(|i| tree_a.proofs(&hashes[i]).unwrap()).collect::<Vec<_>>();
	assert_eq!(proofs_a, proofs_a_act);
}

#[test]
fn recover_merkle_mock_test() {
	with_externalities(&mut new_test_ext(), || {
		type MerkleTree = mock::MerkleTree<H256, BlakeTwo256>;
		recover_merkle_test::<MerkleTree, H256, BlakeTwo256, fn() -> H256>(MerkleTree::new(), H256::random);
	});
}


#[test]
fn check_merkle_trie_id() {
	assert!(mock::MOCK_MERKLE_TREE_TRIE_ID.starts_with(b":child_storage:default:"));
}
