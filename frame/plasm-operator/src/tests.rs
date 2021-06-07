use crate::{self as operator, *};
use frame_support::{assert_err, assert_ok, parameter_types, traits::Currency, weights::Weight};
use pallet_contracts::weights::WeightInfo;
use sp_core::{H160, U256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, Hash, IdentityLookup},
    AccountId32, Perbill,
};
use std::str::FromStr;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;
type Balance = u128;

const GAS_LIMIT: Weight = 10_000_000_000;
const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2u8; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([3u8; 32]);

frame_support::construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Module, Storage},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        Operator: operator::{Module, Call, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Contracts: pallet_contracts::{Module, Call, Storage, Event<T>, Config<T>},
        EVM: pallet_evm::{Module, Call, Storage, Config, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
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

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Module<Runtime>;
    type WeightInfo = ();
    type MaxLocks = ();
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
            <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
            <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
        )) / 5) as u32;
    pub MaxCodeSize: u32 = 128 * 1024;
}

impl pallet_contracts::Config for Runtime {
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
    pub const ChainId: u64 = 0x1234;
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = ();
    type GasWeightMapping = ();
    type CallOrigin = pallet_evm::EnsureAddressRoot<Self::AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = pallet_evm::HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type Event = Event;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type Precompiles = ();
    type ChainId = ChainId;
}

impl Config for Runtime {
    type Event = Event;
}

fn compile_module<T>(fixture_name: &str) -> wat::Result<(Vec<u8>, <T::Hashing as Hash>::Output)>
where
    T: frame_system::Config,
{
    let fixture_path = ["fixtures/", fixture_name, ".wat"].concat();
    let wasm_binary = wat::parse_file(fixture_path)?;
    let code_hash = T::Hashing::hash(&wasm_binary);
    Ok((wasm_binary, code_hash))
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let mut accounts = sp_std::collections::btree_map::BTreeMap::new();
    accounts.insert(
        H160::from_str("1000000000000000000000000000000000000001").unwrap(),
        pallet_evm::GenesisAccount {
            nonce: U256::from(1),
            balance: U256::from(1000000),
            storage: Default::default(),
            code: vec![
                0x00, // STOP
            ],
        },
    );
    pallet_evm::GenesisConfig { accounts }
        .assimilate_storage::<Runtime>(&mut storage)
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn test_is_contract() {
    new_test_ext().execute_with(|| {
        let bad_contract = SmartContract::Evm(Default::default());
        assert_eq!(Module::<Runtime>::is_contract(&bad_contract), false);

        let bad_contract = SmartContract::Wasm(Default::default());
        assert_eq!(Module::<Runtime>::is_contract(&bad_contract), false);

        let ok_contract =
            SmartContract::Evm(H160::from_str("1000000000000000000000000000000000000001").unwrap());
        assert_eq!(Module::<Runtime>::is_contract(&ok_contract), true);

        let _ = Balances::deposit_creating(&ALICE, 1_000_000_000_000_000_000);
        let subsistence = pallet_contracts::Module::<Runtime>::subsistence_threshold();
        let (wasm, code_hash) = compile_module::<Runtime>("return_from_start_fn").unwrap();
        assert_ok!(Contracts::instantiate_with_code(
            Origin::signed(ALICE),
            subsistence * 100,
            GAS_LIMIT,
            wasm,
            vec![],
            vec![],
        ));
        let addr = Contracts::contract_address(&ALICE, &code_hash, &[]);
        let ok_contract = SmartContract::Wasm(addr);
        assert_eq!(Module::<Runtime>::is_contract(&ok_contract), true);
    })
}

#[test]
fn test_claim_contract() {
    new_test_ext().execute_with(|| {
        let bad_contract = SmartContract::Evm(Default::default());
        assert_err!(
            Module::<Runtime>::claim_contract(Origin::signed(ALICE), bad_contract),
            Error::<Runtime>::NotContract,
        );

        let evm_address = H160::from_str("1000000000000000000000000000000000000001").unwrap();
        let ok_contract = SmartContract::Evm(evm_address);

        //claim evm contract
        assert_ok!(Module::<Runtime>::claim_contract(
            Origin::signed(ALICE),
            ok_contract.clone()
        ));

        // verify event for new evm contract
        assert_eq!(
            last_event(),
            Event::operator(crate::Event::ContractClaimed(
                ALICE,
                pallet::SmartContract::Evm(evm_address)
            )),
        );

        // double claim contract - should return error ContractHasOperator
        assert_err!(
            Module::<Runtime>::claim_contract(Origin::signed(ALICE), ok_contract.clone()),
            Error::<Runtime>::ContractHasOperator,
        );

        // claim already claimed contract - should send error
        assert_err!(
            Module::<Runtime>::claim_contract(Origin::signed(BOB), ok_contract),
            Error::<Runtime>::ContractHasOperator,
        );

        // create wasm contract
        let _ = Balances::deposit_creating(&ALICE, 1_000_000_000_000_000_000);
        let subsistence = pallet_contracts::Module::<Runtime>::subsistence_threshold();
        let (wasm, code_hash) = compile_module::<Runtime>("return_from_start_fn").unwrap();
        assert_ok!(Contracts::instantiate_with_code(
            Origin::signed(ALICE),
            subsistence * 100,
            GAS_LIMIT,
            wasm,
            vec![],
            vec![],
        ));
        let wasm_addr = Contracts::contract_address(&ALICE, &code_hash, &[]);
        let ok_contract = SmartContract::Wasm(wasm_addr.clone());

        // assign new contract for operator - should return error OperatorHasContract
        assert_err!(
            Module::<Runtime>::claim_contract(Origin::signed(ALICE), ok_contract.clone()),
            Error::<Runtime>::OperatorHasContract,
        );

        // claim wasm contract
        assert_ok!(Module::<Runtime>::claim_contract(
            Origin::signed(BOB),
            ok_contract.clone()
        ));

        // verify event for new wasm contract
        assert_eq!(
            last_event(),
            Event::operator(crate::Event::ContractClaimed(
                BOB,
                pallet::SmartContract::Wasm(wasm_addr)
            )),
        );

        // double claim contract - should return error ContractHasOperator
        assert_err!(
            Module::<Runtime>::claim_contract(Origin::signed(BOB), ok_contract.clone()),
            Error::<Runtime>::ContractHasOperator,
        );

        // claim already claimed contract - should fail
        assert_err!(
            Module::<Runtime>::claim_contract(Origin::signed(CHARLIE), ok_contract),
            Error::<Runtime>::ContractHasOperator,
        );
    })
}

fn last_event() -> Event {
    frame_system::Module::<Runtime>::events()
        .pop()
        .expect("Event expected")
        .event
}
