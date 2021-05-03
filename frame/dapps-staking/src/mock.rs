//! Test utilities

#![cfg(test)]

use super::*;
use frame_support::{
    assert_ok, parameter_types,
    traits::OnFinalize,
    weights::{WeightToFeeCoefficients, WeightToFeePolynomial},
};
use pallet_contracts::Gas;
use pallet_plasm_rewards::{inflation::SimpleComputeTotalPayout, traits::MaybeValidators};
use pallet_transaction_payment::CurrencyAdapter;
use sp_core::{crypto::key_types, H256};
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, ConvertInto, Hash, IdentityLookup, OpaqueKeys},
    AccountId32, KeyTypeId, Perbill,
};

pub type BlockNumber = u64;
pub type Balance = u64;
pub type AccountId = AccountId32;

pub const ALICE_STASH: AccountId = AccountId32::new([1u8; 32]);
pub const BOB_STASH: AccountId = AccountId32::new([2u8; 32]);
pub const ALICE_CTRL: AccountId = AccountId32::new([3u8; 32]);
pub const BOB_CTRL: AccountId = AccountId32::new([4u8; 32]);
pub const VALIDATOR_A: AccountId = AccountId32::new([5u8; 32]);
pub const VALIDATOR_B: AccountId = AccountId32::new([6u8; 32]);
pub const OPERATOR_A: AccountId = AccountId32::new([9u8; 32]);
pub const OPERATOR_B: AccountId = AccountId32::new([10u8; 32]);
pub const OPERATED_CONTRACT_A: AccountId = AccountId32::new([19u8; 32]);
pub const OPERATED_CONTRACT_B: AccountId = AccountId32::new([20u8; 32]);
pub const BOB_CONTRACT: AccountId = AccountId32::new([12u8; 32]);

use crate as dapps_staking;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>, Config<T>},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Contracts: pallet_contracts::{Module, Call, Storage, Event<T>, Config<T>},
        Operator: pallet_contract_operator::{Module, Call, Storage, Event<T>},
        PlasmRewards: pallet_plasm_rewards::{Module, Call, Storage, Config, Event<T>},
        DappsStaking: dapps_staking::{Module, Call, Storage, Event<T>},
    }
);

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let _ = pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE_STASH, 1000),
            (BOB_STASH, 2000),
            (ALICE_CTRL, 10),
            (BOB_CTRL, 20),
            (VALIDATOR_A, 1_000_000),
            (VALIDATOR_B, 1_000_000),
        ],
    }
    .assimilate_storage(&mut storage);

    let _ = pallet_contracts::GenesisConfig::<Test> {
        current_schedule: pallet_contracts::Schedule {
            enable_println: true,
            ..Default::default()
        },
    }
    .assimilate_storage(&mut storage);

    let _ = pallet_plasm_rewards::GenesisConfig {
        ..Default::default()
    }
    .assimilate_storage(&mut storage);

    storage.into()
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: u32 = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl system::Config for Test {
    type Origin = Origin;
    type BaseCallFilter = ();
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type Version = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type BlockHashCount = ();
    type PalletInfo = PalletInfo;
    type SS58Prefix = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

pub struct TestSessionHandler;

impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];
    fn on_genesis_session<T: OpaqueKeys>(_validators: &[(AccountId, T)]) {}
    fn on_new_session<T: OpaqueKeys>(
        _changed: bool,
        _validators: &[(AccountId, T)],
        _queued_validators: &[(AccountId, T)],
    ) {
    }
    fn on_disabled(_validator_index: usize) {}
    fn on_before_session_ending() {}
}

impl pallet_session::Config for Test {
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = PlasmRewards;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = AccountId32;
    type ValidatorIdOf = ConvertInto;
    type Keys = UintAuthorityId;
    type Event = ();
    type DisabledValidatorsThreshold = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 10;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = system::Module<Test>;
    type WeightInfo = ();
    type MaxLocks = ();
}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = u64;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        Default::default()
    }
}

parameter_types! {
    pub const TransactionByteFee: u64 = 0;
}

impl pallet_transaction_payment::Config for Test {
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = WeightToFee;
    type FeeMultiplierUpdate = ();
    type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
}

pub struct DummyContractAddressFor;
impl pallet_contracts::ContractAddressFor<H256, AccountId> for DummyContractAddressFor {
    fn contract_address_for(_code_hash: &H256, _data: &[u8], origin: &AccountId) -> AccountId {
        origin.clone()
    }
}

