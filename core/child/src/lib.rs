use sr_primitives::traits::{As, Hash, Member, MaybeSerializeDebug};

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result};
use system::ensure_signed;

use merkle::MerkleTreeTrait;

//use rstd::collections::btree_set::BTreeSet;


/// The module's configuration trait.
pub trait Trait: utxo::Trait {
	type Utxo;
	type Tree: MerkleTreeTrait<Self::Hash, Self::Hashing>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage!{
	trait Store for Module<T: Trait> as Child {
		ChildChain get(child_chain): map T::BlockNumber => Option<T::Hash>;
		CurrentBlock get(current_block): T::BlockNumber = T::BlockNumber::sa(0);
		Operators get(oerators): Vec<T::AccountId>;
	}
}

decl_module!{
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		// commit (change childchain)
		pub fn commit(origin, blk_num: T::BlockNumber, hash: T::Hash) -> Result {
			<ChildChain<T>>::insert(&blk_num, hash);
			<CurrentBlock<T>>::put(blk_num);
			Ok(())
		}

		// deposit () verfy and execute(operator -> depositor) by Utxo.
		pub fn deposit(origin, signed_tx: T::SignedTransaction) -> Result {
			let operator = ensure_signed(origin)?;

			// TODO operator checks

			//<utxo::Module<T>>::execute(singed_tx)?;
			Ok(())
		}


		// exitStart() delete exitor utxo by Utxo.
		pub fn exit_start(origin, tx_hash: T::Hash, out_index: u32) -> Result {
			//<utxo::Trait<T>>::remove(&(tx_hash, out_index));
			Ok(())
		}

		pub fn get_proof(origin, blk_num: T::BlockNumber, tx_hash: T::Hash, out_index: u32) -> Result {
			// deposit_event(RawEvent(Proof(blk_num, tx_hash, out_index, proofs, depth, index));
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
