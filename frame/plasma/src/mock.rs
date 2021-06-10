//! Test utilities

#![cfg(test)]

use super::*;
use crate::{self as pallet_plasma};
pub use frame_support::{
    impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types,
    traits::OnFinalize,
    weights::{WeightToFeeCoefficients, WeightToFeePolynomial},
};
pub use hex_literal::hex;
pub use pallet_balances as balances;
pub use pallet_contracts::{self as contracts, weights::WeightInfo, TrieId};
pub use pallet_ovm::{self as ovm, AtomicPredicateIdConfig};
use sp_core::crypto::AccountId32;
pub use sp_core::{
    crypto::{key_types, UncheckedInto},
    Pair, H256,
};
pub use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, Hash, IdentifyAccount, IdentityLookup, Keccak256},
    Perbill,
};

pub type BlockNumber = u64;
pub type AccountId = AccountId32;
pub type Balance = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

lazy_static::lazy_static! {
    pub static ref ALICE_STASH: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000005553"
    ]);
        pub static ref BOB_STASH: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000008553"
    ]);
        pub static ref CHARLIE_STASH: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000009553"
    ]);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let _ = pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            ((*ALICE_STASH).clone(), 1_000_000_000_000_000_000),
            ((*BOB_STASH).clone(), 5_000_000_000_000_000_000),
            ((*CHARLIE_STASH).clone(), 10_000_000_000_000_000_000),
        ],
    }
    .assimilate_storage(&mut storage);

    let _ = ovm::GenesisConfig {
        current_schedule: Default::default(),
    }
    .assimilate_storage(&mut storage);

    storage.into()
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = sp_core::H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Test>;
    type WeightInfo = ();
    type MaxLocks = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const TombstoneDeposit: Balance = 1;
    pub const DepositPerContract: Balance = TombstoneDeposit::get();
    pub const DepositPerStorageByte: Balance = 1;
    pub const DepositPerStorageItem: Balance = 1;
    pub RentFraction: Perbill = Perbill::from_rational_approximation(1u32, 30);
    pub const SurchargeReward: Balance = 150_000_000;
    pub const SignedClaimHandicap: u32 = 2;
    pub const MaxDepth: u32 = 32;
    pub const MaxValueSize: u32 = 16 * 1024;
    // The lazy deletion runs inside on_initialize.
    pub DeletionWeightLimit: Weight = 10_000_000;
    // The weight needed for decoding the queue should be less or equal than a fifth
    // of the overall weight dedicated to the lazy deletion.
    pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
            <Test as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
            <Test as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
        )) / 5) as u32;
    pub MaxCodeSize: u32 = 128 * 1024;
}

impl pallet_contracts::Config for Test {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type Event = Event;
    type RentPayment = ();
    type SignedClaimHandicap = SignedClaimHandicap;
    type TombstoneDeposit = TombstoneDeposit;
    type DepositPerContract = DepositPerContract;
    type DepositPerStorageByte = DepositPerStorageByte;
    type DepositPerStorageItem = DepositPerStorageItem;
    type RentFraction = RentFraction;
    type SurchargeReward = SurchargeReward;
    type MaxDepth = MaxDepth;
    type MaxValueSize = MaxValueSize;
    type WeightPrice = ();
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type DeletionQueueDepth = DeletionQueueDepth;
    type DeletionWeightLimit = DeletionWeightLimit;
    type MaxCodeSize = MaxCodeSize;
}

parameter_types! {
    pub const DisputePeriod: BlockNumber = 7;
}

lazy_static::lazy_static! {
    pub static ref NOT_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000003"
    ]);
    pub static ref AND_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000004"
    ]);
    pub static ref OR_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000005"
    ]);
    pub static ref FOR_ALL_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000006"
    ]);
    pub static ref THERE_EXISTS_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000007"
    ]);
    pub static ref EQUAL_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000008"
    ]);
    pub static ref IS_CONTAINED_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000009"
    ]);
    pub static ref IS_LESS_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000010"
    ]);
    pub static ref IS_STORED_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000011"
    ]);
    pub static ref IS_VALID_SIGNATURE_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000012"
    ]);
    pub static ref VERIFY_INCLUSION_ADDRESS: AccountId = to_account_from_seed(&hex![
        "0000000000000000000000000000000000000000000000000000000000000013"
    ]);
    pub static ref SECP_256_K1: H256 = H256::from(&hex![
        "d4fa99b1e08c4e5e6deb461846aa629344d95ff03ed04754c2053d54c756f439"
    ]);
}

