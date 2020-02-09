//! Plasm Lockdrop module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode, Codec};
use sp_std::prelude::*;
use sp_core::{U256, H256};
use sp_runtime::{
    RuntimeDebug,
    traits::{Member, IdentifyAccount, BlakeTwo256, Hash},
    app_crypto::{KeyTypeId, RuntimeAppPublic},
    offchain::http::Request,
};
use frame_support::{
    decl_module, decl_event, decl_storage, decl_error,
    debug, ensure, StorageValue,
    weights::SimpleDispatchInfo,
    traits::{Get, Currency},
    dispatch::Parameter,
};
use frame_system::{
    self as system, ensure_signed,
    offchain::SubmitSignedTransaction,
};

/// Plasm Lockdrop Authority local KeyType.
///
/// For security reasons the offchain worker doesn't have direct access to the keys
/// but only to app-specific subkeys, which are defined and grouped by their `KeyTypeId`.
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"plaa");

/// SR25519 keys support
pub mod sr25519 {
    mod app_sr25519 {
        use sp_runtime::app_crypto::{app_crypto, sr25519};
        use crate::KEY_TYPE;
        app_crypto!(sr25519, KEY_TYPE);
    }

    /// An authority keypair using sr25519 as its crypto.
    #[cfg(feature = "std")]
    pub type AuthorityPair = app_sr25519::Pair;

    /// An authority signature using sr25519 as its crypto.
    pub type AuthoritySignature = app_sr25519::Signature;

    /// An authority identifier using sr25519 as its crypto.
    pub type AuthorityId = app_sr25519::Public;
}

/// ED25519 keys support
pub mod ed25519 {
    mod app_ed25519 {
        use sp_runtime::app_crypto::{app_crypto, ed25519};
        use crate::KEY_TYPE;
        app_crypto!(ed25519, KEY_TYPE);
    }

    /// An authority keypair using ed25519 as its crypto.
    #[cfg(feature = "std")]
    pub type AuthorityPair = app_ed25519::Pair;

    /// An authority signature using ed25519 as its crypto.
    pub type AuthoritySignature = app_ed25519::Signature;

    /// An authority identifier using ed25519 as its crypto.
    pub type AuthorityId = app_ed25519::Public;
}

// The local storage database key under which the worker progress status is tracked.
//const DB_KEY: &[u8] = b"staketechnilogies/plasm-lockdrop-worker";

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

/// The module's main configuration trait.
pub trait Trait: system::Trait {
    /// The lockdrop balance.
    type Currency: Currency<Self::AccountId>;

    /// How much authority votes module should receive to decide claim result.
    type VoteThreshold: Get<AuthorityVote>;

    /// How much positive votes requered to approve claim.
    ///   Positive votes = approve votes - decline votes.
    type PositiveVotes: Get<AuthorityVote>;

    /// A dispatchable call type.
    type Call: From<Call<Self>>;

    /// Let's define the helper we use to create signed transactions.
    type SubmitTransaction: SubmitSignedTransaction<Self, <Self as Trait>::Call>;

    /// The identifier type for an authority.
    type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord
        + IdentifyAccount<AccountId=<Self as system::Trait>::AccountId>;

    /// The regular events type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

/// Claim id is a hash of claim parameters and sender account id.
pub type ClaimId = H256;

/// Type for enumerating claim proof votes. 
pub type AuthorityVote = u32;

/// Plasm Lockdrop parameters.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum Lockdrop {
    /// Bitcoin lockdrop is pretty simple:
    /// transaction sended with time-lockding opcode,
    /// BTC token locked and could be spend some timestamp.
    /// Duration and value could be derived from BTC transaction.
    Bitcoin { balance: U256, duration: u64, transaction_hash: H256, },
}

impl Default for Lockdrop {
    fn default() -> Self {
        Lockdrop::Bitcoin {
            balance: Default::default(),
            duration: Default::default(),
            transaction_hash: Default::default(),
        }
    }
}

/// Lockdrop claim request description.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug, Clone, Default)]
pub struct Claim<AccountId: Member + Codec> {
    sender: AccountId,
    params: Lockdrop,
    approve: AuthorityVote,
    decline: AuthorityVote,
    complete: bool,
}

