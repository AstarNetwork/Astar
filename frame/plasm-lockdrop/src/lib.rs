//! Plasm Lockdrop module. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use sp_std::prelude::*;
use sp_core::{H256, ecdsa};
use sp_runtime::{
    RuntimeDebug,
    traits::{
        Member, IdentifyAccount, BlakeTwo256, Hash, Saturating, AtLeast32Bit,
    },
    app_crypto::{KeyTypeId, RuntimeAppPublic},
    offchain::http::Request,
};
use frame_support::{
    decl_module, decl_event, decl_storage, decl_error,
    debug, ensure, StorageValue, StorageMap,
    weights::SimpleDispatchInfo,
    traits::{Get, Currency, Time},
    storage::IterableStorageMap,
    dispatch::Parameter,
};
use frame_system::{
    self as system, ensure_signed,
    offchain::SubmitSignedTransaction,
};

/// Bitcoin helpers.
mod btc_utils;
/// Ethereum helpers.
mod eth_utils;

/// Plasm Lockdrop Authority local KeyType.
///
/// For security reasons the offchain worker doesn't have direct access to the keys
/// but only to app-specific subkeys, which are defined and grouped by their `KeyTypeId`.
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"plaa");

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

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

/// The module's main configuration trait.
pub trait Trait: system::Trait {
    /// The lockdrop balance.
    type Currency: Currency<Self::AccountId>;

    /// How much authority votes module should receive to decide claim result.
    type VoteThreshold: Get<AuthorityVote>;

    /// How much positive votes requered to approve claim.
    ///   Positive votes = approve votes - decline votes.
    type PositiveVotes: Get<AuthorityVote>;

    /// Bitcoin price URI.
    type BitcoinTickerUri: Get<&'static str>;

    /// Ethereum price URI.
    type EthereumTickerUri: Get<&'static str>;

    /// Bitcoin transaction fetch URI.
    /// For example: http://api.blockcypher.com/v1/btc/test3/txs
    type BitcoinApiUri: Get<&'static str>;

    /// Ethereum public node URI. 
    /// For example: https://api.blockcypher.com/v1/eth/test/txs/ 
    type EthereumApiUri: Get<&'static str>;

    /// Ethereum lockdrop contract address.
    type EthereumContractAddress: Get<&'static str>;

    /// Timestamp of finishing lockdrop.
    type LockdropEnd: Get<Self::Moment>;

    /// How long dollar rate parameters valid in secs
    type MedianFilterExpire: Get<Self::Moment>;

    /// Width of dollar rate median filter.
    type MedianFilterWidth: Get<usize>;

    /// A dispatchable call type.
    type Call: From<Call<Self>>;

    /// Let's define the helper we use to create signed transactions.
    type SubmitTransaction: SubmitSignedTransaction<Self, <Self as Trait>::Call>;

    /// The identifier type for an authority.
    type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord
        + IdentifyAccount<AccountId=<Self as system::Trait>::AccountId>;

    /// System level account type.
    /// This used for resolving account ID's of ECDSA lockdrop public keys.
    type Account: IdentifyAccount<AccountId=Self::AccountId> + From<ecdsa::Public>;

    /// Module that could provide timestamp.
    type Time: Time<Moment=Self::Moment>;

    /// Timestamp type.
    type Moment: Member + Parameter + Saturating + AtLeast32Bit
        + Copy + Default + From<u64> + Into<u64> + Into<u128>;

    /// Dollar rate number data type.
    type DollarRate: Member + Parameter + AtLeast32Bit
        + Copy + Default + From<u64> + Into<u128>;

    // XXX: I don't known how to convert into Balance from u128 without it
    // TODO: Should be removed
    type BalanceConvert: From<u128> +
        Into<<Self::Currency as Currency<<Self as frame_system::Trait>::AccountId>>::Balance>;

    /// The regular events type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

/// Claim id is a hash of claim parameters.
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
    /// Duration in blocks and value in shatoshi could be derived from BTC transaction.
    Bitcoin { public: ecdsa::Public, value: u64, duration: u64, transaction_hash: H256, },
    /// Ethereum lockdrop transactions is sended to pre-deployed lockdrop smart contract.
    Ethereum { public: ecdsa::Public, value: u64, duration: u64, transaction_hash: H256, },
}

impl Default for Lockdrop {
    fn default() -> Self {
        Lockdrop::Bitcoin {
            public: Default::default(),
            value: Default::default(),
            duration: Default::default(),
            transaction_hash: Default::default(),
        }
    }
}

