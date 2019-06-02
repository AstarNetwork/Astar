use super::*;

#[cfg(feature = "std")]
use serde_derive::{Serialize, Deserialize};

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, Parameter, dispatch::Result, traits::MakePayment};
use system::{ensure_signed, OnNewAccount, IsDeadAccount};
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash, SimpleArithmetic, Verify, As, Zero, CheckedAdd, CheckedSub};

use parity_codec::{Encode, Decode, Codec};

/// H: Hash
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct TransactionInput<H> {
	pub tx_hash: H,
	pub out_index: u32,
}

pub type TxIn<T> = TransactionInput<<T as system::Trait>::Hash>;

impl<H: Copy> TransactionInput<H> {
	fn output_or_default<T: Trait>(&self) -> TransactionOutput<T::Value, T::AccountId>
		where (H, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		match <UnspentOutputs<T>>::get((self.tx_hash.clone(), self.out_index)) {
			Some(tx_out) => tx_out,
			None => Default::default(),
		}
	}
	fn output<T: Trait>(&self) -> Option<TransactionOutput<T::Value, T::AccountId>>
		where (H, u32): rstd::borrow::Borrow<(<T as system::Trait>::Hash, u32)> {
		<UnspentOutputs<T>>::get((self.tx_hash.clone(), self.out_index))
	}
}

/// V: Value, K: Key
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct TransactionOutput<V, K> {
	pub value: V,
	pub keys: Vec<K>,
	pub quorum: u32,
}

pub type TxOut<T> = TransactionOutput<<T as Trait>::Value, <T as system::Trait>::AccountId>;

/// V: Value, K: Key, H: Hash, L: TimeLock
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct Transaction<V, K, H, L> {
	pub inputs: Vec<TransactionInput<H>>,
	pub outputs: Vec<TransactionOutput<V, K>>,
	pub lock_time: L,
}

pub type Tx<T> = Transaction<<T as Trait>::Value, <T as system::Trait>::AccountId, <T as system::Trait>::Hash, <T as Trait>::TimeLock>;

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct SignedTransaction<V, K, H, S, L> {
	pub payload: Transaction<V, K, H, L>,
	pub signatures: Vec<S>,
	pub public_keys: Vec<K>,
}

pub type SignedTx<T> = SignedTransaction<<T as Trait>::Value, <T as system::Trait>::AccountId, <T as system::Trait>::Hash, <T as Trait>::Signature, <T as Trait>::TimeLock>;

pub fn hash_of<Hashing, H>(tx_hash: &H, i: &u32) -> H
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	Hashing::hash(&plasm_primitives::concat_bytes(tx_hash, i))
}

pub trait Trait: system::Trait {
	type Signature: Parameter + Default + Verify<Signer=Self::AccountId>;
	type TimeLock: Parameter + Zero + Default;
	type Value: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + As<usize> + As<u64> + MaybeSerializeDebug;

