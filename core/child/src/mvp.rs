use sr_primitives::traits::{Hash, As};

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, ensure};
use system::ensure_signed;

use merkle::{MerkleTreeTrait, RecoverableMerkleTreeTrait, ReadOnlyMerkleTreeTrait, ProofTrait};
use utxo::{UtxoTrait, WritableUtxoTrait, mvp::{SignedTx, Module as UtxoModule}};


/// The module's configuration trait.
pub trait Trait: utxo::mvp::Trait {
	// TODO : utxo will be not srml. type Utxo;
	type Tree: RecoverableMerkleTreeTrait<Self::Hash, Self::Hashing>;
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Child {
		ChildChain get(child_chain): map T::BlockNumber => Option<T::Hash>;
		CurrentBlock get(current_block): T::BlockNumber = T::BlockNumber::sa(0);
		Operators get(operators) config(): Vec<T::AccountId>;
		SubmitInterval get(submit_interval) config(): T::BlockNumber;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		// commit (change childchain)
		pub fn commit(origin, blk_num: T::BlockNumber, hash: T::Hash) -> Result {
			let operator = ensure_signed(origin)?;
			ensure!(
				Self::operators().contains(&operator),
				"Signer is not operators.");

			<ChildChain<T>>::insert(&blk_num, hash);
			<CurrentBlock<T>>::put(blk_num.clone());
			Self::deposit_event(RawEvent::Commit(blk_num, hash));
			Ok(())
		}

		// deposit () verify and execute(operator -> depositor) by Utxo.
		pub fn deposit(origin, signed_tx: SignedTx<T>) -> Result {
			ensure_signed(origin)?;
			ensure!(signed_tx
				.public_keys
				.iter()
				.filter(|key| Self::operators().contains(key))
				.count() == signed_tx.public_keys.len(),
				"Sender is not operators.");

			<UtxoModule<T>>::exec(signed_tx.clone())?;
			Self::push_tree(&signed_tx);
			Ok(())
		}

		pub fn execute(origin, signed_tx: SignedTx<T>) -> Result {
			ensure_signed(origin)?;
			<UtxoModule<T>>::exec(signed_tx.clone())?;
			Self::push_tree(&signed_tx);
			Ok(())
		}

		// exitStart () delete exitor utxo by Utxo.
		pub fn exit_start(origin, tx_hash: T::Hash, out_index: u32) -> Result {
			let operator = ensure_signed(origin)?;
			ensure!(
				Self::operators().contains(&operator),
				"Signer is not operators.");

			let utxo = <UtxoModule<T>>::unspent_outputs(&(tx_hash.clone(), out_index)).ok_or("Unexist exit utxo.")?;
			let identify = (tx_hash.clone(), out_index);
			for key in utxo.keys.iter() {
				<UtxoModule<T>>::remove_finder(key, &identify);
			}
			<UtxoModule<T>>::remove(&identify);
			Self::deposit_event(RawEvent::ExitStart(tx_hash, out_index));
			Ok(())
		}

		// deposit_event(RawEvent(Proof(blk_num, tx_hash, out_index, proofs, depth, index));
		pub fn get_proof(origin, blk_num: T::BlockNumber, tx_hash: T::Hash, out_index: u32) -> Result {
			ensure_signed(origin)?;
			let hash = Self::child_chain(blk_num).ok_or("unexists block number.")?;

			let tree = T::Tree::load(&hash).ok_or("unexistt root hash.")?;
			let proof = tree.proofs(&T::Hashing::hash_of(&(tx_hash.clone(), out_index))).ok_or("unexist leaf.")?;
			Self::deposit_event(RawEvent::Proof(blk_num, tx_hash, out_index, proof.proofs().clone(), proof.depth() as u32, proof.index()));
			Ok(())
		}

		pub fn on_finalize() {
			<UtxoModule<T>>::deal(&Self::operators());
			let tree = T::Tree::new();
			tree.commit();
			tree.save();
			Self::deposit_event(RawEvent::Submit(tree.root()));
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event<T>
		where
			Hash = <T as system::Trait>::Hash,
			BlockNumber = <T as system::Trait>::BlockNumber,
			AccountId = <T as system::Trait>::AccountId,
			Value = <T as utxo::mvp::Trait>::Value {
		Submit(Hash),
		Commit(BlockNumber, Hash),
		Deposit(AccountId, Value),
		ExitStart(Hash, u32),
		// blocknumber, tx_hash, out_index, proofs, depth, index
		Proof(BlockNumber, Hash, u32, Vec<Hash>, u32, u64),
	}
);

impl<T: Trait> Module<T> {
	fn push_tree(signed_tx: &SignedTx<T>) {
		let hash = T::Hashing::hash_of(&signed_tx.payload);
		for (i, _) in signed_tx.payload.outputs.iter().enumerate() {
			T::Tree::new().push(T::Hashing::hash_of(&(hash.clone(), i as u32)));
		}
	}
}

#[cfg(all(test, feature = "std"))]
mod tests {
	use super::*;
	use runtime_io::with_externalities;

	use support::impl_outer_origin;
	use sr_primitives::{
		BuildStorage,
		traits::{Verify, BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header},
	};
	use primitives::{Blake2Hasher, H256, sr25519, crypto::Pair};
	use parity_codec::{Encode, Decode};
	use std::clone::Clone;

	use utxo::{impl_mvp_test_helper, mvp::{
		UnspentOutputsFinder, SignedTransaction, Transaction, TransactionOutput, TransactionInput, SignedTx, Tx, TxOut, TxIn}};
	use merkle::{MerkleProof, ProofTrait};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct Test;

	pub type Signature = sr25519::Signature;
	// TODO must be sr25519 only used by wasm.
	pub type AccountId = <Signature as Verify>::Signer;

	pub type MerkleTree = merkle::mock::MerkleTree<H256, BlakeTwo256>;

	impl_mvp_test_helper!(Test, utxo::mvp::UnspentOutputs<Test>);

	#[derive(Clone, PartialEq, Eq, Encode, Decode)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub enum TestEvent {
		Some(RawEvent<H256, u64, AccountId, u64>),
		None,
	}

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
		type Event = TestEvent;
		type Log = DigestItem;
	}

