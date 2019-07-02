// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate. If not, see <http://www.gnu.org/licenses/>.

// TODO: #1417 Add more integration tests
// also remove the #![allow(unused)] below.

use sr_io::with_externalities;
use sr_primitives::testing::{Digest, DigestItem, H256, Header, UintAuthorityId};
use sr_primitives::traits::{BlakeTwo256, IdentityLookup};
use sr_primitives::BuildStorage;
use sr_io;
use support::{storage::child, StorageMap, assert_ok, impl_outer_event, impl_outer_dispatch,
				   impl_outer_origin, traits::Currency};
use primitives::Blake2Hasher;
use system::{self, Phase, EventRecord};
use primitives::storage::well_known_keys;
use parity_codec::{Encode, Decode, KeyedVec};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

mod pgspec {
	// Re-export contents of the root. This basically
	// needs to give a name for the current crate.
	// This hack is required for `impl_outer_event!`.
	pub use super::super::*;
	pub use support::impl_outer_event;
}

impl_outer_event! {
	pub enum MetaEvent for Test {
		balances<T>, contract<T>, pgspec<T>,
	}
}
impl_outer_origin! {
	pub enum Origin for Test { }
}
impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		balances::Balances,
		contract::Contract,
		pgspec::PGSpec,
	}
}

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
	type Event = MetaEvent;
	type Log = DigestItem;
}
impl balances::Trait for Test {
	type Balance = u64;
	type OnFreeBalanceZero = Contract;
	type OnNewAccount = ();
	type Event = MetaEvent;
	type TransactionPayment = ();
	type DustRemoval = ();
	type TransferPayment = ();
}
impl timestamp::Trait for Test {
	type Moment = u64;
	type OnTimestampSet = ();
}
impl contract::Trait for Test {
	type Currency = Balances;
	type Call = Call;
	type Gas = u64;
	type DetermineContractAddress = DummyContractAddressFor;
	type Event = MetaEvent;
	type ComputeDispatchFee = DummyComputeDispatchFee;
	type TrieIdGenerator = DummyTrieIdGenerator;
	type GasPayment = ();
}
impl Trait for Test {
	type Event = MetaEvent;
}

type Balances = balances::Module<Test>;
type Contract = contract::Module<Test>;
type System = system::Module<Test>;
type PGSpec = Module<Test>;

pub struct DummyContractAddressFor;
impl contract::ContractAddressFor<H256, u64> for DummyContractAddressFor {
	fn contract_address_for(_code_hash: &H256, _data: &[u8], origin: &u64) -> u64 {
		*origin + 1
	}
}

static KEY_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct DummyTrieIdGenerator;
impl contract::TrieIdGenerator<u64> for DummyTrieIdGenerator {
	fn trie_id(account_id: &u64) -> contract::TrieId {
		use primitives::storage::well_known_keys;

		// TODO: see https://github.com/paritytech/substrate/issues/2325
		let mut res = vec![];
		res.extend_from_slice(well_known_keys::CHILD_STORAGE_KEY_PREFIX);
		res.extend_from_slice(b"default:");
		res.extend_from_slice(&KEY_COUNTER.fetch_add(1, Ordering::Relaxed).to_le_bytes());
		res.extend_from_slice(&account_id.to_le_bytes());
		res
	}
}

pub struct DummyComputeDispatchFee;
impl contract::ComputeDispatchFee<Call, u64> for DummyComputeDispatchFee {
	fn compute_dispatch_fee(call: &Call) -> u64 {
		69
	}
}

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;

pub struct ExtBuilder {
	existential_deposit: u64,
	gas_price: u64,
	block_gas_limit: u64,
	transfer_fee: u64,
	creation_fee: u64,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 0,
			gas_price: 2,
			block_gas_limit: 100_000_000,
			transfer_fee: 0,
			creation_fee: 0,
		}
	}
}
impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}
	pub fn gas_price(mut self, gas_price: u64) -> Self {
		self.gas_price = gas_price;
		self
	}
	pub fn block_gas_limit(mut self, block_gas_limit: u64) -> Self {
		self.block_gas_limit = block_gas_limit;
		self
	}
	pub fn transfer_fee(mut self, transfer_fee: u64) -> Self {
		self.transfer_fee = transfer_fee;
		self
	}
	pub fn creation_fee(mut self, creation_fee: u64) -> Self {
		self.creation_fee = creation_fee;
		self
	}
	pub fn build(self) -> sr_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default()
			.build_storage()
			.unwrap()
			.0;
		t.extend(
			balances::GenesisConfig::<Test> {
				transaction_base_fee: 0,
				transaction_byte_fee: 0,
				balances: vec![],
				existential_deposit: self.existential_deposit,
				transfer_fee: self.transfer_fee,
				creation_fee: self.creation_fee,
				vesting: vec![],
			}
				.build_storage()
				.unwrap()
				.0,
		);
		t.extend(
			contract::GenesisConfig::<Test> {
				transaction_base_fee: 0,
				transaction_byte_fee: 0,
				transfer_fee: self.transfer_fee,
				creation_fee: self.creation_fee,
				contract_fee: 21,
				call_base_fee: 135,
				create_base_fee: 175,
				gas_price: self.gas_price,
				max_depth: 100,
				block_gas_limit: self.block_gas_limit,
				current_schedule: Default::default(),
			}
				.build_storage()
				.unwrap()
				.0,
		);
		sr_io::TestExternalities::new(t)
	}
}


#[test]
fn it_works() {assert!(true); }
