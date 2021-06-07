//! Test utilities

#![cfg(test)]

use crate::{self as rewards, *};
use frame_support::{parameter_types};
use sp_core::{crypto::key_types, H256};
use sp_runtime::{
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys},
    KeyTypeId, BuildStorage
};
use crate::mock::sp_api_hidden_includes_construct_runtime::hidden_include::traits::Hooks;

// use traits::{ComputeEraWithParam, MaybeValidators};

pub type BlockNumber = u64;
pub type AccountId = u64;
pub type Balance = u64;
type Block = frame_system::mocking::MockBlock<Test>;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;

pub const ALICE_STASH: u64 = 1;

// impl_outer_origin! {
//     pub enum Origin for Test {}
// }

// impl_outer_dispatch! {
//     pub enum Call for Test where origin: Origin {
//         pallet_session::Session,
//         pallet_balances::Balances,
//         pallet_plasm_staking_rewards::PlasmRewards,
//         // pallet_plasm_node_staking::Staking,
//     }
// }

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Session: pallet_session::{Module, Call, Storage, Config<T>, Event},
        PlasmRewards: rewards::{Module, Call, Event<T>},
	}
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type Origin = Origin;
    type BaseCallFilter = ();
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
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
    type SystemWeightInfo = ();
    type BlockWeights = ();
	type BlockLength = ();
    type SS58Prefix = SS58Prefix;
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
    pub const ExistentialDeposit: Balance = 10;
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

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

pub struct TestSessionHandler;

impl pallet_session::SessionHandler<u64> for TestSessionHandler {
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

impl pallet_session::Config for Test {
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = PlasmRewards;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type Keys = UintAuthorityId;
    type Event = Event;
    type DisabledValidatorsThreshold = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const SessionsPerEra: sp_staking::SessionIndex = 10;
    // pub const BondingDuration: EraIndex = 3;
}

impl Config for Test {
    type Currency = Balances;
    type UnixTime = Timestamp;
    type SessionsPerEra = SessionsPerEra;
    // type ValidatorInterface = Staking;
    type Event = Event;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let _ = pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 1_000_000_000_000_000_000),
            (2, 1_000_000_000_000_000_000),
            (3, 1_000_000_000_000_000_000),
            (100, 1_000_000_000_000_000_000),
        ],
    }
    .assimilate_storage(&mut storage);

    let validators = vec![1, 2, 3, 100];

    let _ = GenesisConfig {
        ..Default::default()
    }
    .assimilate_storage(&mut storage);

    let _ = pallet_session::GenesisConfig::<Test> {
        keys: validators
            .iter()
            .map(|x| (*x, *x, UintAuthorityId(*x)))
            .collect(),
    }
    .assimilate_storage(&mut storage);

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext}


// #[test]
// fn root_calls_fails_for_user() {
//     new_test_ext().execute_with(|| {
//         let res = PlasmRewards::force_no_eras(Origin::signed(0));
//         assert_eq!(
//             res.or_else(|i| Err(i.error)),
//             Err(DispatchError::BadOrigin)
//         );

//         let res = PlasmRewards::force_new_era(Origin::signed(0));
//         assert_eq!(
//             res.or_else(|i| Err(i.error)),
//             Err(DispatchError::BadOrigin)
//         );
        
//         let res = PlasmRewards::force_new_era_always(Origin::signed(0));
//         assert_eq!(
//             res.or_else(|i| Err(i.error)),
//             Err(DispatchError::BadOrigin)
//         );   
//     })
// }

