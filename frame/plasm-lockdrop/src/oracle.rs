//! Plasm Lockdrop Oracle module client.
//!
//! Lockdrop Oracle has REST HTTP API:
//!
//! **GET**
//! - /btc/ticker - returns BTC price in USD
//! - /eth/ticker - returns ETH price in USD
//!
//! **POST**
//! - /eth/lock
//!   Body: LockCheck struct
//!   Returns: `OK` when lock success
//!
//! - /btc/lock
//!   Body: LockCheck struct
//!   Returns `OK` when lock success
//!

use codec::{Decode, Encode};
use frame_support::debug;
use sp_core::{ecdsa, H256};
use sp_runtime::{offchain::http::Request, RuntimeDebug};
use sp_std::prelude::*;

/// HTTP source of currency price.
pub trait PriceOracle<T: sp_std::str::FromStr> {
    /// HTTP request URI
    fn uri() -> &'static str;

    /// Fetch price data, parse it and return raw dollar rate.
    /// Note: this method requires off-chain worker context.
    fn fetch() -> Result<T, ()> {
        let uri = Self::uri();
        debug::debug!(
            target: "lockdrop-offchain-worker",
            "Price oracle request to {}", uri
        );

        let request = Request::get(uri).send().map_err(|e| {
            debug::error!(
                target: "lockdrop-offchain-worker",
                "Request error: {:?}", e
            );
        })?;

        let response = request.wait().map_err(|e| {
            debug::error!(
                target: "lockdrop-offchain-worker",
                "Response error: {:?}", e
            );
        })?;

        let body = response.body().collect::<Vec<_>>();
        let price = sp_std::str::from_utf8(&body[..]).map_err(|e| {
            debug::error!(
                target: "lockdrop-offchain-worker",
                "Response body isn't UTF-8 string: {:?}", e
            );
        })?;

        price.parse().map_err(|_| {
            debug::error!(
                target: "lockdrop-offchain-worker",
                "Response body string parsing error"
            );
        })
    }
}

/// BTC price oracle.
pub struct BitcoinPrice;
impl<T: sp_std::str::FromStr> PriceOracle<T> for BitcoinPrice {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/btc/ticker"
    }
}

/// ETH price oracle.
pub struct EthereumPrice;
impl<T: sp_std::str::FromStr> PriceOracle<T> for EthereumPrice {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/eth/ticker"
    }
}

/// Lock check request parameters.
#[derive(Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode)]
pub struct LockCheck {
    /// Transaction hash.
    pub tx_hash: H256,
    /// Sender public key.
    pub public_key: ecdsa::Public,
    /// Lock duration in seconds.
    pub duration: u64,
    /// Lock value in units.
    pub value: u128,
}

/// HTTP source of blockchain transactions.
pub trait LockOracle {
    /// HTTP request URI
    fn uri() -> &'static str;

    /// Check lock transaction data.
    /// Note: this method requires off-chain worker context.
    fn check(
        tx_hash: H256,
        public_key: ecdsa::Public,
        duration: u64,
        value: u128,
    ) -> Result<bool, ()> {
        let lock = LockCheck {
            tx_hash,
            public_key,
            duration,
            value,
        };
        let request = Request::post(Self::uri(), vec![lock.encode()])
            .send()
            .map_err(|e| {
                debug::error!(
                    target: "lockdrop-offchain-worker",
                    "Request error: {:?}", e
                );
            })?;

        let response = request.wait().map_err(|e| {
            debug::error!(
                target: "lockdrop-offchain-worker",
                "Response error: {:?}", e
            );
        })?;

        Ok(response.code == 200)
    }
}

/// Bitcoin chain transactions oracle.
pub struct BitcoinLock;
impl LockOracle for BitcoinLock {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/btc/lock"
    }
}

/// Ethereum chain transactions oracle.
pub struct EthereumLock;
impl LockOracle for EthereumLock {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/eth/lock"
    }
}
