#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use rstd::prelude::*;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use support::{
	decl_event, decl_module, decl_storage,
	dispatch::Result,
	traits::{Currency, Get},
};
use system::ensure_signed;

/// The module's configuration trait.
pub trait Trait: balances::Trait + timestamp::Trait {
	// Currency
	type Currency: Currency<Self::AccountId>;

	/// Please waiting minimum period after previous calims.
	type WaitingClaims: Get<Self::Moment>;

	// A faucet can get value.
	type FaucetValue: Get<BalanceOf<Self>>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct FaucetLog<Balance, Moment> {
	pub amount: Balance,
	pub time: Moment,
}

pub type BalanceOf<T> =
<<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

decl_storage! {
    trait Store for Module<T: Trait> as Operator {
        /// A mapping from operators to operated contracts by them.
        pub FaucetHistory: map T::AccountId => Vec<FaucetLog<BalanceOf<T>, T::Moment>>;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// claim
        pub fn claims(origin, ) -> Result {
            let sender = ensure_signed(origin)?;
            let mut history = <FaucetHistory<T>>::get(&sender);
            let now = <timestamp::Module<T>>::now();
            if history.is_empty() || history.last().unwrap().time + T::WaitingClaims::get() < now {
                // can get claims
                history.push(
                    FaucetLog {
                        amount: T::FaucetValue::get(),
                        time: now,
                    });
                <FaucetHistory<T>>::insert(&sender, history);
                // get free balance.
                let free_balance = T::Currency::free_balance(&sender);
                // add balance of sender
                T::Currency::make_free_balance_be(&sender, free_balance + T::FaucetValue::get());
                // issues success events.
                Self::deposit_event(RawEvent::SuccessClaims(sender, T::FaucetValue::get(), now));
                return Ok(())
            }
            // issues failed events.
            Self::deposit_event(RawEvent::FailedClaims(sender, history.last().unwrap().time));
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = BalanceOf<T>,
        Moment = <T as timestamp::Trait>::Moment,
    {
        /// SuccessClaims (sender, amount of faucet, faucet time)
        SuccessClaims(AccountId, Balance, Moment),
        /// FailedClaims (sender, previous faucet time)
        FailedClaims(AccountId, Moment),
    }
);

#[cfg(test)]
mod tests {
	use super::*;

	use primitives::H256;
	use std::cell::RefCell;
	use support::{assert_ok, impl_outer_event, impl_outer_origin, parameter_types};
	use system::{EventRecord, Phase};
	// The testing primitives are very useful for avoiding having to work with signatures
	// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
	use sr_primitives::{
		testing::Header,
		traits::{BlakeTwo256, IdentityLookup},
		Perbill,
	};

	impl_outer_origin! {
        pub enum Origin for Test {}
    }

	mod faucet {
		// Re-export contents of the root. This basically
		// needs to give a name for the current crate.
		// This hack is required for `impl_outer_event!`.
		pub use super::super::*;
	}
	impl_outer_event! {
        pub enum MetaEvent for Test {
            balances<T>, faucet<T>,
        }
    }

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
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
		type Call = ();
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = MetaEvent;
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type AvailableBlockRatio = AvailableBlockRatio;
		type MaximumBlockLength = MaximumBlockLength;
		type Version = ();
	}

	thread_local! {
        static EXISTENTIAL_DEPOSIT: RefCell<u64> = RefCell::new(0);
        static TRANSFER_FEE: RefCell<u64> = RefCell::new(0);
        static CREATION_FEE: RefCell<u64> = RefCell::new(0);
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
        pub const MinimumPeriod: u64 = 5;
    }
	impl timestamp::Trait for Test {
		type Moment = u64;
		type OnTimestampSet = ();
		type MinimumPeriod = MinimumPeriod;
	}

	parameter_types! {
        pub const WaitingClaims: u64 = 200;
        pub const FaucetValue: u64 = 1000;
    }

	impl Trait for Test {
		type Currency = Balances;
		type WaitingClaims = WaitingClaims;
		type FaucetValue = FaucetValue;
		type Event = MetaEvent;
	}

	pub struct ExtBuilder {
		existential_deposit: u64,
		transfer_fee: u64,
		creation_fee: u64,
		monied: bool,
		vesting: bool,
	}

	impl Default for ExtBuilder {
		fn default() -> Self {
			Self {
				existential_deposit: 0,
				transfer_fee: 0,
				creation_fee: 0,
				monied: false,
				vesting: false,
			}
		}
	}

	impl ExtBuilder {
		pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
			self.existential_deposit = existential_deposit;
			self
		}
		#[allow(dead_code)]
		pub fn transfer_fee(mut self, transfer_fee: u64) -> Self {
			self.transfer_fee = transfer_fee;
			self
		}
		pub fn monied(mut self, monied: bool) -> Self {
			self.monied = monied;
			if self.existential_deposit == 0 {
				self.existential_deposit = 1;
			}
			self
		}
		pub fn set_associated_consts(&self) {
			EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
			TRANSFER_FEE.with(|v| *v.borrow_mut() = self.transfer_fee);
			CREATION_FEE.with(|v| *v.borrow_mut() = self.creation_fee);
		}
		pub fn build(self) -> sr_io::TestExternalities {
			self.set_associated_consts();
			let mut t = system::GenesisConfig::default()
				.build_storage::<Test>()
				.unwrap();
			balances::GenesisConfig::<Test> {
				balances: if self.monied {
					vec![
						(1, 10 * self.existential_deposit),
						(2, 20 * self.existential_deposit),
						(3, 30 * self.existential_deposit),
						(4, 40 * self.existential_deposit),
						(12, 10 * self.existential_deposit),
					]
				} else {
					vec![]
				},
				vesting: if self.vesting && self.monied {
					vec![
						(1, 0, 10, 5 * self.existential_deposit),
						(2, 10, 20, 0),
						(12, 10, 20, 5 * self.existential_deposit),
					]
				} else {
					vec![]
				},
			}
				.assimilate_storage(&mut t)
				.unwrap();
			t.into()
		}
	}

	type System = system::Module<Test>;
	type Balances = balances::Module<Test>;
	type Timestamp = timestamp::Module<Test>;
	type Faucet = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sr_io::TestExternalities {
		ExtBuilder::default()
			.existential_deposit(1)
			.monied(true)
			.build()
	}

	#[test]
	fn once_claims() {
		new_test_ext().execute_with(|| {
			assert_eq!(<balances::FreeBalance<Test>>::get(&1), 10);

			assert_ok!(Faucet::claims(Origin::signed(1)));
			assert_eq!(
				System::events(),
				vec![EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: MetaEvent::faucet(faucet::RawEvent::SuccessClaims(
						1,
						<Test as Trait>::FaucetValue::get(),
						Timestamp::now(),
					)),
					topics: vec![],
				}],
			);

			assert_eq!(
				<balances::FreeBalance<Test>>::get(&1),
				10 + <Test as Trait>::FaucetValue::get()
			);
			assert_eq!(
				<FaucetHistory<Test>>::get(&1),
				vec![FaucetLog {
					amount: <Test as Trait>::FaucetValue::get(),
					time: Timestamp::now(),
				}]
			);
		});
	}

