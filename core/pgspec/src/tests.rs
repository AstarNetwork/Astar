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

use super::*;
use parity_codec::{Decode, Encode, KeyedVec};
use primitives::{sr25519, storage::well_known_keys, Blake2Hasher, crypto::{Pair, UncheckedFrom}};
use sr_io;
use sr_io::with_externalities;
use sr_primitives::testing::{Digest, DigestItem, Header, UintAuthorityId, H256};
use sr_primitives::traits::{Hash, BlakeTwo256, IdentityLookup};
use sr_primitives::BuildStorage;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::marker::PhantomData;
use support::{
    assert_err, assert_ok, impl_outer_dispatch, impl_outer_event, impl_outer_origin,
    parameter_types,
    storage::child,
    traits::{Currency, Get},
    StorageMap, StorageValue,
};
use system::{self, EventRecord, Phase};

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

/// Alias to pubkey that identifies an account on the chain.
pub type AccountId = <AccountSignature as Verify>::Signer;
/// The type used by authorities to prove their ID.
pub type AccountSignature = sr25519::Signature;

thread_local! {
    static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
    static TRANSFER_FEE: RefCell<u64> = RefCell::new(0);
    static CREATION_FEE: RefCell<u64> = RefCell::new(0);
    static BLOCK_GAS_LIMIT: RefCell<u64> = RefCell::new(0);
}

pub struct ExistentialDeposit;

impl Get<u64> for ExistentialDeposit {
    fn get() -> u64 {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow())
    }
}

pub struct TransferFee;

impl Get<u64> for TransferFee {
    fn get() -> u64 {
        TRANSFER_FEE.with(|v| *v.borrow())
    }
}

pub struct CreationFee;

impl Get<u64> for CreationFee {
    fn get() -> u64 {
        CREATION_FEE.with(|v| *v.borrow())
    }
}

pub struct BlockGasLimit;

impl Get<u64> for BlockGasLimit {
    fn get() -> u64 {
        BLOCK_GAS_LIMIT.with(|v| *v.borrow())
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
}
impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = MetaEvent;
    type BlockHashCount = BlockHashCount;
}
parameter_types! {
    pub const BalancesTransactionBaseFee: u64 = 0;
    pub const BalancesTransactionByteFee: u64 = 0;
}
impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = Contract;
    type OnNewAccount = ();
    type Event = MetaEvent;
    type TransactionPayment = ();
    type DustRemoval = ();
    type TransferPayment = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
    type TransactionBaseFee = BalancesTransactionBaseFee;
    type TransactionByteFee = BalancesTransactionByteFee;
}
parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}
parameter_types! {
    pub const SignedClaimHandicap: u64 = 2;
    pub const TombstoneDeposit: u64 = 16;
    pub const StorageSizeOffset: u32 = 8;
    pub const RentByteFee: u64 = 4;
    pub const RentDepositOffset: u64 = 10_000;
    pub const SurchargeReward: u64 = 150;
    pub const TransactionBaseFee: u64 = 2;
    pub const TransactionByteFee: u64 = 6;
    pub const ContractFee: u64 = 21;
    pub const CallBaseFee: u64 = 135;
    pub const CreateBaseFee: u64 = 175;
    pub const MaxDepth: u32 = 100;
}
impl contract::Trait for Test {
    type Currency = Balances;
    type Call = Call;
    type DetermineContractAddress = DummyContractAddressFor<Test>;
    type Event = MetaEvent;
    type ComputeDispatchFee = DummyComputeDispatchFee;
    type TrieIdGenerator = DummyTrieIdGenerator<Test>;
    type GasPayment = ();
    type SignedClaimHandicap = SignedClaimHandicap;
    type TombstoneDeposit = TombstoneDeposit;
    type StorageSizeOffset = StorageSizeOffset;
    type RentByteFee = RentByteFee;
    type RentDepositOffset = RentDepositOffset;
    type SurchargeReward = SurchargeReward;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
    type TransactionBaseFee = TransactionBaseFee;
    type TransactionByteFee = TransactionByteFee;
    type ContractFee = ContractFee;
    type CallBaseFee = CallBaseFee;
    type CreateBaseFee = CreateBaseFee;
    type MaxDepth = MaxDepth;
    type BlockGasLimit = BlockGasLimit;
}