	impl utxo::mvp::Trait for Test {
		type Signature = Signature;
		type TimeLock = Self::BlockNumber;
		type Value = u64;

		type Event = TestEvent;
	}

	impl Trait for Test {
		type Tree = MerkleTree;
		type Event = TestEvent;
	}

	impl From<system::Event> for TestEvent {
		fn from(_e: system::Event) -> TestEvent {
			TestEvent::None
		}
	}

	impl From<utxo::mvp::Event<Test>> for TestEvent {
		fn from(_e: utxo::mvp::Event<Test>) -> TestEvent {
			TestEvent::None
		}
	}

	impl From<Event<Test>> for TestEvent {
		fn from(e: Event<Test>) -> TestEvent {
			TestEvent::Some(e)
		}
	}

	type Child = Module<Test>;
	type Utxo = UtxoModule<Test>;

	fn new_test_ext(operator: &sr25519::Pair) -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(utxo::mvp::GenesisConfig::<Test> {
			genesis_tx: genesis_tx(operator),
		}.build_storage().unwrap().0);
		t.extend(GenesisConfig::<Test> {
			operators: vec! {operator.public().clone()},
			submit_interval: 1,
		}.build_storage().unwrap().0);
		t.into()
	}

	#[test]
	fn test_commit() {
		let operator_pair = account_key_pair("operator");
		with_externalities(&mut new_test_ext(&operator_pair), || {
			let random_hash = H256::random();

			assert_eq!(Ok(()), Child::commit(Origin::signed(operator_pair.public()), 1, random_hash.clone()));

			let current = <CurrentBlock<Test>>::get();
			let root = <ChildChain<Test>>::get(&1).unwrap();
			assert_eq!(1, current);
			assert_eq!(random_hash, root);
		});
	}

	#[test]
	fn test_deposit() {
		let operator_pair = account_key_pair("operator");
		with_externalities(&mut new_test_ext(&operator_pair), || {

			// check deposit.
			let receiver_key_pair = account_key_pair("test_receiver");
			let signed_tx = gen_transfer(&operator_pair, &receiver_key_pair.public(), 100000);
			assert_eq!(Ok(()), Child::deposit(Origin::signed(receiver_key_pair.public()), signed_tx));

			// invalid deposit transfer from no operator.
			let invalid_signed_tx = gen_transfer(&receiver_key_pair, &operator_pair.public(), 1000);
			assert_eq!(Err("Sender is not operators."), Child::deposit(Origin::signed(operator_pair.public()), invalid_signed_tx));
		});
	}

	#[test]
	fn test_exit_start() {
		let operator_pair = account_key_pair("operator");
		with_externalities(&mut new_test_ext(&operator_pair), || {
			// check deposit.
			let receiver_key_pair = account_key_pair("test_receiver");
			let signed_tx = gen_transfer(&operator_pair, &receiver_key_pair.public(), 100000);
			assert_eq!(Ok(()), Child::deposit(Origin::signed(receiver_key_pair.public()), signed_tx));

			// invalid deposit transfer from no operator.
			let invalid_signed_tx = gen_transfer(&receiver_key_pair, &operator_pair.public(), 1000);
			assert_eq!(Err("Sender is not operators."), Child::deposit(Origin::signed(operator_pair.public()), invalid_signed_tx));

			let tx_ref = Utxo::unspent_outputs_finder(&receiver_key_pair.public()).unwrap()[0];
			assert_eq!(Ok(()), Child::exit_start(Origin::signed(operator_pair.public()), tx_ref.0, tx_ref.1));
			assert_eq!(None, Utxo::unspent_outputs_finder(&receiver_key_pair.public()));
			assert_eq!(None, Utxo::unspent_outputs(&(tx_ref.0, tx_ref.1)));
		});
	}

	fn get_events() -> Vec<TestEvent> {
		<system::Module<Test>>::events()
			.iter()
			.filter(|e|
				match &e.event {
					TestEvent::Some(_) => true,
					_ => false,
				})
			.cloned()
			.map(|e| e.event)
			.collect::<Vec<_>>()
	}

	fn get_submit_hash_from_events() -> H256 {
		for e in get_events() {
			if let TestEvent::Some(RawEvent::Submit(hash)) = e {
				return hash;
			}
		}
		Default::default()
	}

	fn get_proofs_from_events() -> (u64, H256, u32, Vec<H256>, u32, u64) {
		for e in get_events() {
			if let TestEvent::Some(RawEvent::Proof(blk_num, hash, out, proof, depth, index)) = e {
				return (blk_num, hash, out, proof, depth, index);
			}
		}
		(Default::default(), Default::default(), Default::default(), Default::default(), Default::default(), Default::default())
	}

	#[test]
	fn test_finalize_and_get_proofs() {
		let operator_pair = account_key_pair("operator");
		with_externalities(&mut new_test_ext(&operator_pair), || {
			// transfer operator -> test_receiver;
			let receiver_1_key_pair = account_key_pair("test_receiver");
			let signed_tx_1 = gen_transfer(&operator_pair, &receiver_1_key_pair.public(), 200000);
			let tx_1_hash = BlakeTwo256::hash_of(&signed_tx_1.payload);
			let utxo_1_hash = BlakeTwo256::hash_of(&(tx_1_hash.clone(), 0));
			assert_eq!(Ok(()), Child::deposit(Origin::signed(receiver_1_key_pair.public()), signed_tx_1.clone()));

			// save merkle tree
			assert_eq!(Ok(()), Child::on_finalize());

			let root_hash_1 = MerkleTree::new().root();
			println!("root_hash_1 {:?}", root_hash_1);
			println!("proofs: {:?}", MerkleTree::new().proofs(&utxo_1_hash));
			let submit_hash_1 = get_submit_hash_from_events();
			assert_eq!(root_hash_1, submit_hash_1);
			assert_eq!(Ok(()), Child::commit(Origin::signed(operator_pair.public().clone()), 1, root_hash_1));

			// events killed.
			<system::Module<Test>>::initialize(&1, &[0u8; 32].into(), &[0u8; 32].into());

			// transfer test_receiver -> test_receiver_2
			let receiver_2_key_pair = account_key_pair("test_receiver_2");
			let signed_tx_2 = gen_transfer(&receiver_1_key_pair, &receiver_2_key_pair.public(), 100000);
			assert_eq!(Ok(()), Child::execute(Origin::signed(receiver_1_key_pair.public().clone()), signed_tx_2));

			// save merkle tree
			assert_eq!(Ok(()), Child::on_finalize());

			let root_hash_2 = MerkleTree::new().root();
			let submit_hash_2 = get_submit_hash_from_events();
			assert_ne!(root_hash_1, root_hash_2);
			assert_ne!(submit_hash_1, submit_hash_2);
			assert_eq!(root_hash_2, submit_hash_2);
			assert_eq!(Ok(()), Child::commit(Origin::signed(operator_pair.public().clone()), 2, root_hash_2));


			//GetProofs 1
			assert_eq!(Ok(()),
					   Child::get_proof(Origin::signed(operator_pair.public().clone()),
										1, tx_1_hash, 0));
			let (blk_num, hash, out, proofs, depth, index) = get_proofs_from_events();
			assert_eq!(1, blk_num);
			assert_eq!(tx_1_hash, hash);
			assert_eq!(0, out);

			let proof = MerkleProof::<H256> { proofs, depth, index };
			assert_eq!(&BlakeTwo256::hash_of(&(tx_1_hash.clone(), 0)), proof.leaf());
			assert_eq!(root_hash_1, proof.root::<BlakeTwo256>());
		});
	}
}
