//! # Plasm Lockdrop Module
//! This module held lockdrop event on live network.
//!
//! - [`plasm_lockdrop::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::Parameter,
    ensure,
    storage::IterableStorageMap,
    traits::{Currency, Get, Time},
    weights::Weight,
    StorageMap, StorageValue,
};
use frame_system::{
    self as system, ensure_none, ensure_root,
    offchain::{SendTransactionTypes, SubmitTransaction},
};
pub use generic_array::typenum;
use median::{Filter, ListNode};
use sp_core::{ecdsa, H256};
use sp_runtime::{
    app_crypto::RuntimeAppPublic,
    traits::{AtLeast32Bit, BlakeTwo256, Hash, IdentifyAccount, Member, Saturating},
    transaction_validity::{
        InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
        ValidTransaction,
    },
    DispatchResult, Perbill, RuntimeDebug,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::prelude::*;

/// Authority keys.
mod crypto;
/// Oracle client.
mod oracle;

pub use crypto::*;
pub use oracle::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

/// The module's main configuration trait.
pub trait Trait: SendTransactionTypes<Call<Self>> + frame_system::Trait {
    /// The lockdrop balance.
    type Currency: Currency<Self::AccountId>;

    /// Lock duration bonuses.
    type DurationBonus: DurationBonus;

    /// How long dollar rate parameters valid in secs.
    type MedianFilterExpire: Get<Self::Moment>;

    /// Median filter window size.
    type MedianFilterWidth: generic_array::ArrayLength<ListNode<Self::DollarRate>>;

    /// The identifier type for an authority.
    type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord;

    /// System level account type.
    /// This used for resolving account ID's of ECDSA lockdrop public keys.
    type Account: IdentifyAccount<AccountId = Self::AccountId> + From<ecdsa::Public>;

    /// Module that could provide timestamp.
    type Time: Time<Moment = Self::Moment>;

    /// Timestamp type.
    type Moment: Member
        + Parameter
        + Saturating
        + AtLeast32Bit
        + Copy
        + Default
        + From<u64>
        + Into<u64>
        + Into<u128>;

    /// Dollar rate number data type.
    type DollarRate: Member
        + Parameter
        + AtLeast32Bit
        + num_traits::sign::Unsigned
        + Copy
        + Default
        + Into<u128>
        + From<u64>
        + sp_std::str::FromStr;

    // XXX: I don't known how to convert into Balance from u128 without it
    // TODO: Should be removed
    type BalanceConvert: From<u128>
        + Into<<Self::Currency as Currency<<Self as frame_system::Trait>::AccountId>>::Balance>;

    /// The regular events type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Base priority for unsigned transactions.
    type UnsignedPriority: Get<TransactionPriority>;
}

/// Lock duration bonuses,
/// in principle when you lock for long time you'll get more lockdrop tokens.
pub trait DurationBonus {
    /// Lockdrop bonus depends of lockding duration (in secs).
    fn bonus(duration: u64) -> u16;
}

pub struct DustyDurationBonus;
impl DurationBonus for DustyDurationBonus {
    fn bonus(duration: u64) -> u16 {
        const DAYS: u64 = 24 * 60 * 60; // One day in seconds
        if duration < 3 * DAYS {
            0 // Dont permit to participate with locking less
        } else if duration < 10 * DAYS {
            24
        } else if duration < 30 * DAYS {
            100
        } else if duration < 100 * DAYS {
            360
        } else {
            1600
        }
    }
}

/// Claim id is a hash of claim parameters.
pub type ClaimId = H256;

/// Type for enumerating claim proof votes.
pub type AuthorityVote = u32;

/// Type for enumerating authorities in list (2^16 authorities is enough).
pub type AuthorityIndex = u16;

/// Plasm Lockdrop parameters.
#[cfg_attr(feature = "std", derive(Eq))]
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Clone)]
pub enum Lockdrop {
    /// Bitcoin lockdrop is pretty simple:
    /// transaction sended with time-lockding opcode,
    /// BTC token locked and could be spend some timestamp.
    /// Duration in blocks and value in shatoshi could be derived from BTC transaction.
    Bitcoin {
        transaction_hash: H256,
        public_key: ecdsa::Public,
        duration: u64,
        value: u128,
    },
    /// Ethereum lockdrop transactions sent to pre-deployed smart contract.
    Ethereum {
        transaction_hash: H256,
        public_key: ecdsa::Public,
        duration: u64,
        value: u128,
    },
}