	#[test]
	fn success_twice_claims() {
		new_test_ext().execute_with(|| {
			assert_eq!(<balances::FreeBalance<Test>>::get(&1), 10);

			// Success
			assert_ok!(Faucet::claims(Origin::signed(1)));
			assert_eq!(
				System::events(),
				vec![EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: MetaEvent::faucet(faucet::RawEvent::SuccessClaims(
						1,
						<Test as Trait>::FaucetValue::get(),
						Timestamp::now(),
					)),
					topics: vec![],
				}],
			);
			let now = Timestamp::now();
			Timestamp::set_timestamp(<Test as Trait>::WaitingClaims::get() + 1);

			// Failed
			assert_ok!(Faucet::claims(Origin::signed(1)));
			assert_eq!(
				System::events(),
				vec![
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: MetaEvent::faucet(faucet::RawEvent::SuccessClaims(
							1,
							<Test as Trait>::FaucetValue::get(),
							now,
						)),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: MetaEvent::faucet(faucet::RawEvent::SuccessClaims(
							1,
							<Test as Trait>::FaucetValue::get(),
							now + <Test as Trait>::WaitingClaims::get() + 1,
						)),
						topics: vec![],
					}
				],
			);
			assert_eq!(
				<balances::FreeBalance<Test>>::get(&1),
				10 + <Test as Trait>::FaucetValue::get() + <Test as Trait>::FaucetValue::get()
			);
			assert_eq!(
				<FaucetHistory<Test>>::get(&1),
				vec![
					FaucetLog {
						amount: <Test as Trait>::FaucetValue::get(),
						time: now,
					},
					FaucetLog {
						amount: <Test as Trait>::FaucetValue::get(),
						time: now + <Test as Trait>::WaitingClaims::get() + 1,
					}
				]
			);
		});
	}

	#[test]
	fn failed_claims() {
		new_test_ext().execute_with(|| {
			assert_eq!(<balances::FreeBalance<Test>>::get(&1), 10);

			// Success
			assert_ok!(Faucet::claims(Origin::signed(1)));
			assert_eq!(
				System::events(),
				vec![EventRecord {
					phase: Phase::ApplyExtrinsic(0),
					event: MetaEvent::faucet(faucet::RawEvent::SuccessClaims(
						1,
						<Test as Trait>::FaucetValue::get(),
						Timestamp::now(),
					)),
					topics: vec![],
				}],
			);
			let now = Timestamp::now();
			// Failed
			assert_ok!(Faucet::claims(Origin::signed(1)));
			assert_eq!(
				System::events(),
				vec![
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: MetaEvent::faucet(faucet::RawEvent::SuccessClaims(
							1,
							<Test as Trait>::FaucetValue::get(),
							now,
						)),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::ApplyExtrinsic(0),
						event: MetaEvent::faucet(faucet::RawEvent::FailedClaims(1, now)),
						topics: vec![],
					}
				],
			);
			assert_eq!(
				<balances::FreeBalance<Test>>::get(&1),
				10 + <Test as Trait>::FaucetValue::get()
			);
			assert_eq!(
				<FaucetHistory<Test>>::get(&1),
				vec![FaucetLog {
					amount: <Test as Trait>::FaucetValue::get(),
					time: now,
				}]
			);
		});
	}
}
