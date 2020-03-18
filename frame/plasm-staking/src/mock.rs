//! Test utilities

#![cfg(test)]

use super::*;
use primitives::{crypto::key_types, H256};
use sp_runtime::testing::{Header, UintAuthorityId};
use sp_runtime::traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys};
use sp_runtime::{traits::Hash, KeyTypeId, Perbill};
use support::{assert_ok, impl_outer_dispatch, impl_outer_origin, parameter_types};

pub type BlockNumber = u64;
pub type AccountId = u64;
pub type Balance = u64;

pub const ALICE_STASH: u64 = 1;
pub const BOB_STASH: u64 = 2;
pub const ALICE_CTRL: u64 = 3;
pub const BOB_CTRL: u64 = 4;
pub const VALIDATOR_A: u64 = 5;
pub const VALIDATOR_B: u64 = 6;
pub const VALIDATOR_C: u64 = 7;
pub const VALIDATOR_D: u64 = 8;
pub const OPERATOR: u64 = 9;
pub const OPERATED_CONTRACT: u64 = 19;
pub const BOB_CONTRACT: u64 = 12;

impl_outer_origin! {
    pub enum Origin for Test {}
}

impl_outer_dispatch! {
    pub enum Call for Test where origin: Origin {
        session::Session,
        balances::Balances,
        contracts::Contract,
        plasm_staking::PlasmStaking,
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let _ = balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE_STASH, 1000),
            (BOB_STASH, 2000),
            (ALICE_CTRL, 10),
            (BOB_CTRL, 20),
            (VALIDATOR_A, 1_000_000),
            (VALIDATOR_B, 1_000_000),
            (VALIDATOR_C, 1_000_000),
            (VALIDATOR_D, 1_000_000),
        ],
    }
    .assimilate_storage(&mut storage);

    let _ = contracts::GenesisConfig::<Test> {
        current_schedule: contracts::Schedule {
            enable_println: true,
            ..Default::default()
        },
        gas_price: 2,
    }
    .assimilate_storage(&mut storage);

    let validators = vec![1, 2];

    let _ = GenesisConfig::<Test> {
        storage_version: 1,
        force_era: Forcing::NotForcing,
        validators: validators.clone(),
    }
    .assimilate_storage(&mut storage);

    let _ = session::GenesisConfig::<Test> {
        keys: validators
            .iter()
            .map(|x| (*x, UintAuthorityId(*x)))
            .collect(),
    }
    .assimilate_storage(&mut storage);

    storage.into()
}

#[derive(Clone, PartialEq, Eq, Debug)]
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
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = balances::AccountData<u64>;
    type MigrateAccount = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
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
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

pub struct TestSessionHandler;

impl session::SessionHandler<u64> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [KeyTypeId] = &[key_types::DUMMY];
    fn on_genesis_session<T: OpaqueKeys>(_validators: &[(u64, T)]) {}
    fn on_new_session<T: OpaqueKeys>(
        _changed: bool,
        _validators: &[(u64, T)],
        _queued_validators: &[(u64, T)],
    ) {
    }
    fn on_disabled(_validator_index: usize) {}
    fn on_before_session_ending() {}
}

impl session::Trait for Test {
    type ShouldEndSession = session::PeriodicSessions<Period, Offset>;
    type SessionManager = PlasmStaking;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type Keys = UintAuthorityId;
    type Event = ();
    type DisabledValidatorsThreshold = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 10;
}

impl balances::Trait for Test {
    type Balance = Balance;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = system::Module<Test>;
}

pub struct DummyContractAddressFor;
impl contracts::ContractAddressFor<H256, u64> for DummyContractAddressFor {
    fn contract_address_for(_code_hash: &H256, _data: &[u8], origin: &u64) -> u64 {
        *origin + 10
    }
}

pub struct DummyTrieIdGenerator;

impl contracts::TrieIdGenerator<u64> for DummyTrieIdGenerator {
    fn trie_id(account_id: &u64) -> contracts::TrieId {
        use primitives::storage::well_known_keys;

        let new_seed = contracts::AccountCounter::mutate(|v| {
            *v = v.wrapping_add(1);
            *v
        });

        // TODO: see https://github.com/paritytech/substrate/issues/2325
        let mut res = vec![];
        res.extend_from_slice(well_known_keys::CHILD_STORAGE_KEY_PREFIX);
        res.extend_from_slice(b"default:");
        res.extend_from_slice(&new_seed.to_le_bytes());
        res.extend_from_slice(&account_id.to_le_bytes());
        res
    }
}

pub struct DummyComputeDispatchFee;
impl contracts::ComputeDispatchFee<Call, u64> for DummyComputeDispatchFee {
    fn compute_dispatch_fee(_call: &Call) -> u64 {
        69
    }
}

parameter_types! {
    pub const ContractTransactionBaseFee: Balance = 0;
    pub const ContractTransactionByteFee: Balance = 0;
    pub const ContractFee: Balance = 0;
    pub const TombstoneDeposit: Balance = 0;
    pub const RentByteFee: Balance = 0;
    pub const RentDepositOffset: Balance = 0;
    pub const SurchargeReward: Balance = 0;
}

