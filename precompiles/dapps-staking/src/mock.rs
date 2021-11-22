use super::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{Currency, OnFinalize, OnInitialize, OnUnbalanced},
    PalletId,
};
use pallet_dapps_staking::weights;
use pallet_evm::{AddressMapping, EnsureAddressNever, EnsureAddressRoot, ExitError, PrecompileSet};
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};
use sp_io::TestExternalities;
use sp_runtime::{
    testing::Header,
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
    Perbill,
};
extern crate alloc;

pub(crate) type AccountId = TestAccount;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type EraIndex = u32;
pub(crate) const REWARD_SCALING: u32 = 2;
pub(crate) const MILLIAST: Balance = 1_000_000_000_000_000;
pub(crate) const AST: Balance = 1_000 * MILLIAST;
pub(crate) const TEST_CONTRACT: [u8; 20] = H160::repeat_byte(0x09).to_fixed_bytes();

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const MAX_NUMBER_OF_STAKERS: u32 = 4;
/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const MINIMUM_STAKING_AMOUNT: Balance = 10 * AST;
pub(crate) const DEVELOPER_REWARD_PERCENTAGE: u32 = 80;
pub(crate) const MINIMUM_REMAINING_AMOUNT: Balance = 1;
pub(crate) const HISTORY_DEPTH: u32 = 30;
pub(crate) const MAX_UNLOCKING_CHUNKS: u32 = 4;
pub(crate) const UNBONDING_PERIOD: EraIndex = 3;

// Do note that this needs to at least be 3 for tests to be valid. It can be greater but not smaller.
pub(crate) const BLOCKS_PER_ERA: BlockNumber = 3;

pub(crate) const REGISTER_DEPOSIT: Balance = 10 * AST;

// ignore MILLIAST for easier test handling.
// reward for dapps-staking will be BLOCK_REWARD/2 = 1000
pub(crate) const BLOCK_REWARD: Balance = 1000 * AST;

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Encode,
    Decode,
    Debug,
    MaxEncodedLen,
    Serialize,
    Deserialize,
    derive_more::Display,
    scale_info::TypeInfo,
)]

pub enum TestAccount {
    Empty,
    Alex,
    Bobo,
    Dino,
}

impl Default for TestAccount {
    fn default() -> Self {
        Self::Empty
    }
}

impl AddressMapping<TestAccount> for TestAccount {
    fn into_account_id(h160_account: H160) -> TestAccount {
        match h160_account {
            a if a == H160::repeat_byte(0x11) => Self::Alex,
            a if a == H160::repeat_byte(0x22) => Self::Bobo,
            a if a == H160::repeat_byte(0x33) => Self::Dino,
            _ => Self::Empty,
        }
    }
}

impl TestAccount {
    pub(crate) fn to_h160(&self) -> H160 {
        match self {
            Self::Empty => Default::default(),
            Self::Alex => H160::repeat_byte(0x11),
            Self::Bobo => H160::repeat_byte(0x22),
            Self::Dino => H160::repeat_byte(0x33),
        }
    }
}

impl From<H160> for TestAccount {
    fn from(h160_account: H160) -> TestAccount {
        TestAccount::into_account_id(h160_account)
    }
}

impl TestAccount {
    pub fn to_argument(&self) -> Vec<u8> {
        let mut account_encoded = self.encode();
        let encoded_len = account_encoded.len();
        let mut buffer = vec![0; ARG_SIZE_BYTES - encoded_len];
        buffer.append(&mut account_encoded);
        buffer
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Index = u64;
    type Call = Call;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}
impl pallet_balances::Config for TestRuntime {
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 4];
    type MaxLocks = ();
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

pub fn precompile_address() -> H160 {
    H160::from_low_u64_be(0x5001)
}

#[derive(Debug, Clone, Copy)]
pub struct DappPrecompile<R>(PhantomData<R>);

impl<R> PrecompileSet for DappPrecompile<R>
where
    R::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    R: pallet_dapps_staking::Config + pallet_evm::Config,
    R::Call: From<pallet_dapps_staking::Call<R>>,
{
    fn execute(
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
    ) -> Option<Result<PrecompileOutput, ExitError>> {
        match address {
            a if a == precompile_address() => Some(DappsStakingWrapper::<R>::execute(
                input, target_gas, context,
            )),
            _ => None,
        }
    }
}

pub type Precompiles = DappPrecompile<TestRuntime>;

impl pallet_evm::Config for TestRuntime {
    type FeeCalculator = ();
    type GasWeightMapping = ();
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = AccountId;
    type Currency = Balances;
    type Event = Event;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type Precompiles = DappPrecompile<TestRuntime>;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug, scale_info::TypeInfo)]
pub enum MockSmartContract<AccountId> {
    Evm(sp_core::H160),
    Wasm(AccountId),
}

impl<AccountId> Default for MockSmartContract<AccountId> {
    fn default() -> Self {
        MockSmartContract::Evm(H160::repeat_byte(0x00))
    }
}

