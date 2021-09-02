use crate as custom_signatures;
use codec::Encode;
use custom_signatures::*;
use frame_support::{assert_err, assert_ok, parameter_types};
use hex_literal::hex;
use sp_core::{ecdsa, Pair};
use sp_io::hashing::keccak_256;
use sp_keyring::AccountKeyring as Keyring;
use sp_runtime::{
    testing::{Header, H256},
    traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
    transaction_validity::TransactionPriority,
    MultiSignature, MultiSigner,
};

pub const ECDSA_SEED: [u8; 32] =
    hex_literal::hex!["7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"];

type Balance = u128;
type BlockNumber = u64;
type Signature = MultiSignature;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
type Block = frame_system::mocking::MockBlock<Runtime>;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;

frame_support::construct_runtime!(
    pub enum Runtime where
       Block = Block,
       NodeBlock = Block,
       UncheckedExtrinsic = UncheckedExtrinsic,
    {
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        CustomSignatures: custom_signatures::{Pallet, Call, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type Origin = Origin;
    type BaseCallFilter = ();
    type Index = u32;
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
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

parameter_types! {
    pub const Priority: TransactionPriority = TransactionPriority::max_value();
    pub const CallFee: Balance = 42;
    pub const CallMagicNumber: u16 = 0xff50;
}

impl Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Signature = ethereum::EthereumSignature;
    type Signer = <Signature as Verify>::Signer;
    type CallMagicNumber = CallMagicNumber;
    type Currency = Balances;
    type CallFee = CallFee;
    type OnChargeTransaction = ();
    type UnsignedPriority = Priority;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    let pair = ecdsa::Pair::from_seed(&ECDSA_SEED);
    let account = MultiSigner::from(pair.public()).into_account();
    let _ = pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(account, 1_000_000_000)],
    }
    .assimilate_storage(&mut storage);
    storage.into()
}

/// Simple `eth_sign` implementation, should be equal to exported by RPC
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
    new_test_ext().execute_with(|| {
        assert_err!(
            CustomSignatures::call(Origin::none(), Box::new(call), bob, signature, 0),
            Error::<Runtime>::InvalidSignature,
        );
    });
}

#[test]
fn balance_transfer() {
    new_test_ext().execute_with(|| {
        let pair = ecdsa::Pair::from_seed(&ECDSA_SEED);
        let account = MultiSigner::from(pair.public()).into_account();

        let alice: <Runtime as frame_system::Config>::AccountId = Keyring::Alice.into();
        assert_eq!(System::account(alice.clone()).data.free, 0);

        let call: Call = pallet_balances::Call::<Runtime>::transfer(alice.clone(), 1_000).into();
        let payload = (0xff50u16, 0u32, call.clone());
        let signature = eth_sign(&ECDSA_SEED, payload.encode().as_ref()).into();

        assert_eq!(System::account(account.clone()).nonce, 0);
        assert_ok!(CustomSignatures::call(
            Origin::none(),
            Box::new(call.clone()),
            account.clone(),
            signature,
            0,
        ));
        assert_eq!(System::account(alice.clone()).data.free, 1_000);
        assert_eq!(System::account(account.clone()).nonce, 1);
        assert_eq!(System::account(account.clone()).data.free, 999_998_958);

        let signature = eth_sign(&ECDSA_SEED, payload.encode().as_ref()).into();
        assert_err!(
            CustomSignatures::call(
                Origin::none(),
                Box::new(call.clone()),
                account.clone(),
                signature,
                0,
            ),
            Error::<Runtime>::BadNonce,
        );

        let payload = (0xff50u16, 1u32, call.clone());
        let signature = eth_sign(&ECDSA_SEED, payload.encode().as_ref()).into();
        assert_eq!(System::account(account.clone()).nonce, 1);
        assert_ok!(CustomSignatures::call(
            Origin::none(),
            Box::new(call.clone()),
            account.clone(),
            signature,
            1,
        ));
        assert_eq!(System::account(alice).data.free, 2_000);
        assert_eq!(System::account(account.clone()).nonce, 2);
        assert_eq!(System::account(account.clone()).data.free, 999_997_916);
    })
}

/* TODO: enable it when UI fixtures will be ready
#[test]
fn call_fixtures() {
    use sp_core::crypto::Ss58Codec;

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
    let payload = (0xff50u16, 0u32, call.clone());
    assert_eq!(eth_sign(&seed, payload.encode().as_ref()), signature)
}
*/
