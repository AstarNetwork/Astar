use super::*;

use runtime_io::with_externalities;
use support::{impl_outer_origin, assert_ok};
use sr_primitives::{
	BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},
	testing::{Digest, DigestItem, Header},
};
use primitives::{sr25519, Pair, Blake2Hasher, H256};
use std::clone::Clone;

// plasm pritmitives uses plasm_primitives::mvp::Value
use plasm_primitives;
use plasm_merkle::{MerkleTreeTrait, ProofTrait};

pub use helper::{AccountId, Signature, MerkleTree, Value, TestInput, TestOutput, TestTransaction,
				 account_key_pair, gen_normal_tx, sign, genesis_tx};

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
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type Log = DigestItem;
}

impl Trait for Test {
	type Signature = Signature;
	type TimeLock = Self::BlockNumber;
	type Value = Value;

	type Input = TestInput;
	type Output = TestOutput;

	type Transaction = TestTransaction;
	type SignedTransaction = SignedTransaction<Test>;

	type Inserter = mvp::Inserter<Test, MerkleTree>;
	type Remover = DefaultRemover<Test>;
	type Finalizer = mvp::Finalizer<Test, MerkleTree>;

	type Event = ();
}

fn hash(tx: &<Test as Trait>::Transaction) -> <Test as system::Trait>::Hash {
	<Test as system::Trait>::Hashing::hash_of(tx)
}

// This function basically just builds ax genesis storage key/value store according to
// our desired mockup.
fn new_test_ext(root: &sr25519::Pair) -> runtime_io::TestExternalities<Blake2Hasher> {
	let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
	t.extend(GenesisConfig::<Test> {
		genesis_tx: genesis_tx::<Test>(root),
	}.build_storage().unwrap().0);
	t.into()
}

type UTXO = Module<Test>;

#[test]
fn mvp_minimum_works() {
	let root_key_pair = account_key_pair("test_root");
	with_externalities(&mut new_test_ext(&root_key_pair), || {
		// check merkle root ============================== different default
		let root_hash = MerkleTree::new().root();

		// check reference of genesis tx.
		let ref_utxo = <UnspentOutputsFinder<Test>>::get(root_key_pair.public());
		assert_eq!(1, ref_utxo.as_ref().unwrap().len());
		assert_eq!(0, ref_utxo.as_ref().unwrap()[0].1);
		let exp_gen_outpoint = ref_utxo.as_ref().unwrap()[0];

		// check genesis tx.
		let exp_gen_tx = &genesis_tx::<Test>(&root_key_pair)[0];
		let act_gen_out = <UnspentOutputs<Test>>::get(exp_gen_outpoint);
		assert_eq!(exp_gen_tx.0, act_gen_out.as_ref().unwrap().value());
		assert_eq!(1, act_gen_out.as_ref().unwrap().keys().len());
		assert_eq!(exp_gen_tx.1, act_gen_out.as_ref().unwrap().keys()[0]);

		// check total leftover is 0
		let leftover_total = <LeftoverTotal<Test>>::get();
		assert_eq!(0, *leftover_total);

		let receiver_key_pair = account_key_pair("test_receiver");
		let new_signed_tx = sign::<Test>(
			&gen_normal_tx(exp_gen_outpoint.0,
						   exp_gen_outpoint.1, Value::new(1 << 59), receiver_key_pair.public()),
			&root_key_pair,
		);
		assert_ok!(UTXO::execute(Origin::signed(root_key_pair.public()), new_signed_tx.encode()));

		// already spent genesis utxo.
		let spent_utxo = <UnspentOutputs<Test>>::get(exp_gen_outpoint);
		assert!(spent_utxo.is_none());
		// already spent reference of genesis utxo.
		let ref_utxo = <UnspentOutputsFinder<Test>>::get(root_key_pair.public());
		assert!(ref_utxo.is_none());

		// get new transaction.
		let act_gen_out2 = <UnspentOutputs<Test>>::get((hash(new_signed_tx.payload().as_ref().unwrap()), 0));
		assert!(act_gen_out2.is_some());
		assert_eq!(new_signed_tx.payload().as_ref().unwrap().outputs()[0],
				   act_gen_out2.unwrap());
		// get reference of new teranction.
		let ref_utxo = <UnspentOutputsFinder<Test>>::get(receiver_key_pair.public());
		assert!(ref_utxo.is_some());
		assert_eq!(1, ref_utxo.as_ref().unwrap().len());
		assert_eq!(hash(new_signed_tx.payload().as_ref().unwrap()), ref_utxo.as_ref().unwrap()[0].0);
		assert_eq!(0, ref_utxo.as_ref().unwrap()[0].1);

		// check total leftover is (1<<60) - (1<<59)
		let leftover_total = <LeftoverTotal<Test>>::get();
		assert_eq!((1 << 59), *leftover_total);

		// not yet change root hash ========================= different default TODO genesis tx is not exist (after that issue)
		assert_eq!(root_hash, MerkleTree::new().root());

		// on_finalize
		UTXO::on_finalize(1);

		// changed root hash ============================== different default VVV
		let new_root_hash = MerkleTree::new().root();
		assert_ne!(root_hash, new_root_hash);

		// proofs by ref utxo.
		let proofs = MerkleTree::new().proofs(&BlakeTwo256::hash_of(&ref_utxo.as_ref().unwrap()[0]));
		assert_eq!(new_root_hash, proofs.root::<BlakeTwo256>());
	});
}