impl contracts::Trait for Test {
    type Currency = Balances;
    type Time = Timestamp;
    type Randomness = randomness_collective_flip::Module<Test>;
    type Call = Call;
    type Event = ();
    type DetermineContractAddress = DummyContractAddressFor;
    type ComputeDispatchFee = DummyComputeDispatchFee;
    type TrieIdGenerator = DummyTrieIdGenerator;
    type GasPayment = ();
    type RentPayment = ();
    type SignedClaimHandicap = contracts::DefaultSignedClaimHandicap;
    type TombstoneDeposit = TombstoneDeposit;
    type StorageSizeOffset = contracts::DefaultStorageSizeOffset;
    type RentByteFee = RentByteFee;
    type RentDepositOffset = RentDepositOffset;
    type SurchargeReward = SurchargeReward;
    type TransactionBaseFee = ContractTransactionBaseFee;
    type TransactionByteFee = ContractTransactionByteFee;
    type ContractFee = ContractFee;
    type CallBaseFee = contracts::DefaultCallBaseFee;
    type InstantiateBaseFee = contracts::DefaultInstantiateBaseFee;
    type MaxDepth = contracts::DefaultMaxDepth;
    type MaxValueSize = contracts::DefaultMaxValueSize;
    type BlockGasLimit = contracts::DefaultBlockGasLimit;
}

impl operator::Trait for Test {
    type Parameters = parameters::StakingParameters;
    type Event = ();
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 10;
    pub const BondingDuration: EraIndex = 3;
}

impl Trait for Test {
    type Currency = Balances;
    type BondingDuration = BondingDuration;
    type ContractFinder = Operator;
    type RewardRemainder = (); // Reward remainder is burned.
    type Reward = (); // Reward is minted.
    type Time = Timestamp;
    type Event = ();
    type SessionsPerEra = SessionsPerEra;
}

/// ValidatorManager module.
pub type System = system::Module<Test>;
pub type Session = session::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Timestamp = timestamp::Module<Test>;
pub type Contract = contracts::Module<Test>;
pub type Operator = operator::Module<Test>;
pub type PlasmStaking = Module<Test>;

/// Generate Wasm binary and code hash from wabt source.
pub fn compile_module<T>(
    wabt_module: &str,
) -> result::Result<(Vec<u8>, <T::Hashing as Hash>::Output), wabt::Error>
where
    T: system::Trait,
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

pub fn valid_instatiate() {
    let (wasm, code_hash) = compile_module::<Test>(CODE_RETURN_FROM_START_FN).unwrap();

    // prepare
    let _ = Balances::deposit_creating(&OPERATOR, 1_000_000);
    assert_ok!(Contract::put_code(Origin::signed(OPERATOR), 100_000, wasm));

    let test_params = parameters::StakingParameters {
        can_be_nominated: true,
        option_expired: 100,
        option_p: Perbill::from_percent(20).deconstruct(),
    };

    // instantiate
    // Check at the end to get hash on error easily
    let _ = Operator::instantiate(
        Origin::signed(OPERATOR),
        100,
        100_000,
        code_hash.into(),
        vec![],
        test_params.clone(),
    );
    // checks deployed contract
    assert!(contracts::ContractInfoOf::<Test>::contains_key(OPERATED_CONTRACT));

    // checks mapping operator and contract
    // OPERATOR operates a only OPERATED_CONTRACT contract.
    assert!(operator::OperatorHasContracts::<Test>::contains_key(OPERATOR));
    let tree = operator::OperatorHasContracts::<Test>::get(&OPERATOR);
    assert_eq!(tree.len(), 1);
    assert!(tree.contains(&OPERATED_CONTRACT));

    // OPERATED_CONTRACT contract is operated by OPERATOR.
    assert!(operator::ContractHasOperator::<Test>::contains_key(
        OPERATED_CONTRACT
    ));
    assert_eq!(
        operator::ContractHasOperator::<Test>::get(&OPERATED_CONTRACT),
        Some(OPERATOR)
    );

    // OPERATED_CONTRACT's contract Parameters is same test_params.
    assert!(operator::ContractParameters::<Test>::contains_key(
        OPERATED_CONTRACT
    ));
    assert_eq!(
        operator::ContractParameters::<Test>::get(&OPERATED_CONTRACT),
        Some(test_params)
    );
}

pub const PER_SESSION: u64 = 60 * 1000;

pub fn advance_session() {
    // increase block numebr
    let now = System::block_number();
    System::set_block_number(now + 1);
    // increase timestamp + 10
    let now_time = Timestamp::get();
    Timestamp::set_timestamp(now_time + PER_SESSION);
    Session::rotate_session();
    assert_eq!(Session::current_index(), (now / Period::get()) as u32);
}

pub fn advance_era() {
    let current_era = PlasmStaking::current_era();
    assert_ok!(PlasmStaking::force_new_era(Origin::ROOT));
    assert_eq!(PlasmStaking::force_era(), Forcing::ForceNew);
    advance_session();
    assert_eq!(PlasmStaking::current_era(), current_era + 1);
}