parameter_types! {
    pub const ContractTransactionBaseFee: Balance = 0;
    pub const ContractTransactionByteFee: Balance = 0;
    pub const ContractFee: Balance = 0;
    pub const SignedClaimHandicap: u32 = 2;
    pub const TombstoneDeposit: Balance = 0;
    pub const DepositPerContract: Balance = TombstoneDeposit::get();
    pub const DepositPerStorageByte: Balance = 1;
    pub const DepositPerStorageItem: Balance = 1;
    pub RentFraction: Perbill = Perbill::from_rational_approximation(1u32, 30);
    pub const RentByteFee: Balance = 0;
    pub const RentDepositOffset: Balance = 0;
    pub const SurchargeReward: Balance = 0;
    pub const MaxDepth: u32 = 32;
    pub const MaxValueSize: u32 = 16 * 1024;
    pub DeletionQueueDepth: u32 = 1024;
    pub DeletionWeightLimit: Weight = 10_000_000;
}

impl pallet_contracts::Config for Test {
    type Time = Timestamp;
    type Randomness = pallet_randomness_collective_flip::Module<Test>;
    type Currency = Balances;
    type Event = ();
    type DetermineContractAddress = DummyContractAddressFor;
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
    type WeightPrice = pallet_transaction_payment::Module<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type DeletionQueueDepth = DeletionQueueDepth;
    type DeletionWeightLimit = DeletionWeightLimit;
}

impl pallet_contract_operator::Config for Test {
    type Parameters = parameters::StakingParameters;
    type Event = ();
}

pub struct DummyMaybeValidators;
impl MaybeValidators<EraIndex, AccountId> for DummyMaybeValidators {
    fn compute(_current_era: EraIndex) -> Option<Vec<AccountId>> {
        Some(vec![
            AccountId32::new([1u8; 32]),
            AccountId32::new([2u8; 32]),
            AccountId32::new([3u8; 32]),
        ])
    }
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 10;
    pub const BondingDuration: EraIndex = 3;
}

impl pallet_plasm_rewards::Config for Test {
    type Currency = Balances;
    type Time = Timestamp;
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type ComputeEraForDapps = DappsStaking;
    type ComputeEraForSecurity = DappsStaking;
    type ComputeTotalPayout = SimpleComputeTotalPayout;
    type MaybeValidators = DummyMaybeValidators;
    type Event = ();
}

impl Config for Test {
    type Currency = Balances;
    type BondingDuration = BondingDuration;
    type ContractFinder = Operator;
    type RewardRemainder = (); // Reward remainder is burned.
    type Reward = (); // Reward is minted.
    type Time = Timestamp;
    type ComputeRewardsForDapps = rewards::VoidableRewardsForDapps;
    type EraFinder = PlasmRewards;
    type ForDappsEraReward = PlasmRewards;
    type HistoryDepthFinder = PlasmRewards;
    type Event = ();
}

/// Generate Wasm binary and code hash from wabt source.
pub fn compile_module<T>(
    wabt_module: &str,
) -> result::Result<(Vec<u8>, <T::Hashing as Hash>::Output), wabt::Error>
where
    T: system::Config,
{
    let wasm = wabt::wat2wasm(wabt_module)?;
    let code_hash = T::Hashing::hash(&wasm);
    Ok((wasm, code_hash))
}

pub const CODE_RETURN_FROM_START_FN: &str = r#"
(module
    (import "env" "ext_return" (func $ext_return (param i32 i32)))
    (import "env" "ext_deposit_event" (func $ext_deposit_event (param i32 i32 i32 i32)))
    (import "env" "memory" (memory 1 1))

    (start $start)
    (func $start
        (call $ext_deposit_event
            (i32.const 0) ;; The topics buffer
            (i32.const 0) ;; The topics buffer's length
            (i32.const 8) ;; The data buffer
            (i32.const 4) ;; The data buffer's length
        )
        (call $ext_return
            (i32.const 8)
            (i32.const 4)
        )
        (unreachable)
    )

    (func (export "call")
        (unreachable)
    )
    (func (export "deploy"))

    (data (i32.const 8) "\01\02\03\04")
)
"#;

pub const CODE_RETURN_FROM_START_FN_B: &str = CODE_RETURN_FROM_START_FN;

const GAS_LIMIT: Gas = 10_000_000_000;