impl Default for Lockdrop {
    fn default() -> Self {
        Lockdrop::Bitcoin {
            public_key: Default::default(),
            value: Default::default(),
            duration: Default::default(),
            transaction_hash: Default::default(),
        }
    }
}

/// Lockdrop claim request description.
#[cfg_attr(feature = "std", derive(Eq))]
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Default, Clone)]
pub struct Claim<AuthorityId: Ord> {
    params: Lockdrop,
    approve: BTreeSet<AuthorityId>,
    decline: BTreeSet<AuthorityId>,
    amount: u128,
    complete: bool,
}

/// Lockdrop claim vote.
#[cfg_attr(feature = "std", derive(Eq))]
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Clone)]
pub struct ClaimVote {
    claim_id: ClaimId,
    approve: bool,
    authority: AuthorityIndex,
}

/// Oracle dollar rate ticker.
#[cfg_attr(feature = "std", derive(Eq))]
#[derive(Encode, Decode, RuntimeDebug, PartialEq, Clone)]
pub struct TickerRate<DollarRate: Member + Parameter> {
    authority: AuthorityIndex,
    btc: DollarRate,
    eth: DollarRate,
}

decl_event!(
    pub enum Event<T>
    where <T as system::Trait>::AccountId,
          <T as Trait>::AuthorityId,
          <T as Trait>::DollarRate,
          Balance = BalanceOf<T>,
    {
        /// Lockdrop token claims requested by user
        ClaimRequest(ClaimId),
        /// Lockdrop token claims response by authority
        ClaimResponse(ClaimId, AuthorityId, bool),
        /// Lockdrop token claim paid
        ClaimComplete(ClaimId, AccountId, Balance),
        /// Dollar rate updated by oracle: BTC, ETH.
        NewDollarRate(DollarRate, DollarRate),
        /// New authority list registered
        NewAuthorities(Vec<AuthorityId>),
    }
);

