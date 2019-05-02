use sr_primitives::traits::{As, Hash, Member, MaybeSerializeDebug};

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result, ensure};
use system::ensure_signed;

use parity_codec::{Encode, Decode};

use merkle::MerkleTreeTrait;
use utxo::{SignedTransactionTrait, TransactionOutputTrait};


/// The module's configuration trait.
pub trait Trait: utxo::Trait {
	// TODO : utxo will be not srml. type Utxo;
	type Tree: MerkleTreeTrait<Self::Hash, Self::Hashing>;
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

		// deposit () verfy and execute(operator -> depositor) by Utxo.
		pub fn deposit(origin, signed_tx: T::SignedTransaction) -> Result {
			let operator = ensure_signed(origin)?;

			ensure!(signed_tx
				.public_keys()
				.iter()
				.filter(|key| Self::operators().contains(key))
				.count() == signed_tx.public_keys().len(),
				"Signer is not operators.");

			<utxo::Module<T>>::do_execute(signed_tx)
		}


		// exitStart () delete exitor utxo by Utxo.
		pub fn exit_start(origin, tx_hash: T::Hash, out_index: u32) -> Result {
			let operator = ensure_signed(origin)?;
			ensure!(
				Self::operators().contains(&operator),
				"Signer is not operators.");

			let utxo = <utxo::Module<T>>::unspent_outputs(&(tx_hash, out_index)).ok_or("Unexist exit utxo.")?;
			for key in utxo.keys().iter() {
				<utxo::UnspentOutputsFinder<T>>::mutate(key, |v| {
					*v = match
						v.as_ref()
							.unwrap_or(&vec! {})
							.iter()
							.filter(|e| **e != (tx_hash, out_index))
							.map(|e| *e)
							.collect::<Vec<_>>()
							.as_slice() {
						[] => None,
						s => Some(s.to_vec()),
					}
				})
			}
			<utxo::UnspentOutputs<T>>::remove(&(tx_hash, out_index));
			Self::deposit_event(RawEvent::ExitStart(tx_hash, out_index));
			Ok(())
		}

		// deposit_event(RawEvent(Proof(blk_num, tx_hash, out_index, proofs, depth, index));
		pub fn get_proof(origin, blk_num: T::BlockNumber, tx_hash: T::Hash, out_index: u32) -> Result {
			ensure_signed(origin)?;
			let hash = Self::child_chain(blk_num).ok_or("unexists block number.")


			Ok(())
		}

		fn on_finalize(n_: T::BlockNumber) {
			Self::deposit_event(RawEvent::Submit(T::Tree::root()));
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
			Value = <T as utxo::Trait>::Value {
		Submit(Hash),
		Commit(BlockNumber, Hash),
		Deposit(AccountId, Value),
		ExitStart(Hash, u32),
		// blocknumber, hash, index, proofs, depth, index
		Proof(BlockNumber, Hash, u32, Vec<Hash>, u32, u64),
	}
);

#[cfg(all(test, feature = "std"))]
mod tests {
	use super::*;
	use runtime_io::with_externalities;

	use support::impl_outer_origin;
	use sr_primitives::{
		BuildStorage,
		traits::{Verify, BlakeTwo256, IdentityLookup, Hash},
		testing::{Digest, DigestItem, Header},
	};
	use primitives::{Blake2Hasher, H256, sr25519, crypto::Pair};
	use std::clone::Clone;

	use utxo::{TransactionInputTrait, TransactionOutputTrait, TransactionTrait, SignedTransactionTrait,
			   TransactionInput, TransactionOutput, Transaction, SignedTransaction};

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

	impl utxo::Trait for Test {
		type Signature = Signature;
		type TimeLock = Self::BlockNumber;
		type Value = utxo::mvp::Value;

		type Input = utxo::helper::TestInput;
		type Output = utxo::helper::TestOutput;

		type Transaction = utxo::helper::TestTransaction;
		type SignedTransaction = SignedTransaction<Test>;

		type Inserter = utxo::mvp::Inserter<Test, MerkleTree>;
		type Remover = utxo::DefaultRemover<Test>;
		type Finalizer = utxo::mvp::Finalizer<Test, MerkleTree>;

		type Event = ();
	}

	impl Trait for Test {
		type Tree = MerkleTree;
		type Event = ();
	}

	type Child = Module<Test>;
	type Utxo = utxo::Module<Test>;

	fn new_test_ext(operator: &sr25519::Pair) -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(utxo::GenesisConfig::<Test> {
			genesis_tx: utxo::helper::genesis_tx::<Test>(operator),
		}.build_storage().unwrap().0);
		t.extend(GenesisConfig::<Test> {
			operators: vec! {operator.public().clone()},
			submit_interval: 1,
		}.build_storage().unwrap().0);
		t.into()
	}


	#[test]
	fn test_commit() {
		let operator_pair = utxo::helper::account_key_pair("operator");
		with_externalities(&mut new_test_ext(&operator_pair), || {
			let random_hash = H256::random();

			Child::commit(Origin::signed(operator_pair.public()), 1, random_hash.clone());

			let current = <CurrentBlock<Test>>::get();
			let root = <ChildChain<Test>>::get(&1).unwrap();
			assert_eq!(1, current);
			assert_eq!(random_hash, root);
		});
	}

	#[test]
	fn test_deposit() {
		let operator_pair = utxo::helper::account_key_pair("operator");
		with_externalities(&mut new_test_ext(&operator_pair), || {

			// check deposit.
			let receiver_key_pair = utxo::helper::account_key_pair("test_receiver");
			let signed_tx = utxo::helper::gen_transfer::<Test>(&operator_pair, &receiver_key_pair.public(), 100000);
			assert_eq!(Ok(()), Child::deposit(Origin::signed(receiver_key_pair.public()), signed_tx));

			// invalid deposit transfer from no operator.
			let invalid_signed_tx = utxo::helper::gen_transfer::<Test>(&receiver_key_pair, &operator_pair.public(), 1000);
			assert_eq!(Err("Signer is not operators."), Child::deposit(Origin::signed(operator_pair.public()), invalid_signed_tx));
		});
	}

	#[test]
	fn test_exit_start() {
		let operator_pair = utxo::helper::account_key_pair("operator");
		with_externalities(&mut new_test_ext(&operator_pair), || {

			// check deposit.
			let receiver_key_pair = utxo::helper::account_key_pair("test_receiver");
			let signed_tx = utxo::helper::gen_transfer::<Test>(&operator_pair, &receiver_key_pair.public(), 100000);
			assert_eq!(Ok(()), Child::deposit(Origin::signed(receiver_key_pair.public()), signed_tx));

			// invalid deposit transfer from no operator.
			let invalid_signed_tx = utxo::helper::gen_transfer::<Test>(&receiver_key_pair, &operator_pair.public(), 1000);
			assert_eq!(Err("Signer is not operators."), Child::deposit(Origin::signed(operator_pair.public()), invalid_signed_tx));

			let tx_ref = Utxo::unspent_outputs_finder(&receiver_key_pair.public()).unwrap()[0];
			assert_eq!(Ok(()), Child::exit_start(Origin::signed(operator_pair.public()), tx_ref.0, tx_ref.1));
			assert_eq!(None, Utxo::unspent_outputs_finder(&receiver_key_pair.public()));
			assert_eq!(None, Utxo::unspent_outputs(&(tx_ref.0, tx_ref.1)));
		});
	}
}