pub fn valid_instatiate() {
    let (wasm, code_hash) = compile_module::<Test>(CODE_RETURN_FROM_START_FN).unwrap();

    let (wasm_b, code_hash_b) = compile_module::<Test>(CODE_RETURN_FROM_START_FN_B).unwrap();

    let subsistence = Contracts::subsistence_threshold();

    // prepare
    let _ = Balances::deposit_creating(&OPERATOR_A, 1_000_000);
    assert_ok!(Contracts::instantiate_with_code(
        Origin::signed(OPERATOR_A),
        subsistence * 100,
        GAS_LIMIT,
        wasm,
        vec![],
        vec![]
    ));

    let _ = Balances::deposit_creating(&OPERATOR_B, 1_000_000);
    assert_ok!(Contracts::instantiate_with_code(
        Origin::signed(OPERATOR_B),
        subsistence * 100,
        GAS_LIMIT,
        wasm_b,
        vec![],
        vec![]
    ));

    let test_params = parameters::StakingParameters {
        can_be_nominated: true,
        option_expired: 100,
        option_p: Perbill::from_percent(20).deconstruct(),
    };

    // instantiate
    // Check at the end to get hash on error easily
    let _ = Operator::instantiate(
        Origin::signed(OPERATOR_A),
        100,
        Gas::max_value(),
        code_hash.into(),
        vec![],
        vec![],
        test_params.clone(),
    );
    let _ = Operator::instantiate(
        Origin::signed(OPERATOR_B),
        100,
        Gas::max_value(),
        code_hash_b.into(),
        vec![],
        vec![],
        test_params.clone(),
    );

    // checks deployed contract
    assert!(pallet_contracts::ContractInfoOf::<Test>::contains_key(
        OPERATED_CONTRACT_A
    ));
    assert!(pallet_contracts::ContractInfoOf::<Test>::contains_key(
        OPERATED_CONTRACT_B
    ));

    // checks mapping operator and contract
    // OPERATOR_A operates a only OPERATED_CONTRACT_A contract.
    assert!(pallet_contract_operator::OperatorHasContracts::<Test>::contains_key(OPERATOR_A));
    let tree = pallet_contract_operator::OperatorHasContracts::<Test>::get(&OPERATOR_A);
    assert_eq!(tree.len(), 1);
    assert!(tree.contains(&OPERATED_CONTRACT_A));

    // checks mapping operator and contract
    // OPERATOR_B operates a only OPERATED_CONTRACT_B contract.
    assert!(pallet_contract_operator::OperatorHasContracts::<Test>::contains_key(OPERATOR_B));
    let tree = pallet_contract_operator::OperatorHasContracts::<Test>::get(&OPERATOR_B);
    assert_eq!(tree.len(), 1);
    assert!(tree.contains(&OPERATED_CONTRACT_B));

    // OPERATED_CONTRACT_A contract is operated by OPERATOR_A.
    assert!(
        pallet_contract_operator::ContractHasOperator::<Test>::contains_key(OPERATED_CONTRACT_A)
    );
    assert_eq!(
        pallet_contract_operator::ContractHasOperator::<Test>::get(&OPERATED_CONTRACT_A),
        Some(OPERATOR_A)
    );

    // OPERATED_CONTRACT_B contract is operated by OPERATOR_B.
    assert!(
        pallet_contract_operator::ContractHasOperator::<Test>::contains_key(OPERATED_CONTRACT_B)
    );
    assert_eq!(
        pallet_contract_operator::ContractHasOperator::<Test>::get(&OPERATED_CONTRACT_B),
        Some(OPERATOR_B)
    );

    // OPERATED_CONTRACT's contract Parameters is same test_params.
    assert!(
        pallet_contract_operator::ContractParameters::<Test>::contains_key(OPERATED_CONTRACT_A)
    );
    assert_eq!(
        pallet_contract_operator::ContractParameters::<Test>::get(&OPERATED_CONTRACT_A),
        Some(test_params.clone())
    );

    // OPERATED_CONTRACT_B's contract Parameters is same test_params.
    assert!(
        pallet_contract_operator::ContractParameters::<Test>::contains_key(OPERATED_CONTRACT_B)
    );
    assert_eq!(
        pallet_contract_operator::ContractParameters::<Test>::get(&OPERATED_CONTRACT_B),
        Some(test_params)
    );
}

pub const PER_SESSION: u64 = 60 * 1000;

pub fn advance_session() {
    // increase block numebr
    let next = System::block_number() + 1;
    System::set_block_number(next);
    // increase timestamp + 10
    let now_time = Timestamp::get();
    Timestamp::set_timestamp(now_time + PER_SESSION);
    Session::rotate_session();
    assert_eq!(Session::current_index(), (next / Period::get()) as u32);

    // on finalize
    PlasmRewards::on_finalize(next);
}

pub fn advance_era() {
    let current_era = PlasmRewards::current_era().unwrap_or(Zero::zero());
    while current_era == PlasmRewards::current_era().unwrap_or(Zero::zero()) {
        advance_session();
    }
}