impl Trait for Test {
    type RangeNumber = u128;
    type Data = Self::AccountId;
    type TxBody = ();
    type Signature = AccountSignature;

    type Event = MetaEvent;
}

type Balances = balances::Module<Test>;
type Contract = contract::Module<Test>;
type System = system::Module<Test>;
type PGSpec = Module<Test>;

pub struct DummyContractAddressFor<T: Trait>(PhantomData<T>);
impl<T: Trait> contract::ContractAddressFor<contract::CodeHash<T>, T::AccountId> for DummyContractAddressFor<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn contract_address_for(
        code_hash: &contract::CodeHash<T>,
        data: &[u8],
        origin: &T::AccountId,
    ) -> T::AccountId {
        let data_hash = T::Hashing::hash(data);

        let mut buf = Vec::new();
        buf.extend_from_slice(code_hash.as_ref());
        buf.extend_from_slice(data_hash.as_ref());
        buf.extend_from_slice(origin.as_ref());

        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}

pub struct DummyTrieIdGenerator<T: Trait>(PhantomData<T>);
/// This generator uses inner counter for account id and applies the hash over `AccountId +
/// accountid_counter`.
impl<T: Trait> contract::TrieIdGenerator<T::AccountId> for DummyTrieIdGenerator<T>
	where
		T::AccountId: AsRef<[u8]>
{
	fn trie_id(account_id: &T::AccountId) -> contract::TrieId {
		// Note that skipping a value due to error is not an issue here.
		// We only need uniqueness, not sequence.
		let new_seed = contract::AccountCounter::mutate(|v| {
			*v = v.wrapping_add(1);
			*v
		});

		let mut buf = Vec::new();
		buf.extend_from_slice(account_id.as_ref());
		buf.extend_from_slice(&new_seed.to_le_bytes()[..]);

		// TODO: see https://github.com/paritytech/substrate/issues/2325
		well_known_keys::CHILD_STORAGE_KEY_PREFIX.iter()
			.chain(b"default:")
			.chain(T::Hashing::hash(&buf[..]).as_ref().iter())
			.cloned()
			.collect()
	}
}

pub struct DummyComputeDispatchFee;

impl contract::ComputeDispatchFee<Call, u64> for DummyComputeDispatchFee {
    fn compute_dispatch_fee(call: &Call) -> u64 {
        69
    }
}

fn account_key(s: &str) -> AccountId {
    sr25519::Pair::from_string(&format!("//{}", s), None)
        .expect("static values are valid; qed")
        .public()
}

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
    pub fn set_associated_consts(&self) {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
        TRANSFER_FEE.with(|v| *v.borrow_mut() = self.transfer_fee);
        CREATION_FEE.with(|v| *v.borrow_mut() = self.creation_fee);
        BLOCK_GAS_LIMIT.with(|v| *v.borrow_mut() = self.block_gas_limit);
    }
    pub fn build(self) -> sr_io::TestExternalities<Blake2Hasher> {
        self.set_associated_consts();
        let mut t = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        balances::GenesisConfig::<Test> {
            balances: vec![],
            vesting: vec![],
        }
        .assimilate_storage(&mut t.0, &mut t.1)
        .unwrap();
        contract::GenesisConfig::<Test> {
            current_schedule: Default::default(),
            gas_price: self.gas_price,
        }
        .assimilate_storage(&mut t.0, &mut t.1)
        .unwrap();
        sr_io::TestExternalities::new_with_children(t)
    }
}

//const ALICE: AccountId = account_key("Alice");
//const BOB: AccountId = account_key("Bob");
//const CHARLIE: AccountId = account_key("Charlie");

#[test]
fn it_works() {
    assert!(true);
}