pub const ERROR_ALREADY_CLAIMED: u8 = 1;
pub const ERROR_WRONG_POW_PROOF: u8 = 2;
pub const ERROR_CLAIM_ON_VOTING: u8 = 3;
pub const ERROR_UNKNOWN_AUTHORITY: u8 = 4;

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Unknown authority index in voting message.
        UnknownAuthority,
        /// This claim already paid to requester.
        AlreadyPaid,
        /// Votes for this claim isn't enough to pay it.
        NotEnoughVotes,
        /// Authorities reject this claim request.
        NotApproved,
        /// Lockdrop isn't run now, request could not be processed.
        OutOfBounds,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Provider {
        /// Offchain lock check requests made within this block execution.
        Requests get(fn requests): Vec<ClaimId>;
        /// List of lockdrop authority id's.
        Keys get(fn keys) config(): Vec<T::AuthorityId>;
        /// Token claim requests.
        Claims get(fn claims):
            map hasher(blake2_128_concat) ClaimId
            => Claim<T::AuthorityId>;
        /// Double vote prevention registry.
        HasVote get(fn has_vote):
            double_map hasher(blake2_128_concat) T::AuthorityId, hasher(blake2_128_concat) ClaimId
            => bool;
        /// Lockdrop alpha parameter, where α ∈ [0; 1]
        Alpha get(fn alpha) config(): Perbill;
        /// Lockdrop dollar rate parameter: BTC, ETH.
        DollarRate get(fn dollar_rate) config(): (T::DollarRate, T::DollarRate);
        /// Lockdrop dollar rate median filter table: Time, BTC, ETH.
        DollarRateF get(fn dollar_rate_f):
            map hasher(blake2_128_concat) T::AuthorityId
            => (T::Moment, T::DollarRate, T::DollarRate);
        /// How much authority votes module should receive to decide claim result.
        VoteThreshold get(fn vote_threshold) config(): AuthorityVote;
        /// How much positive votes requered to approve claim.
        ///   Positive votes = approve votes - decline votes.
        PositiveVotes get(fn positive_votes) config(): AuthorityVote;
        /// Timestamp bounds of lockdrop held period.
        LockdropBounds get(fn lockdrop_bounds) config(): (T::BlockNumber, T::BlockNumber);
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        /// Initializing events
        fn deposit_event() = default;

        /// Clean the state on initialisation of a block
        fn on_initialize(_now: T::BlockNumber) -> Weight {
            // At the beginning of each block execution, system triggers all
            // `on_initialize` functions, which allows us to set up some temporary state or - like
            // in this case - clean up other states
            <Requests>::kill();
            // TODO: weight
            50_000
        }

        /// Request authorities to check locking transaction.
        /// TODO: weight
        #[weight = 50_000]
        fn request(
            origin,
            params: Lockdrop,
            _nonce: H256,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let claim_id = BlakeTwo256::hash_of(&params);
            ensure!(
                !<Claims<T>>::get(claim_id).complete,
                Error::<T>::AlreadyPaid,
            );

            if !<Claims<T>>::contains_key(claim_id) {
                let now = <frame_system::Module<T>>::block_number();
                ensure!(Self::is_active(now), Error::<T>::OutOfBounds);

                let amount = match params {
                    Lockdrop::Bitcoin { value, duration, .. } => {
                        // Cast bitcoin value to PLM order:
                        // satoshi = BTC * 10^9;
                        // PLM unit = PLM * 10^15;
                        // (it also helps to make evaluations more precise)
                        let value_btc = value * 1_000_000;
                        Self::btc_issue_amount(value_btc, duration)
                    },
                    Lockdrop::Ethereum { value, duration, .. } => {
                        // Cast bitcoin value to PLM order:
                        // satoshi = ETH * 10^18;
                        // PLM unit = PLM * 10^15;
                        // (it also helps to make evaluations more precise)
                        let value_eth = value / 1_000;
                        Self::eth_issue_amount(value_eth, duration)
                    }
                };
                let claim = Claim { params, amount, .. Default::default() };
                <Claims<T>>::insert(claim_id, claim);
            }

            <Requests>::mutate(|requests| requests.push(claim_id));
            Self::deposit_event(RawEvent::ClaimRequest(claim_id));

            Ok(())
        }

        /// Claim tokens according to lockdrop procedure.
        /// TODO: weight
        #[weight = 50_000]
        fn claim(
            origin,
            claim_id: ClaimId,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let claim = <Claims<T>>::get(claim_id);
            ensure!(!claim.complete, Error::<T>::AlreadyPaid);

            let approve = claim.approve.len();
            let decline = claim.decline.len();
            ensure!(
                approve + decline >= <VoteThreshold>::get() as usize,
                Error::<T>::NotEnoughVotes,
            );
            ensure!(
                approve.saturating_sub(decline) >= <PositiveVotes>::get() as usize,
                Error::<T>::NotApproved,
            );


            // Deposit lockdrop tokens on locking public key.
            let public_key = match claim.params {
                Lockdrop::Bitcoin { public_key, .. } => public_key,
                Lockdrop::Ethereum { public_key, .. } => public_key,
            };
            let account = T::Account::from(public_key).into_account();
            let amount: BalanceOf<T> = T::BalanceConvert::from(claim.amount).into();
            T::Currency::deposit_creating(&account, amount);

            // Finalize claim request
            <Claims<T>>::mutate(claim_id, |claim| claim.complete = true);
            Self::deposit_event(RawEvent::ClaimComplete(claim_id, account, amount));

            Ok(())
        }

        /// Vote for claim request according to check results. (for authorities only)
        /// TODO: weight
        #[weight = 50_000]
        fn vote(
            origin,
            vote: ClaimVote,
            // since signature verification is done in `validate_unsigned`
            // we can skip doing it here again.
            _signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let keys = Keys::<T>::get();
            if let Some(authority) = keys.get(vote.authority as usize) {
                <HasVote<T>>::insert(authority, &vote.claim_id, true);
                Self::deposit_event(RawEvent::ClaimResponse(vote.claim_id, authority.clone(), vote.approve));

                <Claims<T>>::mutate(&vote.claim_id, |claim|
                    if vote.approve {
                        claim.approve.insert(authority.clone());
                        claim.decline.remove(&authority);
                    }
                    else {
                        claim.decline.insert(authority.clone());
                        claim.approve.remove(&authority);
                    }
                );

                Ok(())
            } else {
                Err(Error::<T>::UnknownAuthority)?
            }
        }

        /// Dollar Rate oracle entrypoint. (for authorities only)
        /// TODO: weight
        #[weight = 50_000]
        fn set_dollar_rate(
            origin,
            rate: TickerRate<T::DollarRate>,
            // since signature verification is done in `validate_unsigned`
            // we can skip doing it here again.
            _signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let now = T::Time::now();

            let keys = Keys::<T>::get();
            if let Some(authority) = keys.get(rate.authority as usize) {
                DollarRateF::<T>::insert(authority, (now, rate.btc, rate.eth));
            } else {
                return Err(Error::<T>::UnknownAuthority)?
            }

            let expire = T::MedianFilterExpire::get();
            let mut btc_filter: Filter<T::DollarRate, T::MedianFilterWidth> = Filter::new();
            let mut eth_filter: Filter<T::DollarRate, T::MedianFilterWidth> = Filter::new();
            let (mut btc_filtered_rate, mut eth_filtered_rate) = <DollarRate<T>>::get();
            for (a, item) in <DollarRateF<T>>::iter() {
                if now.saturating_sub(item.0) < expire {
                    // Use value in filter when not expired
                    btc_filtered_rate = btc_filter.consume(item.1);
                    eth_filtered_rate = eth_filter.consume(item.2);
                } else {
                    // Drop value when expired
                    <DollarRateF<T>>::remove(a);
                }
            }

            <DollarRate<T>>::put((btc_filtered_rate, eth_filtered_rate));
            Self::deposit_event(RawEvent::NewDollarRate(btc_filtered_rate, eth_filtered_rate));

            Ok(())
        }

        /// Set lockdrop alpha value.
        #[weight = 50_000]
        fn set_alpha(origin, alpha_parts: u32) {
            ensure_root(origin)?;
            <Alpha>::put(Perbill::from_parts(alpha_parts));
        }

        /// Set lockdrop held time.
        #[weight = 50_000]
        fn set_bounds(origin, from: T::BlockNumber, to: T::BlockNumber) {
            ensure_root(origin)?;
            ensure!(from < to, "wrong arguments");
            <LockdropBounds<T>>::put((from, to));
        }

        /// Set minimum of positive votes required for lock approve.
        #[weight = 50_000]
        fn set_positive_votes(origin, count: AuthorityVote) {
            ensure_root(origin)?;
            ensure!(count > 0, "wrong argument");
            <PositiveVotes>::put(count);
        }

        /// Set minimum votes required to pass lock confirmation process.
        #[weight = 50_000]
        fn set_vote_threshold(origin, count: AuthorityVote) {
            ensure_root(origin)?;
            ensure!(count > 0, "wrong argument");
            <VoteThreshold>::put(count);
        }

        /// Set lockdrop authorities list.
        #[weight = 50_000]
        fn set_authorities(origin, authorities: Vec<T::AuthorityId>) {
            ensure_root(origin)?;
            Keys::<T>::put(authorities.clone());
            Self::deposit_event(RawEvent::NewAuthorities(authorities));
        }

        // Runs after every block within the context and current state of said block.
        fn offchain_worker(now: T::BlockNumber) {
            // Launch if validator and lockdrop is active.
            if Self::is_active(now) && T::AuthorityId::all().len() > 0 {
                debug::RuntimeLogger::init();

                match Self::offchain() {
                    Err(_) => debug::error!(
                        target: "lockdrop-offchain-worker",
                        "lockdrop worker failed",
                    ),
                    _ => (),
                }
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// The main offchain worker entry point.
    fn offchain() -> Result<(), ()> {
        let btc_price: f32 = BitcoinPrice::fetch()?;
        let eth_price: f32 = EthereumPrice::fetch()?;
        // TODO: add delay to prevent frequent transaction sending
        Self::send_dollar_rate((btc_price as u32).into(), (eth_price as u32).into())?;

        // TODO: use permanent storage to track request when temporary failed
        Self::claim_request_oracle()
    }

    fn claim_request_oracle() -> Result<(), ()> {
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

            for key in T::AuthorityId::all() {
                if let Some(authority) = Self::authority_index_of(&key) {
                    let vote = ClaimVote {
                        authority,
                        claim_id,
                        approve,
                    };
                    let signature = key.sign(&vote.encode()).ok_or(())?;
                    let call = Call::vote(vote, signature);
                    debug::debug!(
                        target: "lockdrop-offchain-worker",
                        "claim id {} => vote extrinsic: {:?}", claim_id, call
                    );

                    let res =
                        SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
                    debug::debug!(
                        target: "lockdrop-offchain-worker",
                        "claim id {} => vote extrinsic send: {:?}", claim_id, res
                    );
                }
            }
        }

        Ok(())
    }

    /// Send dollar rate as unsigned extrinsic from authority.
    fn send_dollar_rate(btc: T::DollarRate, eth: T::DollarRate) -> Result<(), ()> {
        for key in T::AuthorityId::all() {
            if let Some(authority) = Self::authority_index_of(&key) {
                let rate = TickerRate {
                    authority,
                    btc,
                    eth,
                };
                let signature = key.sign(&rate.encode()).ok_or(())?;
                let call = Call::set_dollar_rate(rate, signature);
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "dollar rate extrinsic: {:?}", call
                );

                let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "dollar rate extrinsic send: {:?}", res
                );
            }
        }
        Ok(())
    }

    /// Check locking parameters of given claim.
    fn check_lock(claim_id: ClaimId) -> Result<bool, ()> {
        let Claim { params, .. } = Self::claims(claim_id);
        match params {
            Lockdrop::Bitcoin {
                public_key,
                value,
                duration,
                transaction_hash,
            } => {
                let success = BitcoinLock::check(transaction_hash, public_key, duration, value)?;
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "claim id {} => lock result: {}", claim_id, success
                );
                Ok(success)
            }
            Lockdrop::Ethereum {
                transaction_hash,
                public_key,
                duration,
                value,
            } => {
                let success = EthereumLock::check(transaction_hash, public_key, duration, value)?;
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "claim id {} => lock result: {}", claim_id, success
                );
                Ok(success)
            }
        }
    }

    /// PLM issue amount for given BTC value and locking duration (in secs).
    fn btc_issue_amount(value: u128, duration: u64) -> u128 {
        // https://medium.com/stake-technologies/plasm-lockdrop-introduction-99fa2dfc37c0
        let rate = Self::alpha() * Self::dollar_rate().0 * T::DurationBonus::bonus(duration).into();
        rate.into() * value
    }

    /// PLM issue amount for given ETH value and locking duration (in secs).
    fn eth_issue_amount(value: u128, duration: u64) -> u128 {
        // https://medium.com/stake-technologies/plasm-lockdrop-introduction-99fa2dfc37c0
        let rate = Self::alpha() * Self::dollar_rate().1 * T::DurationBonus::bonus(duration).into();
        rate.into() * value
    }

    /// Check that authority key list contains given account
    fn authority_index_of(public: &T::AuthorityId) -> Option<AuthorityIndex> {
        let keys = Keys::<T>::get();
        // O(n) is ok because of short list
        for (i, elem) in keys.iter().enumerate() {
            if elem.eq(public) {
                return Some(i as AuthorityIndex);
            }
        }
        None
    }

    /// Check that block suits lockdrop bounds.
    fn is_active(now: T::BlockNumber) -> bool {
        let bounds = <LockdropBounds<T>>::get();
        now >= bounds.0 && now < bounds.1
    }
}

