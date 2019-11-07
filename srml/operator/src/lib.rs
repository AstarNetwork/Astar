/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result, Parameter};
use system::ensure_signed;
use sr_primitives::traits::{self, Member, MaybeDisplay, MaybeSerializeDebug};
use rstd::collections::btree_set::BTreeSet;
use scale::{Encode, Decode};

/// The module's configuration trait.
pub trait Trait: system::Trait {
	type Parameters: Parameter + Member + MaybeSerializeDebug + MaybeDisplay + Default + rstd::hash::Hash;


	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as operator {
		/// A mapping from operators to operated contracts by them.
		pub OperatorHasContracts: map T::AccountId => BTreeSet<T::AccountId>;
		/// A mapping from operated contract by operator to it.
		pub ContractHasOperator: map T::AccountId => Option<T::AccountId>;
		/// A mapping from contract to it's parameters.
		pub ContractParameters: map T::AccountId => Option<T::Parameters>;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event() = default;

		/// Deploys a contact and insert relation of a contract and an operator to mapping.
		pub fn instantiate(origin,
			#[compact] endowment: BalanceOf<T>,
			#[compact] gas_limit: Gas,
			code_hash: CodeHash<T>,
			data: Vec<u8>,
			parameters: OperateParameters) -> Result {
			Ok(())
		}

		/// Updates parameters for an identified contact.
		pub fn update_parameters(origin, paramters: OperateParameters) -> Result {
			Ok(())
		}

		/// Changes an operator for identified contracts.
		pub fn change_operator(origin, contracts: Vec<AccountId>, new_operator: AccountId) -> Result {
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		/// When operator changed,
		/// it is issued that 1-st Operator AccountId and 2-nd Contract AccountId.
		SetOperator(AccountId, AccountId),

		/// When contract's parameters changed,
		/// it is issued that 1-st Contract AccountId and 2-nd the contract's new parameters.
		SetParameter(AccountId, Parameters),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use sr_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok, parameter_types};
	use sr_primitives::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, weights::Weight, Perbill,
	};
	use serde::{Serialize, Deserialize, de::DeserializeOwned};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type WeightMultiplierUpdate = ();
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
	}

	#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, Hash)]
	#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize, derive_more::Display))]
	pub struct TestParameters {
		pub a: u128,
	}
	impl Trait for Test {
		type Parameters = TestParameters;
		type Event = ();
	}
	type TemplateModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(TemplateModule::something(), Some(42));
		});
	}


}
