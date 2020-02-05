use super::*;
use primitives::storage::well_known_keys;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sp_runtime::{
    testing::{Digest, DigestItem, Header, UintAuthorityId, H256},
    traits::{BlakeTwo256, Hash, IdentityLookup, SignedExtension},
    BuildStorage, DispatchError, Perbill,
};
use sp_std::marker::PhantomData;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};
use support::{
    assert_err, assert_ok, impl_outer_dispatch, impl_outer_event, impl_outer_origin,
    parameter_types,
    storage::child,
    traits::{Currency, Get},
    weights::{DispatchClass, DispatchInfo},
    StorageMap, StorageValue,
};
use system::{self, EventRecord, Phase};

mod trading {
    // Re-export contents of the root. This basically
    // needs to give a name for the current crate.
    // This hack is required for `impl_outer_event!`.
    pub use super::super::*;
    use support::impl_outer_event;
}
impl_outer_event! {
    pub enum MetaEvent for Test {
        balances<T>, trading<T>,
    }
}
impl_outer_origin! {
    pub enum Origin for Test { }
}

pub type AccountId = u64;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: u32 = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Call = ();
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = MetaEvent;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type AvailableBlockRatio = AvailableBlockRatio;
    type MaximumBlockLength = MaximumBlockLength;
    type Version = ();
    type ModuleToIndex = ();
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
    pub const TransferFee : u64 = 0;
    pub const CreationFee : u64 = 0;
}
impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = ();
    type OnNewAccount = ();
    type Event = MetaEvent;
    type DustRemoval = ();
    type TransferPayment = ();
    type ExistentialDeposit = ExistentialDeposit;
    type TransferFee = TransferFee;
    type CreationFee = CreationFee;
}
parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}

// define mock operator trait
pub trait MockOperatorTrait: system::Trait {}
decl_storage! {
    trait Store for MockOperatorModule<T: MockOperatorTrait> as MockOperator {
        /// A mapping from operators to operated contracts by them.
        pub OperatorHasContracts: map T::AccountId => Vec<T::AccountId>;
     }
}
pub struct MockOperatorModule<T: MockOperatorTrait>(PhantomData<T>);
impl<T: MockOperatorTrait> OperatorFinder<T::AccountId> for MockOperatorModule<T> {
    fn contracts(operator_id: &T::AccountId) -> Vec<T::AccountId> {
        <OperatorHasContracts<T>>::get(operator_id)
    }
}
impl<T: MockOperatorTrait> TransferOperator<T::AccountId> for MockOperatorModule<T> {
    /// Force Changes an operator for identified contracts without verify.
    fn force_transfer_operator(
        current_operator: T::AccountId,
        contracts: Vec<T::AccountId>,
        new_operator: T::AccountId,
    ) {
        // remove origin operator to contracts
        <OperatorHasContracts<T>>::mutate(&current_operator, |tree| {
            *tree = tree
                .iter()
                .filter(|&x| !contracts.contains(x))
                .cloned()
                .collect()
        });

        // add new_operator to contracts
        <OperatorHasContracts<T>>::mutate(&new_operator, |tree| {
            for c in contracts.iter() {
                (*tree).push(c.clone());
            }
        });
    }
}

impl MockOperatorTrait for Test {}

impl Trait for Test {
    type OperatorFinder = MockOperatorModule<Test>;
    /// The helper of transfering operator's authorities.
    type TransferOperator = MockOperatorModule<Test>;
    type Event = MetaEvent;
}

struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut t = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        balances::GenesisConfig::<Test> {
            balances: vec![],
            vesting: vec![],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        sp_io::TestExternalities::new(t)
    }
}

type System = system::Module<Test>;
type Balancecs = balances::Module<Test>;
type Timestamp = timestamp::Module<Test>;

#[test]
fn instantiate_and_call_and_deposit_event() {
    ExtBuilder::build().execute_with(|| {
        assert!(true);
    })
}
