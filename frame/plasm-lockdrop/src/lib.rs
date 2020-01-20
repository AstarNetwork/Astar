//! Plasm Lockdrop module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use sp_std::{
    prelude::*,
    fmt::Debug,
    collections::btree_map::BTreeMap,
};
use sp_core::offchain::StorageKind;
use sp_runtime::{
    RuntimeDebug,
    traits::{
        Member, CheckEqual, MaybeSerializeDeserialize, Hash,
        MaybeDisplay, SimpleBitOps,
    },
};
use frame_support::{
    decl_module, decl_event, decl_storage, decl_error,
    debug, StorageValue, weights::SimpleDispatchInfo,
};
use frame_system::{
    self as system, ensure_signed,
    offchain::SubmitSignedTransaction
};

/// Plasm Lockdrop Authority local KeyType.
///
/// For security reasons the offchain worker doesn't have direct access to the keys
/// but only to app-specific subkeys, which are defined and grouped by their `KeyTypeId`.
pub const KEY_TYPE: app_crypto::KeyTypeId = app_crypto::KeyTypeId(*b"plas");

/// SR25519 keys support
pub mod sr25519 {
    mod app_sr25519 {
        use app_crypto::{app_crypto, sr25519};
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
        use app_crypto::{app_crypto, ed25519};
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

/// The local storage database key under which the worker progress status
/// is tracked.
const DB_KEY: &[u8] = b"staketechnilogies/plasm-lockdrop-worker";

/// The module's main configuration trait.
pub trait Trait: ::Trait {
	/// The identifier type for an authority.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord;

    /// A dispatchable call type.
    type Call: From<Call<Self>>;

    /// How much votes module should receive to decide claim result.
    type VoteThreshold: Get<ClaimVotes>;

    /// How much positive votes requered to approve claim.
    /// Total positive votes = positive votes - negative votes.
    type PositiveVotes: Get<ClaimVotes>;

    /// Let's define the helper we use to create signed transactions.
    type SubmitTransaction: SubmitSignedTransaction<Self, <Self as Trait>::Call>;

    /// The regular events type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

/// Enumerate up to 2^32 claim requests.
pub type ClaimId = u32;

/// Type for enumerating claim proof votes. 
pub type ClaimVotes = u32;

/// Plasm Lockdrop parameters.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub enum Lockdrop {
    /// Bitcoin lockdrop is pretty simple:
    /// transaction sended with time-lockding opcode,
    /// BTC token locked and could be spend some timestamp.
    /// Duration and value could be derived from BTC transaction.
    Bitcoin { balance: U256, duration: u64, transaction_hash: H256, },
}

/// Lockdrop claim request description.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub struct Claim<T> {
    sender: <T as system::Trait>::AccountId,
    params: Lockdrop,
    vote_up: ClaimVotes,
    vote_down: ClaimVotes,
}

decl_event!(
    pub enum Event<T>
    where <T as system::Trait>::AccountId,
          <T as Trait>::AuthorityId,
    {
        NewRequest(ClaimId, AccountId),
        NewAuthorities(Vec<AuthorityId>),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Provider {
        /// Requests made within this block execution.
        Requests get(fn requests): Vec<ClaimId>;
        /// List of lockdrop authorities keys.
        Keys get(fn keys): Vec<T::AuthorityId>;
        /// Claim requests.
        Claims get(fn claims): map ClaimId => Claim<T>;
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
            <OcRequests<T>>::kill();
        }

        /// Check lock transaction to claim lockdrop tokens. 
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn claim(
            origin,
            params: Lockdrop,
        ) {
            let sender = ensure_signed!(origin);
            let claim = Claim { sender, params, vote_up: 0, vote_down: 0 };

            let claim_id = <LatestClaim>::get();
            <LatestClaim>::push(claim_id + 1);

            <Claims<T>>::insert(claim_id, claim);
            <Requests<T>>::mutate(|requests| requests.push(claim_id));

            Self::deposit_event(RawEvent::NewRequest(claim_id));
        }

        #[weight = SimpleDispatchInfo::FreeOperational]
        fn vote(
            origin,
            claim_id: ClaimId,
            approve: bool,
        ) {
            let sender = ensure_signed!(origin);
            ensure!(T::is_authority(&sender), "Only lockdrop authorities can vote");
            ensure!(<Claims<T>>::exists(claim_id), "Claim with this id doesn't exist");

            <Claims<T>>::mutate(&claim_id, |claim|
                if approve { claim.vote_up += 1 }
                else { claim.vote_down += 1 }
            );
        }

        #[weight = SimpleDispatchInfo::FreeOperational]
        fn set_authorities(
            origin,
            authorities: Vec<T::AuthorityId>,
        ) {
            let _ = ensure_root!(origin);
            <Keys<T>>::put(authorities.clone());
            Self::deposit_event(RawEvent::NewAuthorities(authorities));
        }

        // Runs after every block within the context and current state of said block.
        fn offchain_worker(now: T::BlockNumber) {
            debug::RuntimeLogger::init();
            if sp_io::offchain::is_validator() {
                Self::offchain(now);
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// The main offchain worker entry point.
    fn offchain(now: T::BlockNumber) {
        for claim_id in <Requests>::get() {
            let approve = Self::check_lock(claim_id);
            let call = Call::vote(claim_id, approve);
            T::SubmitTransaction::submit_signed(call);
        }
    }

    fn is_authority(authority: &T::AuthorityId) -> bool {
        <Keys<T>>::get().binary_search(authority).ok()
    }

    fn check_lock(claim_id: ClaimId) -> bool {
        unimplemented!()
    }
}