	type OnNewAccount: OnNewAccount<Self::AccountId>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

fn generate_tx_from_conf<T: Trait>(genesis: &Vec<(T::Value, <T as system::Trait>::AccountId)>) -> Tx<T> {
	Transaction {
		inputs: vec! {},
		outputs: genesis
			.clone()
			.iter()
			.map(|e| TransactionOutput { value: e.0.clone(), keys: vec! {e.1.clone()}, quorum: 1 })
			.collect::<Vec<_>>(),
		lock_time: T::TimeLock::zero(),
	}
}

decl_storage! {
	trait Store for Module <T: Trait> as Utxo {
		/// All valid unspent transaction outputs are stored in this map.
		/// Initial set of UTXO is populated from the list stored in genesis.
		pub UnspentOutputs get(unspent_outputs) build( |config: &GenesisConfig<T>| {
			let tx = generate_tx_from_conf::<T>(&config.genesis_tx);
			tx.clone()
				.outputs
				.iter()
				.enumerate()
				.map(|(i, u)| ((<T as system::Trait>::Hashing::hash_of(&tx), i as u32), u.clone()))
				.collect::<Vec<_>>()
		}): map (<T as system::Trait>::Hash, u32) => Option<TxOut<T>>;

		/// [AccountId] = reference of UTXO.
		pub UnspentOutputsFinder get(unspent_outputs_finder) build( |config: &GenesisConfig<T> | {
			let tx = generate_tx_from_conf::<T>(&config.genesis_tx);
			config.genesis_tx
				.clone()
				.iter()
				.enumerate()
				.map(|(i, e)| (e.1.clone(), vec!{(<T as system::Trait>::Hashing::hash_of(&tx), i as u32)}))
				.collect::<Vec<_>>()
		}): map <T as system::Trait>::AccountId => Option<Vec<(<T as system::Trait>::Hash, u32)>>; //TODO HashSet<>

		pub TxList get(tx_list) build( |config: &GenesisConfig<T> | {
			let tx = generate_tx_from_conf::<T>(&config.genesis_tx);
			vec!{(<T as system::Trait>::Hashing::hash_of(&tx),
				SignedTx::<T> {
					payload: tx,
					signatures: vec!{},
					public_keys: vec!{},
				})}
		}): map <T as system::Trait>::Hash => SignedTx<T>;

		/// Total leftover value to be redistributed among authorities.
		/// It is accumulated during block execution and then drained
		/// on block finalization.
		pub LeftoverTotal get(leftover_total): T::Value;

		// TODO Outputs that are locked using mining incentive utxo.
		//	pub LockedOutputs get(locked_outputs): map T::BlockNumber => Option <Vec<TxOut<T>>>;
	}

	add_extra_genesis {
		config(genesis_tx): Vec<(T::Value, <T as system::Trait>::AccountId)>;
	}
}

decl_module! {
	// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T> () = default;

		// Dispatch a single transaction and update UTXO set accordingly
		pub fn execute(origin, signed_tx: SignedTx<T> ) -> Result {
			ensure_signed(origin)?;
			Self::exec(signed_tx)
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event<T> where SignedTransaction = SignedTx<T> {
		/// Transaction was executed successfully
		TransactionExecuted(SignedTransaction),
	}
);

impl<T: Trait> WritableUtxoTrait<SignedTx<T>, T::AccountId, (T::Hash, u32)> for Module<T> {
	/// push: updated UnspentOutputs, UnspentOutputsFinder and TxList.
	fn push(signed_tx: SignedTx<T>) {
		let tx = &signed_tx.payload;
		let hash = <T as system::Trait>::Hashing::hash_of(tx);
		for (i, out) in tx.outputs
			.iter()
			.enumerate() {
			let identify = (hash.clone(), i as u32);
			<UnspentOutputs<T>>::insert(identify.clone(), out.clone());
			for key in out.keys.iter() {
				if !<UnspentOutputsFinder<T>>::exists(key) { // if unexits outputs finder, create accounts.
					T::OnNewAccount::on_new_account(key);
				}
				<UnspentOutputsFinder<T>>::mutate(key, |v| {
					match v.as_mut() {
						Some(vc) => vc.push(identify.clone()),
						None => *v = Some(vec! {identify.clone()}),
					}
				});
			}
		}
		<TxList<T>>::insert(&hash, signed_tx);
	}

	/// spent transaction, remove finder and utxo used by inputs.
	fn spent(signed_tx: &SignedTx<T>) {
		for inp in signed_tx.payload.inputs.iter() {
			let identify = (inp.tx_hash.clone(), inp.out_index);
			for key in inp.output_or_default::<T>().keys.iter() {
				Self::remove_finder(key, &identify);
			}
			Self::remove(&identify);
		}
	}

	/// remove utxo finder.
	fn remove_finder(who: &T::AccountId, out_point: &(T::Hash, u32)) {
		<UnspentOutputsFinder<T>>::mutate(who, |v| {
			*v = match
				v.as_ref()
					.unwrap_or(&vec! {})
					.iter()
					.filter(|e| **e != *out_point)
					.map(|e| *e)
					.collect::<Vec<_>>()
					.as_slice() {
				[] => None,
				s => Some(s.to_vec()),
			}
		});
	}

	/// remove utxo.
	fn remove(out_point: &(T::Hash, u32)) {
		<UnspentOutputs<T>>::remove(out_point);
	}

	fn deal(whoes: &Vec<T::AccountId>) {
		let leftover = <LeftoverTotal<T>>::take();

		// send leftover to all authorities.
		if whoes.len() == 0 { return; }
		let shared_value = leftover / T::Value::sa(whoes.len() as u64);
		if shared_value == T::Value::zero() { return; }

		// create UnspentTransactionOutput
		let outs: Vec<_> = whoes.iter()
			.map(|key|
				TransactionOutput {
					value: shared_value.clone(),
					keys: vec! {key.clone(), },
					quorum: 1,
				})
			.collect();

		// crate Transaction.
		let tx = Tx::<T> {
			inputs: vec! {},
			outputs: outs,
			lock_time: T::TimeLock::zero(),
		};
		Self::push(SignedTx::<T> {
			payload: tx,
			signatures: vec! {},
			public_keys: vec! {},
		});
		<LeftoverTotal<T>>::put(T::Value::zero());
	}
}

impl<T: Trait> ReadbleUtxoTrait<SignedTx<T>, T::Value> for Module<T> {
	fn verify(signed_tx: &SignedTx<T>) -> Result {
		let mut hash = <T as system::Trait>::Hashing::hash_of(&signed_tx.payload);
		for (sign, key) in signed_tx.signatures.iter().zip(signed_tx.public_keys.iter()) {
			if !sign.verify(hash.as_mut() as &[u8], key) {
				return Err("signature is unverified.");
			}
		}
		if <TxList<T>>::exists(&T::Hashing::hash_of(&signed_tx.payload)) {
			return Err("already exist same transaction.");
		}
		Ok(())
	}

	fn unlock(signed_tx: &SignedTx<T>) -> Result {
		let keys: Vec<_> = signed_tx.public_keys.iter().collect();
		for input in signed_tx.payload
			.inputs
			.iter() {
			let output = input.output::<T>()
				.ok_or("specified utxo by input is not found.")?;
			if output.quorum > output
				.keys
				.iter()
				.filter(|key|
					keys.contains(key))
				.count() as u32 {
				return Err("not enough public_keys to unlock all specified utxo.");
			}
		}
		Ok(())
	}

	fn leftover(signed_tx: &SignedTx<T>) -> CheckResult<T::Value> {
		let sum_in: T::Value = signed_tx
			.payload
			.inputs
			.iter()
			.try_fold(T::Value::zero(), |sum, inp| sum.checked_add(&inp.output_or_default::<T>().value))
			.ok_or("sum of inputs value is overflow")?;
		let sum_out: T::Value = signed_tx
			.payload
			.outputs
			.iter()
			.try_fold(T::Value::zero(), |sum, out| sum.checked_add(&out.value))
			.ok_or("sum of outputs value is overflow")?;
		let leftover = sum_in.checked_sub(&sum_out).ok_or("leftover invalid (sum of input) - (sum of output)")?;
		Ok(leftover)
	}
}

impl<T: Trait> UtxoTrait<SignedTx<T>, T::AccountId, (T::Hash, u32), T::Value> for Module<T> {
	fn exec(signed_tx: SignedTx<T>) -> Result {
		// all signature checking Signature.Verify(HashableAccountId, hash(transaction.payload)).
		Self::verify(&signed_tx)?;
		// UTXO unlocked checking.
		Self::unlock(&signed_tx)?;
		// LeftOver(Fee) calclate.
		let leftover = Self::leftover(&signed_tx)?;

		// Calculate new leftover total
		let new_total = <LeftoverTotal<T>>::get()
			.checked_add(&leftover)
			.ok_or("leftover overflow")?;

		<LeftoverTotal<T>>::put(new_total);
		Self::spent(&signed_tx);
		Self::push(signed_tx.clone());
		Self::deposit_event(RawEvent::TransactionExecuted(signed_tx));
		Ok(())
	}
}

impl<T: Trait> MakePayment<T::AccountId> for Module<T> {
	/// `encoded_len` bytes. Return `Ok` iff the payment was successful.
	fn make_payment(_: &T::AccountId, _: usize) -> Result {
		Ok(()) // now not do.
	}
}

impl<T: Trait> IsDeadAccount<T::AccountId> for Module<T>
{
	fn is_dead_account(who: &T::AccountId) -> bool {
		Self::unspent_outputs_finder(who).is_none()
	}
}


#[macro_export]
macro_rules! impl_mvp_test_helper {
	( $a:ty, $b:ty ) => (
		fn hash(tx: &Tx<$a>) -> H256 {
			BlakeTwo256::hash_of(tx)
		}

		pub fn genesis_tx(root: &sr25519::Pair) -> Vec<(u64, AccountId)> {
			vec! {(1000000000000000, root.public().clone()), }
		}

		pub fn account_key_pair(s: &str) -> sr25519::Pair {
			sr25519::Pair::from_string(&format!("//{}", s), None)
				.expect("static values are valid; qed")
		}

		pub fn get_values_from_refs(refs: Vec<(H256, u32)>) -> u64 {
			let utxos = refs
				.iter()
				.map(|r| <$b>::get((r.0.clone(), r.1)))
				.map(|r| r.unwrap())
				.collect::<Vec<_>>();
			utxos.iter().fold(0, |sum, o| sum + o.value)
		}

		fn gen_tx_in(hash: H256, index: u32) -> TxIn<$a> {
			TransactionInput {
				tx_hash: hash,
				out_index: index,
			}
		}

		fn gen_tx_out(value: u64, out_key: AccountId) -> TxOut<$a> {
			TransactionOutput {
				value: value,
				keys: vec! {out_key, },
				quorum: 1,
			}
		}

		pub fn gen_tx_form_ref(refs: Vec<(H256, u32)>, sender: AccountId, receiver: AccountId, value: u64) -> Tx<$a> {
			let sum = get_values_from_refs(refs.clone());
			Transaction {
				inputs: refs.iter()
					.cloned()
					.map(|r| gen_tx_in(r.0.clone(), r.1))
					.collect::<Vec<_>>(),
				outputs: vec! {
					gen_tx_out(value, receiver),
					gen_tx_out(sum - value - 1000, sender)
				},
				lock_time: 0,
			}
		}

		fn sign(tx: Tx<$a>, key_pair: &sr25519::Pair) -> SignedTx<$a> {
			let signature = key_pair.sign(hash(&tx).as_ref());
			SignedTransaction {
				payload: tx,
				signatures: vec! {signature},
				public_keys: vec! {key_pair.public().clone()},
			}
		}

		pub fn gen_transfer(sender: &sr25519::Pair, receiver: &AccountId, value: u64) -> SignedTx<$a> {
			let ref_utxo = <UnspentOutputsFinder<$a>>::get(&sender.public()).unwrap();
			let tx = gen_tx_form_ref(ref_utxo, sender.public().clone(), receiver.clone(), value);
			sign(tx, sender)
		}

		pub fn verify(account_id: &AccountId, value: u64, num_of_utxo: usize) {
			let ref_utxo = <UnspentOutputsFinder<$a>>::get(account_id).unwrap();
			assert_eq!(num_of_utxo, ref_utxo.len());
			let utxos = ref_utxo
				.iter()
				.map(|r| <$b>::get((r.0.clone(), r.1)))
				.map(|r| r.unwrap())
				.collect::<Vec<_>>();
			let sum = utxos.iter().fold(0, |sum, o| sum + o.value);
			assert_eq!(value, sum);
		}

	)
}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;
	use runtime_io::with_externalities;

	use support::{impl_outer_origin, assert_ok, assert_err};
	use sr_primitives::{
		BuildStorage,
		traits::{Verify, BlakeTwo256, IdentityLookup, Hash},
		testing::{Digest, DigestItem, Header},
	};
	use primitives::{Blake2Hasher, H256, sr25519, crypto::Pair};
	use std::clone::Clone;

	pub type Signature = sr25519::Signature;

	pub type AccountId = <Signature as Verify>::Signer;

	impl_outer_origin! {
		pub enum Origin for Test {}
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
		type Event = ();
		type Log = DigestItem;
	}

	impl Trait for Test {
		type Signature = Signature;
		type TimeLock = Self::BlockNumber;
		type Value = u64;

		type OnNewAccount = ();

		type Event = ();
	}

	type UTXO = Module<Test>;

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct Test;

	// This function basically just builds ax genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext(root: &sr25519::Pair) -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(GenesisConfig::<Test> {
			genesis_tx: genesis_tx(root),
		}.build_storage().unwrap().0);
		t.into()
	}

	impl_mvp_test_helper!(Test, UnspentOutputs<Test>);

	#[test]
	fn mvp_minimum_works() {
		let root_key_pair = account_key_pair("test_root");
		with_externalities(&mut new_test_ext(&root_key_pair), || {
			// check merkle root ============================== different default
			verify(&root_key_pair.public(), 1000000000000000, 1);

			// check total leftover is 0
			let leftover_total = <LeftoverTotal<Test>>::get();
			assert_eq!(0, leftover_total);

			let receiver_key_pair = account_key_pair("test_receiver");
			let transfer_1 = gen_transfer(&root_key_pair, &receiver_key_pair.public(), 100000);
			assert_ok!(UTXO::execute(Origin::signed(root_key_pair.public()), transfer_1.clone()));

			verify(&root_key_pair.public(), 1000000000000000 - 100000 - 1000, 1);
			verify(&receiver_key_pair.public(), 100000, 1);
			assert_eq!(transfer_1, <TxList<Test>>::get(hash(&transfer_1.payload)));
			assert_eq!(1000, <LeftoverTotal<Test>>::get());

			// double spending error!
			assert_err!(UTXO::execute(Origin::signed(root_key_pair.public()), transfer_1), "already exist same transaction.");

			let receiver_key_pair = account_key_pair("test_receiver");
			let transfer_2 = gen_transfer(&root_key_pair, &receiver_key_pair.public(), 200000);

			assert_ok!(UTXO::execute(Origin::signed(root_key_pair.public()), transfer_2.clone()));
			verify(&root_key_pair.public().clone(), 1000000000000000 - 300000 - 2000, 1);
			verify(&receiver_key_pair.public(), 300000, 2);
			assert_eq!(transfer_2, <TxList<Test>>::get(hash(&transfer_2.payload)));
			assert_eq!(2000, <LeftoverTotal<Test>>::get());

			// deal test.
			UTXO::deal(&vec! {root_key_pair.public(), });

			verify(&root_key_pair.public().clone(), 1000000000000000 - 300000, 2);
			assert_eq!(0, <LeftoverTotal<Test>>::get());

			// receiver -> receiver2
			let receiver_key_pair_2 = account_key_pair("test_receiver_2");
			let transfer_3 = gen_transfer(&receiver_key_pair, &receiver_key_pair_2.public(), 200000);

			assert_ok!(UTXO::execute(Origin::signed(receiver_key_pair.public()), transfer_3.clone()));
			verify(&receiver_key_pair.public(), 300000 - 200000 - 1000, 1);
			verify(&receiver_key_pair_2.public(), 200000, 1);
			assert_eq!(transfer_3, <TxList<Test>>::get(hash(&transfer_3.payload)));
			assert_eq!(1000, <LeftoverTotal<Test>>::get());
		});
	}
}