decl_event!(
    pub enum Event<T>
    where <T as system::Trait>::AccountId,
          <T as Trait>::AuthorityId,
    {
        /// Lockdrop token claims requested by user
        ClaimRequest(ClaimId),
        /// Lockdrop token claims response by authority
        ClaimResponse(ClaimId, AccountId, bool),
        /// New authority list registered
        NewAuthorities(Vec<AuthorityId>),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Provider {
        /// Offchain lock check requests made within this block execution.
        Requests get(fn requests): Vec<ClaimId>;
        /// List of lockdrop authority id's.
        Keys get(fn keys): Vec<T::AuthorityId>;
        /// Token claim requests.
        Claims get(fn claims): linked_map hasher(blake2_256)
                               ClaimId => Claim<T::AccountId>;
        /// Latest claim index.
        LatestClaim get(fn latest_claim): ClaimId;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Initializing events
        fn deposit_event() = default;

        /// Clean the state on initialisation of a block
        fn on_initialize(_now: T::BlockNumber) {
            // At the beginning of each block execution, system triggers all
            // `on_initialize` functions, which allows us to set up some temporary state or - like
            // in this case - clean up other states
            <Requests>::kill();
        }

        /// Request authorities to check locking transaction. 
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn request(
            origin,
            params: Lockdrop,
        ) {
            let sender = ensure_signed(origin)?;
            let claim_id = BlakeTwo256::hash_of(&(sender.clone(), params.clone()));
            ensure!(!Claims::<T>::get(claim_id).complete, "claim should not be already paid"); 

            if Claims::<T>::exists(claim_id) {
                let claim = Claim { sender, params, .. Default::default() };
                Claims::<T>::insert(claim_id, claim);
            }

            <Requests>::mutate(|requests| requests.push(claim_id));
            Self::deposit_event(RawEvent::ClaimRequest(claim_id));
        }

        /// Claim tokens according to lockdrop procedure.
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn claim(
            origin,
            _claim_id: ClaimId,
        ) {
            let _sender = ensure_signed(origin)?;
            unimplemented!();
        }

        /// Vote for claim request according to check results. (for authorities only) 
        #[weight = SimpleDispatchInfo::FixedOperational(10_000)]
        fn vote(
            origin,
            claim_id: ClaimId,
            approve: bool,
        ) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_authority(&sender), "this method for lockdrop authorities only");
            ensure!(Claims::<T>::exists(claim_id), "request with this id doesn't exist");

            Claims::<T>::mutate(&claim_id, |claim|
                if approve { claim.approve += 1 }
                else { claim.decline += 1 }
            );
            Self::deposit_event(RawEvent::ClaimResponse(claim_id, sender, approve));
        }

        // Runs after every block within the context and current state of said block.
        fn offchain_worker(_now: T::BlockNumber) {
            debug::RuntimeLogger::init();

            if sp_io::offchain::is_validator() {
                match Self::offchain() {
                    Err(e) => debug::error!(
                        target: "lockdrop-offchain-worker",
                        "Lockdrop worker failed with error: {}", e
                    ),
                    _ => (),
                }
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// The main offchain worker entry point.
    fn offchain() -> Result<(), String> {
        // TODO: use permanent storage to track request when temporary failed
        for claim_id in Self::requests() {
            debug::debug!(
                target: "lockdrop-offchain-worker",
                "new claim request: id = {}", claim_id 
            );

            let approve = Self::check_lock(claim_id)?;
            debug::info!(
                target: "lockdrop-offchain-worker",
                "claim id {} => check result: {}", claim_id, approve 
            );

            let call = Call::vote(claim_id, approve);
            debug::debug!(
                target: "lockdrop-offchain-worker",
                "claim id {} => vote extrinsic: {:?}", claim_id, call
            );

            let res = T::SubmitTransaction::submit_signed(call);
            debug::debug!(
                target: "lockdrop-offchain-worker",
                "claim id {} => vote extrinsic send: {:?}", claim_id, res
            );
        }

        Ok(())
    }

    fn is_authority(account: &T::AccountId) -> bool {
        Keys::<T>::get()
            .binary_search_by(|key| key.clone().into_account().cmp(account))
            .is_ok()
    }

    fn check_lock(claim_id: ClaimId) -> Result<bool, String> {
        let Claim { sender, params, .. } = Self::claims(claim_id);
        match params {
            Lockdrop::Bitcoin { balance, duration, transaction_hash } => {
                let uri = format!(
                    // XXX: Fetch transaction description in BTC Testnet using blockcypher API.
                    "http://api.blockcypher.com/v1/btc/test3/txs/{}",
                    hex::encode(transaction_hash)
                );
                let tx = Self::fetch_json(uri)?;
                unimplemented!()
            },
        }
    }

    fn fetch_json(uri: String) -> Result<serde_json::Value, String> {
        let request = Request::get(uri.as_ref()).send()
            .map_err(|e| format!("HTTP request error: {:?}", e))?;
        let response = request.wait()
            .map_err(|e| format!("HTTP response error: {:?}", e))?;
        serde_json::to_value(response.body().clone().collect::<Vec<_>>())
            .map_err(|e| format!("JSON decode error: {}", e))
    }
}

impl<T: Trait> sp_runtime::BoundToRuntimeAppPublic for Module<T> {
    type Public = T::AuthorityId;
}

impl<T: Trait> pallet_session::OneSessionHandler<T::AccountId> for Module<T> {
    type Key = T::AuthorityId;

    fn on_genesis_session<'a, I: 'a>(validators: I)
        where I: Iterator<Item=(&'a T::AccountId, T::AuthorityId)>
    {
        // Init authorities on genesis session.
        let authorities: Vec<_> = validators.map(|x| x.1).collect();
        Keys::<T>::put(authorities.clone());
        Self::deposit_event(RawEvent::NewAuthorities(authorities));
    }

    fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_validators: I)
        where I: Iterator<Item=(&'a T::AccountId, T::AuthorityId)>
    {
        // Remember who the authorities are for the new session.
        let authorities: Vec<_> = validators.map(|x| x.1).collect();
        Keys::<T>::put(authorities.clone());
        Self::deposit_event(RawEvent::NewAuthorities(authorities));
    }

    fn on_before_session_ending() { }
    fn on_disabled(_i: usize) { }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as lockdrop;
    use sp_runtime::{
        Perbill, generic,
        testing::Header,
        traits::{IdentityLookup, BlakeTwo256},
    };
    use frame_support::{
        impl_outer_event,
        impl_outer_origin,
        impl_outer_dispatch,
        parameter_types,
        assert_ok
    };
    use sp_runtime::{traits::{Verify, IdentifyAccount}};
    use sp_core::{
        offchain::{
            OffchainExt, TransactionPoolExt,
            testing::{TestOffchainExt, TestTransactionPoolExt},
        },
        H256, sr25519, crypto::Pair
    };

    impl_outer_event! {
        pub enum MetaEvent for Runtime {
            lockdrop<T>,
            pallet_balances<T>,
        }
    }

    impl_outer_origin!{
        pub enum Origin for Runtime {}
    }

    impl_outer_dispatch! {
        pub enum Call for Runtime where origin: Origin {
            system::System,
            lockdrop::Lockdrop,
        }
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct Runtime;

    // Define the required constants for system module,
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: u32 = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();
    }

    // and add it to our test runtime.
    impl system::Trait for Runtime {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Call = Call;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = MetaEvent;
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
    }

    parameter_types! {
        pub const ExistentialDeposit: u64 = 0;
        pub const TransferFee: u64 = 0;
        pub const CreationFee: u64 = 0;
    }

    impl pallet_balances::Trait for Runtime {
        type Balance = u64;
        type OnNewAccount = ();
        type Event = MetaEvent;
        type DustRemoval = ();
        type TransferPayment = ();
        type ExistentialDeposit = ExistentialDeposit;
        type CreationFee = CreationFee;
        type OnReapAccount = ();
    }

    parameter_types! {
        pub const VoteThreshold: AuthorityVote = 10;
        pub const PositiveVotes: AuthorityVote = 10;
    }

    type SignedExtra = (
        frame_system::CheckEra<Runtime>,
        frame_system::CheckNonce<Runtime>,
        frame_system::CheckWeight<Runtime>,
    );
    type TestXt = sp_runtime::testing::TestXt<Call, SignedExtra>;

    fn extra(nonce: u64) -> SignedExtra {
        (
            frame_system::CheckEra::from(frame_system::Era::Immortal),
            frame_system::CheckNonce::from(nonce),
            frame_system::CheckWeight::new(),
        )
    }

    fn sign_extra(who: u64, nonce: u64) -> Option<(u64, SignedExtra)> {
        Some((who, extra(nonce)))
    }

    impl Trait for Runtime {
        type Event = MetaEvent;
        type Call = Call;
        type SubmitTransaction = frame_system::offchain::TransactionSubmitter<
            lockdrop::sr25519::AuthorityId, Call, TestXt,
        >;
        type VoteThreshold = VoteThreshold;
        type PositiveVotes = PositiveVotes;
        type Currency = Balances;
    }

    type System = frame_system::Module<Runtime>;
    type Balances = pallet_balances::Module<Runtime>;
    type Lockdrop = Module<Runtime>;

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
        t.into()
    }

    #[test]
    fn test_initial_setup() {
        new_test_ext().execute_with(|| {
        })
    }
}