impl<T: Trait> sp_runtime::BoundToRuntimeAppPublic for Module<T> {
    type Public = T::AuthorityId;
}

impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
    type Call = Call<T>;

    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
        match call {
            Call::request(params, nonce) => {
                let claim_id = BlakeTwo256::hash_of(&params);
                if <Claims<T>>::get(claim_id).complete {
                    return InvalidTransaction::Custom(ERROR_ALREADY_CLAIMED).into();
                }

                // Simple proof of work
                let pow_byte = BlakeTwo256::hash_of(&(claim_id, nonce)).as_bytes()[0];
                if pow_byte > 0 {
                    return InvalidTransaction::Custom(ERROR_WRONG_POW_PROOF).into();
                }

                ValidTransaction::with_tag_prefix("PlasmLockdrop")
                    .priority(T::UnsignedPriority::get())
                    .and_provides((params, nonce))
                    .longevity(64_u64)
                    .propagate(true)
                    .build()
            }

            Call::claim(claim_id) => {
                let claim = <Claims<T>>::get(claim_id);
                if claim.complete {
                    return InvalidTransaction::Custom(ERROR_ALREADY_CLAIMED).into();
                }

                let approve = claim.approve.len();
                let decline = claim.decline.len();
                let on_vote = approve + decline < <VoteThreshold>::get() as usize;
                let not_approved =
                    approve.saturating_sub(decline) < <PositiveVotes>::get() as usize;
                if on_vote || not_approved {
                    return InvalidTransaction::Custom(ERROR_CLAIM_ON_VOTING).into();
                }

                ValidTransaction::with_tag_prefix("PlasmLockdrop")
                    .priority(T::UnsignedPriority::get())
                    .and_provides(claim_id)
                    .longevity(64_u64)
                    .propagate(true)
                    .build()
            }

            Call::vote(vote, signature) => {
                // Verify call params
                if !<Claims<T>>::contains_key(vote.claim_id.clone()) {
                    return InvalidTransaction::Call.into();
                }

                vote.using_encoded(|encoded_vote| {
                    // Verify authority
                    let keys = Keys::<T>::get();
                    if let Some(authority) = keys.get(vote.authority as usize) {
                        // Check that sender is authority
                        if !authority.verify(&encoded_vote, &signature) {
                            return InvalidTransaction::BadProof.into();
                        }
                    } else {
                        return InvalidTransaction::Custom(ERROR_UNKNOWN_AUTHORITY).into();
                    }

                    ValidTransaction::with_tag_prefix("PlasmLockdrop")
                        .priority(T::UnsignedPriority::get())
                        .and_provides(encoded_vote)
                        .longevity(64_u64)
                        .propagate(true)
                        .build()
                })
            }

            Call::set_dollar_rate(rate, signature) => {
                rate.using_encoded(|encoded_rate| {
                    let keys = Keys::<T>::get();
                    if let Some(authority) = keys.get(rate.authority as usize) {
                        // Check that sender is authority
                        if !authority.verify(&encoded_rate, &signature) {
                            return InvalidTransaction::BadProof.into();
                        }
                    } else {
                        return InvalidTransaction::Custom(ERROR_UNKNOWN_AUTHORITY).into();
                    }

                    ValidTransaction::with_tag_prefix("PlasmLockdrop")
                        .priority(T::UnsignedPriority::get())
                        .and_provides(encoded_rate.to_vec())
                        .longevity(64_u64)
                        .propagate(true)
                        .build()
                })
            }

            _ => InvalidTransaction::Call.into(),
        }
    }
}
