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
use sp_std::convert::TryFrom;
use sp_std::prelude::*;

/// Ethereum-compatible signatures (eth_sign API call).
pub mod ethereum;

/// The module's configuration trait.
pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// A signable call.
    type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;

    /// User defined signature type.
    type Signature: Parameter + Verify<Signer = Self::Signer> + TryFrom<Vec<u8>>;

    /// User defined signer type.
    type Signer: IdentifyAccount<AccountId = Self::AccountId>;

    /// A configuration for base priority of unsigned transactions.
    ///
    /// This is exposed so that it can be tuned for particular runtime, when
    /// multiple pallets send unsigned transactions.
    type UnsignedPriority: Get<TransactionPriority>;
}

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Signature decode fails.
        DecodeFailure,
        /// Signature and account mismatched.
        InvalidSignature,
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
    {
        /// A call just executed. \[result\]
        Executed(AccountId, DispatchResult),
    }
);

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
        fn call(
            origin,
            call: Box<<T as Config>::Call>,
            account: T::AccountId,
            signature: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            let signature = <T as Config>::Signature::try_from(signature)
                .map_err(|_| Error::<T>::DecodeFailure)?;
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

const SIGNATURE_DECODE_FAILURE: u8 = 1;

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        if let Call::call(call, signer, signature) = call {
            frame_support::runtime_print!("CALL: {:?}", call.encode());
            frame_support::runtime_print!("SIGNATURE: {:?}", signature);

            if let Ok(signature) = <T as Config>::Signature::try_from(signature.clone()) {
                if signature.verify(&call.encode()[..], &signer) {
                    return ValidTransaction::with_tag_prefix("CustomSignatures")
                        .priority(T::UnsignedPriority::get())
                        .and_provides((call, signer))
                        .longevity(64_u64)
                        .propagate(true)
                        .build();
                } else {
                    InvalidTransaction::BadProof.into()
                }
            } else {
                InvalidTransaction::Custom(SIGNATURE_DECODE_FAILURE).into()
            }
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
    use sp_core::{crypto::Ss58Codec, ecdsa, Pair};
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

    impl frame_system::Config for Runtime {
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
        pub const Priority: TransactionPriority = TransactionPriority::max_value();
    }

    impl Config for Runtime {
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

    // Simple `eth_sign` implementation, should be equal to exported by RPC
    fn eth_sign(seed: &[u8; 32], data: &[u8]) -> Vec<u8> {
        let call_msg = ethereum::signable_message(data);
        let ecdsa_msg = secp256k1::Message::parse(&keccak_256(&call_msg));
        let secret = secp256k1::SecretKey::parse(&seed).expect("valid seed");
        let mut ecdsa: ecdsa::Signature = secp256k1::sign(&ecdsa_msg, &secret).into();
        // Fix recovery ID: Ethereum uses 27/28 notation
        ecdsa.as_mut()[64] += 27;
        Vec::from(ecdsa.as_ref() as &[u8])
    }

    #[test]
    fn eth_sign_works() {
        let seed = hex!["7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"];
        let text = b"Hello Plasm";
        let signature = hex!["79eec99d7f5b321c1b75d2fc044b555f9afdbc4f9b43a011085f575b216f85c452a04373d487671852dca4be4fe5fd90836560afe709d1dab45ab18bc936c2111c"];
        assert_eq!(eth_sign(&seed, &text[..]), signature);
    }

    #[test]
    fn invalid_signature() {
        let bob: <Runtime as frame_system::Config>::AccountId = Keyring::Bob.into();
        let alice: <Runtime as frame_system::Config>::AccountId = Keyring::Alice.into();
        let call = pallet_balances::Call::<Runtime>::transfer(alice.clone(), 1_000).into();
        let signature = Vec::from(&hex!["dd0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"][..]);
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

            let alice: <Runtime as frame_system::Config>::AccountId = Keyring::Alice.into();
            assert_eq!(System::account(alice.clone()).data.free, 0);

            let call: Call =
                pallet_balances::Call::<Runtime>::transfer(alice.clone(), 1_000).into();
            let signature = eth_sign(&ECDSA_SEED, call.encode().as_ref()).into();

            assert_ok!(CustomSignatures::call(
                Origin::none(),
                Box::new(call),
                account,
                signature
            ));
            assert_eq!(System::account(alice).data.free, 1_000);
        })
    }

    #[test]
    fn call_fixtures() {
        let seed = hex!["7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"];
        let pair = ecdsa::Pair::from_seed(&seed);
        assert_eq!(
            MultiSigner::from(pair.public())
                .into_account()
                .to_ss58check(),
            "5Geeci7qCoYHyg9z2AwfpiT4CDryvxYyD7SAUdfNBz9CyDSb",
        );

        let dest =
            AccountId::from_ss58check("5GVwcV6EzxxYbXBm7H6dtxc9TCgL4oepMXtgqWYEc3VXJoaf").unwrap();
        let call: Call = pallet_balances::Call::<Runtime>::transfer(dest, 1000).into();
        assert_eq!(
            call.encode(),
            hex!["0000c4305fb88b6ccb43d6552dc11d18e7b0ee3185247adcc6e885eb284adf6c563da10f"],
        );

        let signature = hex!["96cd8087ef720b0ec10d96996a8bbb45005ba3320d1dde38450a56f77dfd149720cc2e6dcc8f09963aad4cdf5ec15e103ce56d0f4c7a753840217ef1787467a01c"];
        assert_eq!(eth_sign(&seed, call.encode().as_ref()), signature)
    }
}
