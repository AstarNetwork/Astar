pub mod mvp;

use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result};
use system::ensure_signed;

/// The module's configuration trait.
pub trait Trait: system::Trait {
	type Utxo;
	type Tree;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage!{
	trait Store for Module<T: Trait> as Child {
		ChildChain get(child_chain): map T::BlockNumber => Option<T::Hash>;
		CurrentBlock get(current_block): T::BlockNumber = T::BlockNumber::sa(0);
	}
}

decl_module!{
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		// commit (change childchain)
		pub fn commit(origin, blk_num: T::BlockNumber, hash: T::Hash) {
			<ChildChain<T>>::insert(&blk_num, hash);
			<CurrentBlock<T>>::put(blk_num);
			Ok(())
		}

		pub fn deposit(origin, sign_tx) {

		}

		// Just a dummy entry point.
		// function that can be called by the external world as an extrinsics call
		// takes a parameter of the type `AccountId`, stores it and emits an event
		pub fn do_something(origin, something: u32) -> Result {
			// TODO: You only need this if you want to check it was signed.
			let who = ensure_signed(origin)?;

			// TODO: Code to execute when something calls this.
			// For example: the following line stores the passed in u32 in the storage
			<Something<T>>::put(something);

			// here we are raising the Something event
			Self::deposit_event(RawEvent::SomethingStored(something, who));
			Ok(())
		}
	}

	// deposit () verfy and execute(operator -> depositor) by Utxo.
	// exitStart() delete exitor utxo by Utxo.
}

decl_event!(
	/// An event in this module.
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		// Submit
		// Commit
		// Deposit
		// ExitStart
	}
);