/// Lockdrop claim request description.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug, Default, Clone)]
pub struct Claim {
    params:   Lockdrop,
    approve:  AuthorityVote,
    decline:  AuthorityVote,
    amount:   u128,
    complete: bool,
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
        ClaimResponse(ClaimId, AccountId, bool),
        /// Lockdrop token claim paid
        ClaimComplete(ClaimId, AccountId, Balance),
        /// Dollar rate updated by oracle: BTC, ETH.
        NewDollarRate(DollarRate, DollarRate),
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
        Claims get(fn claims): map hasher(blake2_128_concat)
                               ClaimId => Claim;
        /// Lockdrop alpha parameter.
        Alpha get(fn alpha) config(): u128;
        /// Lockdrop dollar rate parameter: BTC, ETH.
        DollarRate get(fn dollar_rate) config(): (T::DollarRate, T::DollarRate);
        /// Lockdrop dollar rate median filter table: Time, BTC, ETH.
        DollarRateF get(fn dollar_rate_f): map hasher(blake2_128_concat)
                                           T::AccountId => (T::Moment, T::DollarRate, T::DollarRate);
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
            let _ = ensure_signed(origin)?;
            let claim_id = BlakeTwo256::hash_of(&params);
            ensure!(!<Claims>::get(claim_id).complete, "claim should not be already paid"); 

            if !<Claims>::contains_key(claim_id) {
                let amount = match params {
                    Lockdrop::Bitcoin { value, duration, .. } => {
                        // Average block duration in BTC is 10 min = 600 sec
                        let duration_sec = duration * 600;
                        // Cast bitcoin value to PLM order:
                        // satoshi = BTC * 10^9;
                        // PLM unit = PLM * 10^15;
                        // (it also helps to make evaluations more precise)
                        let value_btc = (value as u128) * 1_000_000;
                        Self::btc_issue_amount(value_btc, duration_sec.into())
                    },
                    Lockdrop::Ethereum { value, duration, .. } => {
                        // Cast bitcoin value to PLM order:
                        // satoshi = ETH * 10^18;
                        // PLM unit = PLM * 10^15;
                        // (it also helps to make evaluations more precise)
                        let value_eth = (value as u128) / 1_000;
                        Self::eth_issue_amount(value_eth, duration.into())
                    }
                };
                let claim = Claim { params, amount, .. Default::default() };
                <Claims>::insert(claim_id, claim);
            }

