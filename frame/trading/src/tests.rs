use super::*;
use balances::Reasons;
use sp_runtime::{
    testing::{Header, H256},
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use sp_std::marker::PhantomData;
use support::{
    assert_err, assert_ok, impl_outer_event, impl_outer_origin, parameter_types, StorageMap,
};
use system::{self, EventRecord, Phase};

mod trading {
    // Re-export contents of the root. This basically
    // needs to give a name for the current crate.
    // This hack is required for `impl_outer_event!`.
    pub use super::super::*;
}
impl_outer_event! {
    pub enum MetaEvent for Test {
        system<T>, balances<T>, trading<T>,
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
    type AccountData = balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnReapAccount = Balances;
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 1; // Should be greather than zero
}
impl balances::Trait for Test {
    type Balance = u64;
    type Event = MetaEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = system::Module<Test>;
}
parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}
impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
}

/// define mock operator trait
pub trait MockOperatorTrait: system::Trait {}
decl_storage! {
    trait Store for MockOperatorModule<T: MockOperatorTrait> as MockOperator {
        /// A mapping from operators to operated contracts by them.
        pub OperatorHasContracts: map hasher(blake2_256) T::AccountId => Vec<T::AccountId>;
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
    type Currency = Balances;
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
            balances: vec![(ALICE, 1000), (BOB, 1000), (CHARLIE, 1000), (DAVE, 1000)],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        sp_io::TestExternalities::new(t)
    }
}

type System = system::Module<Test>;
type Balances = balances::Module<Test>;
type Timestamp = timestamp::Module<Test>;
type Trading = Module<Test>;

// operators
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;

// contracts
pub const ANT: u64 = 11;
pub const BLUE: u64 = 12;
pub const CUT: u64 = 13;
pub const DEEN: u64 = 14;

pub const PER_SESSION: u64 = 10;

fn initialize_operator_storage_settings() {
    for (x, y) in vec![
        (ALICE, vec![ANT, BLUE, CUT]),
        (BOB, vec![DEEN]),
        (CHARLIE, vec![]),
        (DAVE, vec![]),
    ] {
        <OperatorHasContracts<Test>>::insert(&x, y);
    }
}

fn advance_session() {
    let now = System::block_number();
    System::initialize(
        &(now + 1),
        &[0u8; 32].into(),
        &[0u8; 32].into(),
        &Default::default(),
        system::InitKind::Full,
    );
    // increase timestamp + 10
    let now_time = Timestamp::get();
    Timestamp::set_timestamp(now_time + PER_SESSION);
}

#[test]
fn correct_offer_test() {
    ExtBuilder::build().execute_with(|| {
        initialize_operator_storage_settings();
        advance_session();
        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 800, 2);
    })
}

#[test]
fn offer_error_test() {
    ExtBuilder::build().execute_with(|| {
        initialize_operator_storage_settings();
        advance_session();

        assert_err!(
            Trading::offer(Origin::signed(CHARLIE), ALICE, vec![CUT, DEEN], 800, 20),
            "sender does not have these contracts.",
        );
        assert_err!(
            Trading::offer(Origin::signed(BOB), ALICE, vec![CUT, DEEN], 1000, 20),
            "buyer does not have enough balances.",
        );

        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 800, 2);

        advance_session();

        assert_err!(
            Trading::offer(Origin::signed(CHARLIE), ALICE, vec![ANT, BLUE], 800, 20),
            "this offer was already issued.",
        );
    })
}

fn correct_offer(
    buyer: AccountId,
    sender: AccountId,
    contracts: Vec<AccountId>,
    amount: u64,
    expired: u64,
) {
    assert_ok!(Trading::offer(
        Origin::signed(buyer),
        sender,
        contracts.clone(),
        amount,
        expired,
    ));
    let offer = OfferOf::<Test> {
        buyer,
        sender,
        contracts,
        amount,
        expired,
        state: OfferState::Waiting,
    };
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: MetaEvent::trading(RawEvent::Offer(buyer, sender)),
            topics: vec![],
        },]
    );
    assert_eq!(Some(offer.clone()), <Offers<Test>>::get(&buyer));
    // Locking buyer test
    assert_eq!(
        vec![balances::BalanceLock {
            id: TRADING_ID,
            amount: amount,
            reasons: Reasons::from(WithdrawReasons::all()),
        }],
        Balances::locks(&buyer)
    )
}

#[test]
fn offer_and_reject_test() {
    ExtBuilder::build().execute_with(|| {
        initialize_operator_storage_settings();

        advance_session();

        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 800, 2);

        advance_session();

        correct_reject(CHARLIE, CHARLIE);

        advance_session();

        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 500, 4);

        advance_session();

        correct_reject(ALICE, CHARLIE);
    })
}

fn correct_reject(rejector: AccountId, offer_id: AccountId) {
    let offer = <Offers<Test>>::get(&offer_id).unwrap();
    let contracts = MockOperatorModule::<Test>::contracts(&offer.sender);
    assert_ok!(Trading::reject(Origin::signed(rejector), offer_id));
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: MetaEvent::trading(RawEvent::Reject(rejector, offer_id.clone())),
            topics: vec![],
        },]
    );
    let mut reject_offer = offer.clone();
    reject_offer.state = OfferState::Reject;
    // equal reject.
    assert_eq!(Some(reject_offer), <Offers<Test>>::get(&offer_id));
    // not changed.
    assert_eq!(MockOperatorModule::<Test>::contracts(&ALICE), contracts);
    // Unlocking buyer test
    assert_eq!(
        Vec::<balances::BalanceLock<u64>>::new(),
        Balances::locks(&offer_id)
    )
}