pub struct MockAtomicPredicateIdConfigGetter;
impl Get<AtomicPredicateIdConfig<AccountId, H256>> for MockAtomicPredicateIdConfigGetter {
    fn get() -> AtomicPredicateIdConfig<AccountId, H256> {
        AtomicPredicateIdConfig {
            not_address: (*NOT_ADDRESS).clone(),
            and_address: (*AND_ADDRESS).clone(),
            or_address: (*OR_ADDRESS).clone(),
            for_all_address: (*FOR_ALL_ADDRESS).clone(),
            there_exists_address: (*THERE_EXISTS_ADDRESS).clone(),
            equal_address: (*EQUAL_ADDRESS).clone(),
            is_contained_address: (*IS_CONTAINED_ADDRESS).clone(),
            is_less_address: (*IS_LESS_ADDRESS).clone(),
            is_stored_address: (*IS_STORED_ADDRESS).clone(),
            is_valid_signature_address: (*IS_VALID_SIGNATURE_ADDRESS).clone(),
            verify_inclusion_address: (*VERIFY_INCLUSION_ADDRESS).clone(),
            secp256k1: (*SECP_256_K1).clone(),
        }
    }
}

impl pallet_ovm::Config for Test {
    type MaxDepth = MaxDepth;
    type DisputePeriod = DisputePeriod;
    type DeterminePredicateAddress = ovm::SimpleAddressDeterminer<Test>;
    type HashingL2 = Keccak256;
    type ExternalCall = ovm::predicate::CallContext<Test>;
    type AtomicPredicateIdConfig = MockAtomicPredicateIdConfigGetter;
    type Event = Event;
}

pub struct MaximumTokenAddress;
impl Get<AccountId> for MaximumTokenAddress {
    fn get() -> AccountId {
        H256::from(&hex![
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        ])
        .unchecked_into()
    }
}

impl Config for Test {
    type Currency = Balances;
    type DeterminePlappsAddress = SimpleAddressDeterminer<Test>;
    type MaximumTokenAddress = MaximumTokenAddress;
    type PlasmaHashing = Keccak256;
    type Event = Event;
}

pub fn advance_block() {
    System::finalize();
    let next = System::block_number() + 1;
    // increase block numebr
    System::set_block_number(next);
    System::initialize(
        &next,
        &[0u8; 32].into(),
        &Default::default(),
        system::InitKind::Full,
    );
    System::note_finished_initialize();
}

/// Generate compiled predicate binary and code hash from predicate source.
pub fn compile_predicate<T>(predicate_module: &str) -> (Vec<u8>, <T::Hashing as Hash>::Output)
where
    T: frame_system::Config,
{
    // TODO actually predicate to compiled predicate.
    let compiled_predicate = predicate_module.as_bytes().to_vec();
    let code_hash = T::Hashing::hash_of(&compiled_predicate);
    (compiled_predicate.to_vec(), code_hash)
}

pub fn to_account_from_seed(seed: &[u8; 32]) -> AccountId {
    to_account(sp_core::ecdsa::Pair::from_seed(&seed).public().as_ref())
}

pub fn to_account(full_public: &[u8]) -> AccountId {
    let public = sp_core::ecdsa::Public::from_full(full_public).unwrap();
    sp_runtime::MultiSigner::from(public).into_account()
}

/// For merkle Tree simulator
pub fn compute_parent(
    a: &IntervalTreeNodeOf<Test>,
    b: &IntervalTreeNodeOf<Test>,
) -> IntervalTreeNodeOf<Test> {
    IntervalTreeNodeOf::<Test> {
        start: b.start.clone(),
        data: <Test as Config>::PlasmaHashing::hash_of(&(&(&a.data, &a.start, &b.data, &b.start))),
    }
}

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Module, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        Timestamp: pallet_timestamp::{Module, Storage},
        Contracts: pallet_contracts::{Module, Call, Storage, Event<T>, Config<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Ovm: pallet_ovm::{Module, Call, Storage, Config, Event<T>},
        Plasma: pallet_plasma::{Module, Call, Storage, Event<T>},
    }
);
