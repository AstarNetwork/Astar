use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result};
use system::ensure_signed;
use sr_primitives::traits::{CheckedAdd, CheckedSub, Zero, One};

/// The module's configuration trait.
pub trait Trait: balances::Trait {
	type ExitId;
	type ExitStatus;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as TemplateModule {
		TotalDeposit get(total_deposit) config() : <T as balances::Trait>::Balance;
		ChildChain get(child_chain): map T::BlockNumber => T::Hash;
		CurrentBlock get(current_block): T::BlockNumber = T::BlockNumber::zero();
		Operator get(operator) config() : Vec<T::AccountId> = Default::default();
		ExitStatus get(exit_stats) : map T::ExitId = T::ExitStatus;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		/// submit childchain merkle root to parant chain.
		pub fn submit(origin, root: T::Hash) -> Result {
			let origin = ensure_signed(origin)?;

			// validate
			if !Self::operator().contains(&origin) { return Err("permission error submmit can be only operator."); }
			let current = Self::current_block();
			let next = current.checked_add(&T::BlockNumber::one()).ok_or("block number is overflow.")?;

			/// update
			<ChildChain<T>>::insert(&next, root);
			<CurrentBlock<T>>::put(next);
			Ok(())
		}

		/// deposit balance parent chain to childchain.
		pub fn deposit(origin, #[compact] value: <T as balances::Trait>::Balance) -> Result {
			let depositor = ensure_signed(origin)?;

			// validate
			let now_balance = <balances::Module<T>>::free_balance(&depositor);
			let new_balance = now_balance.checked_sub(&value).ok_or("not enough balance.")?;

			let now_total_deposit = Self::total_deposit();
			let new_total_deposit = now_total_deposit.checked_add(&value).ok_or("overflow total deposit.")?;

			// update
			<balances::FreeBalance<T>>::insert(&depositor, new_balance);
			<TotalDeposit<T>>::put(value.clone());
			Self::deposit_event(RawEvent::Deposit(depositor, value));
			Ok(())
		}

		/// exit balances start parent chain from childchain.
		pub fn exit_start(origin) -> Result {
			Ok(())
		}

		/// exit finalize parent chain from childchain.
		pub fn exit_finalize(origin) -> Result {
			Ok(())
		}

		/// exit challenge(fraud proofs) parent chain from child chain.
		pub fn exit_challenge(origin) -> Result {
			Ok(())
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event<T>
		where 	AccountId = <T as system::Trait>::AccountId,
				Balance = <T as balances::Trait>::Balance {
		/// Submit Events
		Submit,
		/// Deposit Events to child operator.
		Deposit(AccountId, Balance),
		// Start Exit Events to child operator
		ExitStart(u32),
		/// Challenge Events
		Challenge(u32),
		/// Exit Finalize Events
		ExitFinalize(AccountId),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use sr_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok};
	use sr_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header},
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
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

	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = ();
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
	}

	impl Trait for Test {
		type Event = ();
	}

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {});
	}
}
