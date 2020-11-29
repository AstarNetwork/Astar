#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::{
    decl_error, decl_event, decl_module,
    dispatch::DispatchResultWithPostInfo,
    traits::{Get, UnfilteredDispatchable},
    weights::{GetDispatchInfo, Pays},
    Parameter,
};
use frame_system::ensure_none;
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
        ValidTransaction,
    },
    DispatchResult,
};
use sp_std::prelude::*;

/// Ethereum-compatible signatures (eth_sign API call).
pub mod ethereum;

/// The module's configuration trait.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// A signable call.
    type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;

    /// User defined signature type.
    type Signature: Parameter + Verify<Signer = Self::Signer>;

    /// User defined signer type.
    type Signer: IdentifyAccount<AccountId = Self::AccountId>;

    /// A configuration for base priority of unsigned transactions.
    ///
    /// This is exposed so that it can be tuned for particular runtime, when
    /// multiple pallets send unsigned transactions.
    type UnsignedPriority: Get<TransactionPriority>;
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Provided invalid signature data.
        InvalidSignature,
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
    {
        /// A call just executed. \[result\]
        Executed(AccountId, DispatchResult),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
        fn call(
            origin,
            call: Box<<T as Trait>::Call>,
            account: T::AccountId,
            signature: <T as Trait>::Signature,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            if signature.verify(&call.encode()[..], &account) {
                let new_origin = frame_system::RawOrigin::Signed(account.clone()).into();
                let res = call.dispatch_bypass_filter(new_origin).map(|_| ());
                Self::deposit_event(RawEvent::Executed(account, res.map_err(|e| e.error)));
                Ok(Pays::No.into())
            } else {
                Err(Error::<T>::InvalidSignature)?
            }
        }
    }
}

impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        if let Call::call(call, signer, signature) = call {
            if !signature.verify(call.encode().as_ref(), &signer) {
                return InvalidTransaction::BadProof.into();
            }

            ValidTransaction::with_tag_prefix("CustomSignatures")
                .priority(T::UnsignedPriority::get())
                .and_provides((call, signer))
                .longevity(64_u64)
                .propagate(true)
                .build()
        } else {
            InvalidTransaction::Call.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as custom_signatures;
    use custom_signatures::*;
    use frame_support::{
        assert_err, assert_ok, impl_outer_dispatch, impl_outer_event, impl_outer_origin,
        parameter_types,
    };
    use hex_literal::hex;
    use sp_core::{ecdsa, Pair};
    use sp_io::hashing::keccak_256;
    use sp_keyring::AccountKeyring as Keyring;
    use sp_runtime::{
        testing::{Header, H256},
        traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
        transaction_validity::TransactionPriority,
        MultiSignature, MultiSigner, Perbill,
    };

    pub const ECDSA_SEED: [u8; 32] =
        hex_literal::hex!["7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"];

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;

    type Balance = u128;
    type BlockNumber = u64;
    type Signature = MultiSignature;
    type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

    impl_outer_origin! {
        pub enum Origin for Runtime {}
    }

    impl_outer_dispatch! {
        pub enum Call for Runtime where origin: Origin {
            pallet_balances::Balances,
        }
    }

    impl_outer_event! {
        pub enum Event for Runtime {
            frame_system<T>,
            pallet_balances<T>,
            custom_signatures<T>,
        }
    }

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: u32 = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();
    }

    impl frame_system::Trait for Runtime {
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
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type PalletInfo = ();
        type AccountData = pallet_balances::AccountData<Balance>;
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type DbWeight = ();
        type BlockExecutionWeight = ();
        type ExtrinsicBaseWeight = ();
        type MaximumExtrinsicWeight = ();
        type SystemWeightInfo = ();
    }

    parameter_types! {
        pub const ExistentialDeposit: Balance = 1;
    }

    impl pallet_balances::Trait for Runtime {
        type Balance = Balance;
        type Event = Event;
        type DustRemoval = ();
        type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = frame_system::Module<Runtime>;
        type WeightInfo = ();
        type MaxLocks = ();
    }

    parameter_types! {
        pub const Priority: TransactionPriority = TransactionPriority::max_value();
    }

    impl Trait for Runtime {
        type Event = Event;
        type Call = Call;
        type Signature = ethereum::EthereumSignature;
        type Signer = <Signature as Verify>::Signer;
        type UnsignedPriority = Priority;
    }

    type System = frame_system::Module<Runtime>;
    type Balances = pallet_balances::Module<Runtime>;
    type CustomSignatures = custom_signatures::Module<Runtime>;

    fn new_test_ext() -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        let pair = ecdsa::Pair::from_seed(&ECDSA_SEED);
        let account = MultiSigner::from(pair.public()).into_account();
        let _ = pallet_balances::GenesisConfig::<Runtime> {
            balances: vec![(account, 1_000_000_000_000_000_000)],
        }
        .assimilate_storage(&mut storage);
        storage.into()
    }

    fn sign_call(call: &Call) -> ecdsa::Signature {
        let call_msg = ethereum::signable_message(&call.encode());
        let ecdsa_msg = secp256k1::Message::parse(&keccak_256(&call_msg));
        let secret = secp256k1::SecretKey::parse(&ECDSA_SEED).expect("valid seed");
        secp256k1::sign(&ecdsa_msg, &secret).into()
    }

    #[test]
    fn invalid_signature() {
        let bob: <Runtime as frame_system::Trait>::AccountId = Keyring::Bob.into();
        let alice: <Runtime as frame_system::Trait>::AccountId = Keyring::Alice.into();
        let call = pallet_balances::Call::<Runtime>::transfer(alice.clone(), 1_000).into();
        let signature = ethereum::EthereumSignature(hex!["dd0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"]);
        assert_err!(
            CustomSignatures::call(Origin::none(), Box::new(call), bob, signature),
            Error::<Runtime>::InvalidSignature,
        );
    }

    #[test]
    fn balance_transfer() {
        new_test_ext().execute_with(|| {
            let pair = ecdsa::Pair::from_seed(&ECDSA_SEED);
            let account = MultiSigner::from(pair.public()).into_account();

            let alice: <Runtime as frame_system::Trait>::AccountId = Keyring::Alice.into();
            assert_eq!(System::account(alice.clone()).data.free, 0);

            let call = pallet_balances::Call::<Runtime>::transfer(alice.clone(), 1_000).into();
            let signature = sign_call(&call).into();

            assert_ok!(CustomSignatures::call(
                Origin::none(),
                Box::new(call),
                account,
                signature
            ));
            assert_eq!(System::account(alice).data.free, 1_000);
        })
    }
}