#[test]
fn reject_error_test() {
    ExtBuilder::build().execute_with(|| {
        initialize_operator_storage_settings();

        advance_session();

        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 800, 5);

        advance_session();

        assert_err!(
            Trading::reject(Origin::signed(CHARLIE), BOB),
            "can not find the offer id."
        );
        assert_err!(
            Trading::reject(Origin::signed(BOB), CHARLIE),
            "the rejector can not reject. only sender or buyer can reject."
        );

        correct_accept(ALICE, CHARLIE);

        assert_err!(
            Trading::reject(Origin::signed(ALICE), CHARLIE),
            "the offer was already accepted."
        );
    })
}

#[test]
fn offer_and_accept_test() {
    ExtBuilder::build().execute_with(|| {
        initialize_operator_storage_settings();

        advance_session();

        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 800, 10);

        advance_session();

        correct_accept(ALICE, CHARLIE);

        advance_session();

        correct_offer(BOB, CHARLIE, vec![ANT], 200, 10);

        advance_session();

        correct_accept(CHARLIE, BOB);
    })
}

fn correct_accept(acceptor: AccountId, offer_id: AccountId) {
    let offer = <Offers<Test>>::get(&offer_id).unwrap();
    let buyer_contracts = MockOperatorModule::<Test>::contracts(&offer.buyer);
    let sender_contracts = MockOperatorModule::<Test>::contracts(&offer.sender);
    let buyer_balances = Balances::free_balance(&offer.buyer);
    let sender_balances = Balances::free_balance(&offer.sender);

    assert_ok!(Trading::accept(Origin::signed(acceptor), offer_id));
    assert_eq!(
        System::events(),
        vec![
            EventRecord {
                phase: Phase::ApplyExtrinsic(0),
                event: MetaEvent::balances(balances::RawEvent::Transfer(
                    offer.buyer,
                    offer.sender,
                    offer.amount,
                )),
                topics: vec![],
            },
            EventRecord {
                phase: Phase::ApplyExtrinsic(0),
                event: MetaEvent::trading(RawEvent::Accept(acceptor, offer_id.clone())),
                topics: vec![],
            },
        ],
    );

    let mut accept_offer = offer.clone();
    accept_offer.state = OfferState::Accept;
    // equal reject.
    assert_eq!(Some(accept_offer), <Offers<Test>>::get(&offer_id));
    // Changed!. sender contracts
    let sender_new_contracts: Vec<AccountId> = sender_contracts
        .iter()
        .filter(|c| !offer.contracts.contains(*c))
        .cloned()
        .collect();
    assert_eq!(
        MockOperatorModule::<Test>::contracts(&offer.sender),
        sender_new_contracts
    );
    // Changed!. buyer contracts.
    let buyer_new_contracts: Vec<AccountId> = buyer_contracts
        .iter()
        .chain(offer.contracts.iter())
        .cloned()
        .collect();
    assert_eq!(
        MockOperatorModule::<Test>::contracts(&offer.buyer),
        buyer_new_contracts
    );
    // Check transfered bueyr -> sende: amount
    assert_eq!(
        buyer_balances - offer.amount,
        Balances::free_balance(&offer.buyer)
    );
    assert_eq!(
        sender_balances + offer.amount,
        Balances::free_balance(&offer.sender)
    );
    // Unlocking buyer test
    assert_eq!(
        Vec::<balances::BalanceLock<u64>>::new(),
        Balances::locks(&offer.buyer)
    )
}

#[test]
fn accept_error_test() {
    ExtBuilder::build().execute_with(|| {
        initialize_operator_storage_settings();

        advance_session();

        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 800, 3);

        advance_session();

        assert_err!(
            Trading::accept(Origin::signed(CHARLIE), BOB),
            "can not find the offer id."
        );
        assert_err!(
            Trading::accept(Origin::signed(BOB), CHARLIE),
            "the accept can not accept. only sender can accept."
        );

        advance_session();

        advance_session();

        assert_err!(
            Trading::accept(Origin::signed(ALICE), CHARLIE),
            "the offer was already expired."
        )
    })
}

#[test]
fn remove_test() {
    ExtBuilder::build().execute_with(|| {
        initialize_operator_storage_settings();

        advance_session();

        correct_offer(CHARLIE, ALICE, vec![ANT, BLUE], 800, 4);

        advance_session();

        assert_err!(
            Trading::remove(Origin::signed(CHARLIE)),
            "the offer is living."
        );
        assert_err!(
            Trading::remove(Origin::signed(ALICE)),
            "the remover does not have a offer."
        );

        advance_session();
        advance_session();

        correct_remove(CHARLIE);
    })
}

fn correct_remove(remover: AccountId) {
    assert_ok!(Trading::remove(Origin::signed(remover)));
    assert_eq!(
        System::events(),
        vec![EventRecord {
            phase: Phase::ApplyExtrinsic(0),
            event: MetaEvent::trading(RawEvent::Remove(remover)),
            topics: vec![],
        },]
    );
    // Unlocking buyer test
    assert_eq!(
        Vec::<balances::BalanceLock<u64>>::new(),
        Balances::locks(&remover)
    )
}
