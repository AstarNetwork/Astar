///// A runtime module template with necessary imports
//
///// Feel free to remove or edit this file as needed.
///// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
///// If you remove this file, you can remove those references
//
//
///// For more guidance on Substrate modules, see the example module
///// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs
//
//use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result};
//use system::ensure_signed;
//use primitives::{Blake2Hasher, H256};
//
//// use Vec<>
//use rstd::prelude::*;
//// use Encode, Decode
//use parity_codec::{Encode, Decode};
//use serde::{Serialize, Deserialize};
//
///// Single transaction to be dispatched
//#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
//#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash)]
//pub struct Transaction {
//	/// UTXOs to be used as inputs for current transaction
//	pub inputs: Vec<TransactionInput>,
//	/// UTXOs to be created as a result of current transaction dispatch
//	pub outputs: Vec<TransactionOutput>,
//	/// LockTime
//	pub lock_time: i32,
//}
//
///// Single transaction input that refers to one UTXO
//#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
//#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash)]
//pub struct TransactionInput {
//	/// Reference to an UTXO to be spent
//	pub parent_output: H256,
//	/// Proof that transaction owner is authorized to spend referred UTXO
//	pub signature: Signature,
//}
//
///// Single transaction output to create upon transaction dispatch
//#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
//#[derive(PartialEq, Eq, PartialOrd, Ord, Default, Clone, Encode, Decode, Hash)]
//pub struct TransactionOutput {
//	/// Value associated with this output
//	pub value: Value,
//	/// Public key associated with this output. In order to spend this output
//    /// owner must provide a proof by hashing whole `TransactionOutput` and
//    /// signing it with a corresponding private key.
//	pub pubkey: H256,
//	/// Unique (potentially random) value used to distinguish this
//    /// particular output from others addressed to the same public
//    /// key with the same value. Prevents potential replay attacks.
//	pub salt: u32,
//}
//
///// Information collected during transaction verification
//pub enum CheckInfo<'a> {
//	/// Combined value of all inputs and outputs
//	Totals { input: Value, output: Value },
//	/// Some referred UTXOs were missing
//	MissingInputs(Vec<&'a H256>),
//}
//
///// Result of transaction verification
//pub type CheckResult<'a> = rstd::result::Result<CheckInfo<'a>, &'static str>;
//
//
///// Check transaction for validity
//pub fn check_transaction(transaction: &Transaction) -> CheckResult<'_> {
//	// check
//}
//
//
///// The module's configuration trait.
//pub trait Trait: system::Trait {
//	// TODO: Add other types and constants required configure this module.
//
//	/// The overarching event type.
//	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
//	type Consensus: consensus::Authorities<<A as AuthoringApi>::Block> + Send + Sync + 'static;
//}
//
///// This module's storage items.
//decl_storage! {
//	trait Store for Module<T: Trait> as Utxo {
//		/// All valid unspent transaction outputs are stored in this map.
//		/// Initial set of UTXO is populated from the list stored in genesis.
//		UnspentOutputs build(|config: &GenesisConfig<T>| {
//			config.initial_utxo
//				.iter()
//				.cloned()
//				.map(|u| (BlakeTwo256::hash_of(&u), u))
//				.collect::<Vec<_>>()
//		}): map H256 => Option<TransactionOutput>;
//
//		/// Total leftover value to be redistributed among authorities.
//		/// It is accumulated during block execution and then drained
//		/// on block finalization.
//		LeftoverTotal: Value;
//
//		/// Outputs that are locked
//		LockedOutputs: map H256 => Option<LockStatus<T::BlockNumber>>;
//	}
//
//	add_extra_genesis {
//		config(initial_utxo): Vec<TransactionOutput>;
//	}
//}
//
///// Redistribute combined leftover value evenly among authorities
//fn spend_leftover(authorities: &[H256]) {
//	let leftover = <LeftoverTotal<T>>::take();
//	let share_value = leftover / authorities.len() as Value;
//	if share_value == 0 { return }
//	for authority in authorities {
//		let utxo = TransactionOutput {
//			pubkey: *authority,
//			value: share_value,
//			salt: System::block_number() as u32,
//		};
//		let hash = BlakeTwo256::hash_of(&utxo);
//		if !<UnspentOutputs<T>>::exists(hash) {
//			<UnspentOutputs<T>>::insert(hash, utxo);
//			runtime_io::print("leftover share sent to");
//			runtime_io::print(hash.as_fixed_bytes() as &[u8]);
//		} else {
//			runtime_io::print("leftover hash collision");
//		}
//	}
//}
//
//decl_module! {
//	/// The module declaration.
//	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
//		// Initializing events
//		// this is needed only if you are using events in your module
//		fn deposit_event<T>() = default;
//
//		/// Dispatch a single transaction and update UTXO set accordingly
//		pub fn execute(origin, transaction: Transaction) -> Result {
//			ensure_inherent(origin)?;
//			let leftover = match Self::check_transaction(&transaction)? {
//				CheckInfo::MissingInputs(_) => return Err("output missing"),
//				CheckInfo::Totals { input, output } => input - output
//			};
//			Self::update_storage(&transaction, leftover)?;
//			Self::deposit_event(Event::TransactionExecuted(transaction));
//			Ok(())
//		}
//
//		/// Handler called by the system on block finalization
//		pub fn on_finalise(origin, ) {
//			let authorities: Vec<_> = consensus::authorities()
//				.iter().map(|&a| a.into()).collect();
//			Self::spend_leftover(&authorities);
//		}
//
//		/// Update storage to reflect changes made by transaction
//		pub fn update_storage(transaction: &Transaction, leftover: Value) -> Result {
//			/// Calculate new leftover total
//			let new_total = <LeftoverTotal<T>>::get()
//				.checked_add(leftover)
//				.ok_or("leftover overflow")?;
//
//			/// Storing updated leftover value
//			<LeftoverTotal<T>>::put(new_total);
//
//			/// Remove all used UTXO since they are now spent
//			for input in &transaction.inputs {
//				<UnspentOutputs<T>>::remove(input.parent_output);
//			}
//
//			/// Add new UTXO to be used by future transactions
//			for output in &transaction.outputs {
//				let hash = T::Hashing::hash_of(output);
//				<UnspentOutputs<T>>::insert(hash, output);
//			}
//			Ok(())
//		}
//	}
//}
//
//impl<T: Trait> Module<T> {
//	/// generate_block on child chain.(this utxo)
//	pub fn generate_block() -> Result {
//		OK(())
//	}
//
//	/// submits to parent chain, executed it after submit parent chain.
//	pub fn submit_block() -> Resut {
//		OK(())
//	}
//
//	/// deposit tokens from parent chain, executed it after deposit parent chain.
//	pub fn deposit() -> Result {
//		OK(())
//	}
//
//	/// starting exit to parent chain, executed it after start_exit parent chain.
//	pub fn start_exit() -> Result {
//		OK(())
//	}
//
//	/// append transaction.
//	pub fn append_tx() -> Result {
//		/*
//		  appendTx(tx: SignedTransaction): ChamberResult<boolean> {
//			try {
//			  if(this.txFilter.checkAndInsertTx(tx)
//			  && this.segmentChecker.isContain(tx)) {
//				this.txQueue.push(tx)
//				return new ChamberOk(true)
//			  }else{
//				return new ChamberOk(false)
//			  }
//			} catch (e) {
//			  return new ChamberResultError(ChainErrorFactory.InvalidTransaction())
//			}
//		  }
//		*/
//		OK(())
//	}
//}
//
//decl_event!(
//	/// An event in this module.
//	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
//		/// Transaction was executed successfully
//		TransactionExecuted(Transaction),
//	}
//);
//
///// tests for this module
//#[cfg(test)]
//mod tests {
//	use super::*;
//
//	use runtime_io::with_externalities;
//	use primitives::{H256, Blake2Hasher};
//	use support::{impl_outer_origin, assert_ok};
//	use runtime_primitives::{
//		BuildStorage,
//		traits::{BlakeTwo256, IdentityLookup},
//		testing::{Digest, DigestItem, Header},
//	};
//
//	impl_outer_origin! {
//		pub enum Origin for Test {}
//	}
//
//	// For testing the module, we construct most of a mock runtime. This means
//	// first constructing a configuration type (`Test`) which `impl`s each of the
//	// configuration traits of modules we want to use.
//	#[derive(Clone, Eq, PartialEq)]
//	pub struct Test;
//
//	impl system::Trait for Test {
//		type Origin = Origin;
//		type Index = u64;
//		type BlockNumber = u64;
//		type Hash = H256;
//		type Hashing = BlakeTwo256;
//		type Digest = Digest;
//		type AccountId = u64;
//		type Lookup = IdentityLookup<Self::AccountId>;
//		type Header = Header;
//		type Event = ();
//		type Log = DigestItem;
//	}
//
//	impl Trait for Test {
//		type Event = ();
//	}
//
//	type TemplateModule = Module<Test>;
//
//	// This function basically just builds a genesis storage key/value store according to
//	// our desired mockup.
//	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
//		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
//	}
//
//	#[test]
//	fn it_works_for_default_value() {
//		with_externalities(&mut new_test_ext(), || {
//			// Just a dummy test for the dummy funtion `do_something`
//			// calling the `do_something` function with a value 42
//			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
//			// asserting that the stored value is equal to what we stored
//			assert_eq!(TemplateModule::something(), Some(42));
//		});
//	}
//}