impl<AccountId> pallet_dapps_staking::IsContract for MockSmartContract<AccountId> {
    fn is_valid(&self) -> bool {
        match self {
            MockSmartContract::Wasm(_account) => false,
            MockSmartContract::Evm(_account) => true,
        }
    }
}

parameter_types! {
    pub const RegisterDeposit: Balance = REGISTER_DEPOSIT;
    pub const BlockPerEra: BlockNumber = BLOCKS_PER_ERA;
    pub const MaxNumberOfStakersPerContract: u32 = MAX_NUMBER_OF_STAKERS;
    pub const MinimumStakingAmount: Balance = MINIMUM_STAKING_AMOUNT;
    pub const HistoryDepth: u32 = HISTORY_DEPTH;
    pub const DeveloperRewardPercentage: Perbill = Perbill::from_percent(DEVELOPER_REWARD_PERCENTAGE);
    pub const DappsStakingPalletId: PalletId = PalletId(*b"mokdpstk");
    pub const MinimumRemainingAmount: Balance = MINIMUM_REMAINING_AMOUNT;
    pub const BonusEraDuration: u32 = 3;
    pub const MaxUnlockingChunks: u32 = MAX_UNLOCKING_CHUNKS;
    pub const UnbondingPeriod: EraIndex = UNBONDING_PERIOD;
}

impl pallet_dapps_staking::Config for TestRuntime {
    type Event = Event;
    type Currency = Balances;
    type BlockPerEra = BlockPerEra;
    type RegisterDeposit = RegisterDeposit;
    type DeveloperRewardPercentage = DeveloperRewardPercentage;
    type SmartContract = MockSmartContract<AccountId>;
    type WeightInfo = weights::SubstrateWeight<TestRuntime>;
    type MaxNumberOfStakersPerContract = MaxNumberOfStakersPerContract;
    type HistoryDepth = HistoryDepth;
    type BonusEraDuration = BonusEraDuration;
    type MinimumStakingAmount = MinimumStakingAmount;
    type PalletId = DappsStakingPalletId;
    type MinimumRemainingAmount = MinimumRemainingAmount;
    type MaxUnlockingChunks = MaxUnlockingChunks;
    type UnbondingPeriod = UnbondingPeriod;
}

pub struct ExternalityBuilder {
    balances: Vec<(AccountId, Balance)>,
}

impl Default for ExternalityBuilder {
    fn default() -> ExternalityBuilder {
        ExternalityBuilder { balances: vec![] }
    }
}

impl ExternalityBuilder {
    pub fn build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: self.balances,
        }
        .assimilate_storage(&mut storage)
        .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }

    pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
        self.balances = balances;
        self
    }
}

construct_runtime!(
    pub enum TestRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Evm: pallet_evm::{Pallet, Call, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        DappsStaking: pallet_dapps_staking::{Pallet, Call, Storage, Event<T>},
    }
);

/// Used to run to the specified block number
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        DappsStaking::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        // This is performed outside of dapps staking but we expect it before on_initialize
        DappsStaking::on_unbalanced(Balances::issue(BLOCK_REWARD));
        DappsStaking::on_initialize(System::block_number());
    }
}

/// Used to run the specified number of blocks
pub fn run_for_blocks(n: u64) {
    run_to_block(System::block_number() + n);
}

/// Advance blocks to the beginning of an era.
///
/// Function has no effect if era is already passed.
pub fn advance_to_era(n: EraIndex) {
    while DappsStaking::current_era() < n {
        run_for_blocks(1);
    }
}

/// Initialize first block.
/// This method should only be called once in a UT otherwise the first block will get initialized multiple times.
pub fn initialize_first_block() {
    // This assert prevents method misuse
    assert_eq!(System::block_number(), 1 as BlockNumber);

    // We need to beef up the pallet account balance in case of bonus rewards
    let starting_balance = BLOCK_REWARD * BLOCKS_PER_ERA as Balance * REWARD_SCALING as Balance;
    let _ = Balances::deposit_creating(
        &<TestRuntime as pallet_dapps_staking::Config>::PalletId::get().into_account(),
        starting_balance,
    );

    // This is performed outside of dapps staking but we expect it before on_initialize
    DappsStaking::on_unbalanced(Balances::issue(BLOCK_REWARD));
    DappsStaking::on_initialize(System::block_number());
    run_to_block(2);
}

/// default evm context
pub fn default_context() -> evm::Context {
    evm::Context {
        address: Default::default(),
        caller: Default::default(),
        apparent_value: U256::zero(),
    }
}

/// Returns an evm error with provided (static) text.
pub fn exit_error<T: Into<alloc::borrow::Cow<'static, str>>>(text: T) -> ExitError {
    ExitError::Other(text.into())
}

/// returns call struct to be used with evm calls
pub fn evm_call(source: AccountId, input: Vec<u8>) -> pallet_evm::Call<TestRuntime> {
    pallet_evm::Call::call {
        source: source.to_h160(),
        target: precompile_address(),
        input,
        value: U256::zero(),
        gas_limit: u64::max_value(),
        gas_price: U256::zero().into(),
        nonce: None,
    }
}