//#[test]
//fn minimum_works() { // TODO fix divided tests.
//	let root_key_pair = account_key_pair("test_root");
//	let accounts = vec! {
//		account_key_pair("test_account_1").public(),
//		account_key_pair("test_account_2").public()};
//	with_externalities(&mut new_test_ext(&root_key_pair), || {
//		// check reference of genesis tx.
//		let ref_utxo = <UnspentOutputsFinder<Test>>::get(root_key_pair.public());
//		assert_eq!(1, ref_utxo.as_ref().unwrap().len());
//		assert_eq!(0, ref_utxo.as_ref().unwrap()[0].1);
//		let exp_gen_outpoint = ref_utxo.as_ref().unwrap()[0];
//
//		// check genesis tx.
//		let exp_gen_tx = &genesis_tx(&root_key_pair)[0];
//		let act_gen_out = <UnspentOutputs<Test>>::get(exp_gen_outpoint);
//		assert_eq!(exp_gen_tx.0, act_gen_out.as_ref().unwrap().value());
//		assert_eq!(1, act_gen_out.as_ref().unwrap().keys().len());
//		assert_eq!(exp_gen_tx.1, act_gen_out.as_ref().unwrap().keys()[0]);
//
//		// check total leftover is 0
//		let leftover_total = <LeftoverTotal<Test>>::get();
//		assert_eq!(0, *leftover_total);
//
//		let receiver_key_pair = account_key_pair("test_receiver");
//		let new_signed_tx = sign(
//			&gen_normal_tx(exp_gen_outpoint.0,
//						   exp_gen_outpoint.1, plasm_primitives::mvp::Value::new(1 << 59), receiver_key_pair.public()),
//			&root_key_pair,
//		);
//		assert_ok!(UTXO::execute(Origin::signed(1), new_signed_tx.encode()));
//
//		// already spent genesis utxo.
//		let spent_utxo = <UnspentOutputs<Test>>::get(exp_gen_outpoint);
//		assert!(spent_utxo.is_none());
//		// already spent reference of genesis utxo.
//		let ref_utxo = <UnspentOutputsFinder<Test>>::get(root_key_pair.public());
//		assert!(ref_utxo.is_none());
//
//		// get new transaction.
//		let act_gen_out2 = <UnspentOutputs<Test>>::get((hash(new_signed_tx.payload().as_ref().unwrap()), 0));
//		assert!(act_gen_out2.is_some());
//		assert_eq!(new_signed_tx.payload().as_ref().unwrap().outputs()[0],
//				   act_gen_out2.unwrap());
//		// get reference of new teranction.
//		let ref_utxo = <UnspentOutputsFinder<Test>>::get(receiver_key_pair.public());
//		assert!(ref_utxo.is_some());
//		assert_eq!(1, ref_utxo.as_ref().unwrap().len());
//		assert_eq!(hash(new_signed_tx.payload().as_ref().unwrap()), ref_utxo.as_ref().unwrap()[0].0);
//		assert_eq!(0, ref_utxo.as_ref().unwrap()[0].1);
//
//		// check total leftover is (1<<60) - (1<<59)
//		let leftover_total = <LeftoverTotal<Test>>::get();
//		assert_eq!((1 << 59), *leftover_total);
//
//		// on_finalize
//		UTXO::on_finalize(1);
//		// get reference of getting accounts leftover and get utxo.
//		for account in &accounts {
//			// ref utxo
//			let ref_utxo_account = <UnspentOutputsFinder<Test>>::get(account);
//			let ref_utxo_account = ref_utxo_account.unwrap();
//			assert_eq!(1, ref_utxo_account.len());
//
//			// utxo
//			let utxo_account = <UnspentOutputs<Test>>::get(ref_utxo_account[0]);
//			let utxo_account = utxo_account.unwrap();
//			// value is (1<<59)/2 = (1<<58);
//			assert_eq!((1 << 58), *utxo_account.value());
//			// keys = {account}
//			assert_eq!(1, utxo_account.keys().len());
//			assert_eq!(account, &utxo_account.keys()[0]);
//		}
//
//		// check total leftover is 0 after finalize
//		let leftover_total = <LeftoverTotal<Test>>::get();
//		assert_eq!(0, *leftover_total);
//	});
//}