#[test]
fn normal_incremental_era() {
    new_test_ext().execute_with(|| {
        assert_eq!(System::block_number(), 1);
        assert_eq!(PlasmRewards::current_era().unwrap(), 0);
        assert_eq!(
            PlasmRewards::active_era().unwrap(),
            ActiveEraInfo {
                index: 0,
                start: None,
            }
        );
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmRewards::eras_start_session_index(0).unwrap(), 0);
        assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
        assert_eq!(PlasmRewards::for_security_era_reward(0), None);
        assert_eq!(Session::validators(), vec![1, 2, 3, 100]);
        assert_eq!(Session::current_index(), 0);
        
        advance_session();

        assert_eq!(PlasmRewards::current_era().unwrap(), 0);
        assert_eq!(
            PlasmRewards::active_era().unwrap(),
            ActiveEraInfo {
                index: 0,
                start: Some(PER_SESSION),
            }
        );
        assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
        assert_eq!(PlasmRewards::eras_start_session_index(0).unwrap(), 0);
        assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
        assert_eq!(PlasmRewards::for_security_era_reward(0), None);
        assert_eq!(Session::validators(), vec![1, 2, 3, 100]);
        assert_eq!(Session::current_index(), 1);

        // 2~9-th session
        for i in 2..10 {
            advance_session();
            match i {
                9 => assert_eq!(PlasmRewards::current_era().unwrap(), 1),
                _ => assert_eq!(PlasmRewards::current_era().unwrap(), 0),
            }
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo {
                    index: 0,
                    start: Some(PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmRewards::for_dapps_era_reward(0), None);
            assert_eq!(PlasmRewards::for_security_era_reward(0), None);
            assert_eq!(Session::validators(), vec![1, 2, 3, 100]);
            assert_eq!(Session::current_index(), i);
        }
        
        // 10~19-th session
        assert_eq!(PlasmRewards::eras_start_session_index(1).unwrap(), 10);
        for i in 10..20 {
            advance_session();
            match i {
                19 => assert_eq!(PlasmRewards::current_era().unwrap(), 2),
                _ => assert_eq!(PlasmRewards::current_era().unwrap(), 1),
            }
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo {
                    index: 1,
                    start: Some(10 * PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmRewards::eras_start_session_index(1).unwrap(), 10);
            assert_eq!(PlasmRewards::for_security_era_reward(0).unwrap(), 0);
            assert_eq!(PlasmRewards::for_dapps_era_reward(0).unwrap(), 0);
            assert_eq!(Session::current_index(), i);
            assert_eq!(Session::validators(), vec![1, 2, 3, 101]);
        }

        // 20~29-th session
        for i in 20..30 {
            advance_session();
            match i {
                29 => assert_eq!(PlasmRewards::current_era().unwrap(), 3),
                _ => assert_eq!(PlasmRewards::current_era().unwrap(), 2),
            }
            assert_eq!(
                PlasmRewards::active_era().unwrap(),
                ActiveEraInfo {
                    index: 2,
                    start: Some(20 * PER_SESSION),
                }
            );
            assert_eq!(PlasmRewards::force_era(), Forcing::NotForcing);
            assert_eq!(PlasmRewards::eras_start_session_index(2).unwrap(), 20);
            assert_eq!(
                PlasmRewards::for_security_era_reward(1).unwrap(),
                3168333332066
            );
            assert_eq!(PlasmRewards::for_dapps_era_reward(1).unwrap(), 633666667934);
            assert_eq!(Session::current_index(), i);
            assert_eq!(Session::validators(), vec![1, 2, 3, 102]);
        }
    })
}

pub const PER_SESSION: u64 = 60 * 1000;

pub fn advance_session() {
    let next = System::block_number() + 1;
    // increase block numebr
    System::set_block_number(next);
    // increase timestamp + 10
    let now_time = Timestamp::get();
    // on initialize
    Timestamp::set_timestamp(now_time + PER_SESSION);
    Session::rotate_session();
    eprintln!("Period={:?}", Period::get());
    assert_eq!(Session::current_index(), ((next - 1) / Period::get()) as u32);
    // on finalize
    PlasmRewards::on_finalize(next);
    eprintln!("Advanced to block_num={:?}, session={:?}, {:?}",next , Session::current_index(), PlasmRewards::active_era().unwrap());
}
