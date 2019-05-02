use super::*;

use sr_primitives::{
	traits::{BlakeTwo256},
};
use primitives::{sr25519, Pair, H256};

pub type Signature = sr25519::Signature;

pub type AccountId = <Signature as Verify>::Signer;

pub type MerkleTree = plasm_merkle::mock::MerkleTree<H256, BlakeTwo256>;

pub type Value = plasm_primitives::mvp::Value;
pub type TestInput = TransactionInput<H256>;
pub type TestOutput = TransactionOutput<Value, AccountId>;
pub type TestTransaction = Transaction<TestInput, TestOutput, u64>;

pub fn account_key_pair(s: &str) -> sr25519::Pair {
	sr25519::Pair::from_string(&format!("//{}", s), None)
		.expect("static values are valid; qed")
}

fn default_tx_in(in_hash: H256, in_index: u32) -> TestInput {
	TestInput::new(in_hash, in_index)
}

fn default_tx_out(out_value: Value, out_key: AccountId) -> TestOutput {
	TestOutput::new(out_value, vec! {out_key, }, 1)
}

fn gen_normal_tx(in_hash: H256, in_index: u32,
				 out_value: Value, out_key: AccountId) -> TestTransaction {
	TestTransaction::new(
		vec! {
			default_tx_in(in_hash, in_index),
		},
		vec! {
			default_tx_out(out_value, out_key),
		}, 0)
}

fn from_trait_ac<T: Trait>(key: &AccountId) -> <T as system::Trait>::AccountId {
	let mut key: &[u8] = key.as_ref();
	<T as system::Trait>::AccountId::decode(&mut key).unwrap()
}

fn sign<T: Trait>(tx: &TestTransaction, key_pair: &sr25519::Pair) -> T::SignedTransaction {
	let tx = T::Transaction::decode(&mut &tx.encode()[..]).unwrap();
	let signature = key_pair.sign(<T as system::Trait>::Hashing::hash_of(&tx).as_ref());
	let signature = T::Signature::decode(&mut &signature.encode()[..]).unwrap();
	let ret = SignedTransaction::<T> {
		payload: Some(tx),
		signatures: vec! {signature},
		public_keys: vec! {from_trait_ac::<T>(&key_pair.public())},
	};
	T::SignedTransaction::decode(&mut &ret.encode()[..]).unwrap()
}

pub fn genesis_tx<T: Trait>(root: &sr25519::Pair) -> Vec<(T::Value, <T as system::Trait>::AccountId)> {
	vec! {(T::Value::sa(1 << 60), from_trait_ac::<T>(&root.public())), }
}

pub fn gen_transfer<T: Trait>(sender: &sr25519::Pair, receiver: &AccountId, value: u64) -> T::SignedTransaction {
	let ref_utxo = <UnspentOutputsFinder<T>>::get(from_trait_ac::<T>(&sender.public())).as_ref().unwrap()[0];
	let tx = gen_normal_tx(H256::decode(&mut ref_utxo.0.as_ref()).unwrap(), ref_utxo.1, Value::sa(value), receiver.clone());
	sign::<T>(&tx, sender)
}