            <Requests>::mutate(|requests| requests.push(claim_id));
            Self::deposit_event(RawEvent::ClaimRequest(claim_id));
        }

        /// Claim tokens according to lockdrop procedure.
        #[weight = SimpleDispatchInfo::FixedNormal(1_000_000)]
        fn claim(
            origin,
            claim_id: ClaimId,
        ) {
            let _ = ensure_signed(origin)?;
            let claim = <Claims>::get(claim_id);
            ensure!(!claim.complete, "claim should be already paid"); 

            let positive = claim.approve.saturating_sub(claim.decline) > T::PositiveVotes::get();
            let threshold = claim.approve + claim.decline > T::VoteThreshold::get();
            if !threshold || !positive {
                Err("this request don't get enough authority confirmations")?
            }

            // Deposit lockdrop tokens on locking public key.
            let account = match claim.params {
                Lockdrop::Bitcoin { public, .. }  => T::Account::from(public).into_account(),
                Lockdrop::Ethereum { public, .. } => T::Account::from(public).into_account(),
            };
            let amount: BalanceOf<T> = T::BalanceConvert::from(claim.amount).into();
            T::Currency::deposit_creating(&account, amount);

            // Finalize claim request
            <Claims>::mutate(claim_id, |claim| claim.complete = true);
            Self::deposit_event(RawEvent::ClaimComplete(claim_id, account, amount));
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
            ensure!(<Claims>::contains_key(claim_id), "request with this id doesn't exist");

            <Claims>::mutate(&claim_id, |claim|
                if approve { claim.approve += 1 }
                else { claim.decline += 1 }
            );

            Self::deposit_event(RawEvent::ClaimResponse(claim_id, sender, approve));
        }

        /// Dollar rate oracle method. (for authorities only) 
        #[weight = SimpleDispatchInfo::FixedOperational(10_000)]
        fn set_dollar_rate(
            origin,
            btc_rate: T::DollarRate,
            eth_rate: T::DollarRate,
        ) {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_authority(&sender), "this method for lockdrop authorities only");

            let now = T::Time::now();
            <DollarRateF<T>>::insert(sender, (now, btc_rate, eth_rate));

            let expire = T::MedianFilterExpire::get();
            let mut btc_filter = median::Filter::new(T::MedianFilterWidth::get());
            let mut eth_filter = median::Filter::new(T::MedianFilterWidth::get());
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
        }

        // Runs after every block within the context and current state of said block.
        fn offchain_worker(_now: T::BlockNumber) {
            debug::RuntimeLogger::init();

            if sp_io::offchain::is_validator() {
                match Self::offchain() {
                    Err(e) => debug::error!(
                        target: "lockdrop-offchain-worker",
                        "lockdrop worker fails: {}", e
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
        Self::claim_request_oracle()?;

        // TODO: add delay to prevent frequent transaction sending
        Self::dollar_rate_oracle()
    }

    /// Check that authority key list contains given account
    fn is_authority(account: &T::AccountId) -> bool {
        Keys::<T>::get()
            .binary_search_by(|key| key.clone().into_account().cmp(account))
            .is_ok()
    }

    fn claim_request_oracle() -> Result<(), String> {
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

    fn dollar_rate_oracle() -> Result<(), String> {
        // TODO: Multiple rate sources
        let res = fetch_json(T::BitcoinTickerUri::get())?;
        let btc_rate = res["price_usd"].as_u64()
            .ok_or("BTC ticker JSON parsing error".to_string())?;

        let res = fetch_json(T::EthereumTickerUri::get())?;
        let eth_rate = res["price_usd"].as_u64()
            .ok_or("ETH ticker JSON parsing error".to_string())?;

        let call = Call::set_dollar_rate(btc_rate.into(), eth_rate.into());
        debug::debug!(
            target: "lockdrop-offchain-worker",
            "dollar rate extrinsic: {:?}", call
        );

        let res = T::SubmitTransaction::submit_signed(call);
        debug::debug!(
            target: "lockdrop-offchain-worker",
            "dollar rate extrinsic send: {:?}", res
        );

        Ok(())
    }

    /// Check locking parameters of given claim
    fn check_lock(claim_id: ClaimId) -> Result<bool, String> {
        let Claim { params, .. } = Self::claims(claim_id);
        match params {
            Lockdrop::Bitcoin { public, value, duration, transaction_hash } => {
                let uri = format!(
                    "{}/{}",
                    T::BitcoinApiUri::get(),
                    hex::encode(transaction_hash),
                );
                let tx = fetch_json(uri.as_ref())?;
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "claim id {} => fetched transaction: {:?}", claim_id, tx
                );

                let lock_script = btc_utils::lock_script(public, duration);
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "claim id {} => desired lock script: {}", claim_id, hex::encode(lock_script.clone())
                );

                let script = btc_utils::p2sh(&btc_utils::script_hash(&lock_script[..]));
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "claim id {} => desired P2HS script: {}", claim_id, hex::encode(script.clone())
                );

                Ok(tx["configurations"].as_u64().unwrap() > 10 &&
                   tx["outputs"][0]["script"] == serde_json::json!(hex::encode(script)) &&
                   tx["outputs"][0]["value"].as_u64().unwrap() == value)
            },
            Lockdrop::Ethereum { public, value, duration, transaction_hash } => {
                let uri = format!(
                    "{}/{}",
                    T::EthereumApiUri::get(),
                    hex::encode(transaction_hash),
                );
                let tx = fetch_json(uri.as_ref())?;
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "claim id {} => fetched transaction: {:?}", claim_id, tx
                );

                let script = eth_utils::lock_method(duration);
                debug::debug!(
                    target: "lockdrop-offchain-worker",
                    "claim id {} => desired lock script: {}", claim_id, hex::encode(script.clone())
                );

                Ok(tx["configurations"].as_u64().unwrap() > 10 &&
                   tx["inputs"][0]["addresses"] == serde_json::json!(eth_utils::to_address(public)) &&
                   tx["outputs"][0]["script"] == serde_json::json!(hex::encode(script)) &&
                   tx["outputs"][0]["value"].as_u64().unwrap() == value &&
                   tx["outputs"][0]["addresses"][0] == serde_json::json!(T::EthereumContractAddress::get()))
            }
        }
    }

    fn btc_issue_amount(value: u128, duration: T::Moment) -> u128 {
        // https://medium.com/stake-technologies/plasm-lockdrop-introduction-99fa2dfc37c0
        Self::alpha() * value * Self::dollar_rate().0.into() * Self::time_bonus(duration)
    }

    fn eth_issue_amount(value: u128, duration: T::Moment) -> u128 {
        // https://medium.com/stake-technologies/plasm-lockdrop-introduction-99fa2dfc37c0
        Self::alpha() * value * Self::dollar_rate().1.into() * Self::time_bonus(duration)
    }

    fn time_bonus(duration: T::Moment) -> u128 {
        let days: u64 = 24 * 60 * 60; // One day in seconds
        let duration_sec = Into::<u64>::into(duration);
        if duration_sec < 30 * days {
            0 // Dont permit to participate with locking less that one month
        } else if duration_sec < 100 * days {
            24
        } else if duration_sec < 300 * days {
            100
        } else if duration_sec < 1000 * days {
            360
        } else {
            1600
        }
    }
}

/// HTTP fetch JSON value by URI
fn fetch_json(uri: &str) -> Result<serde_json::Value, String> {
    let request = Request::get(uri).send()
        .map_err(|e| format!("HTTP request error: {:?}", e))?;
    let response = request.wait()
        .map_err(|e| format!("HTTP response error: {:?}", e))?;
    serde_json::to_value(response.body().clone().collect::<Vec<_>>())
        .map_err(|e| format!("JSON decode error: {}", e))
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
