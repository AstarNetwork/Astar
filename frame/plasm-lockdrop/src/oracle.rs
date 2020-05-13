//! Plasm Lockdrop Oracle module client.
//!
//! Lockdrop Oracle has REST HTTP API:
//! - /ticker/btc - returns BTC price in USD
//! - /ticker/eth - returns ETH price in USD
//! - /tx/btc/${tx_hash} - returns transaction by it's hash
//! - /tx/eth/${tx_hash} - returns transaction by it's hash

use codec::{Decode, Encode};
use frame_support::debug;
use sp_runtime::offchain::http::Request;
use sp_runtime::RuntimeDebug;
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
                "Request error {:?}", e
            );
        })?;
        let response = request.wait().map_err(|e| {
            debug::error!(
                target: "lockdrop-offchain-worker",
                "Response error {:?}", e
            );
        })?;
        let body = response.body().collect::<Vec<_>>();
        let price = sp_std::str::from_utf8(&body[..]).map_err(|_| ())?;
        price.parse().map_err(|_| ())
    }
}

/// BTC price oracle.
pub struct BitcoinPrice;
impl<T: sp_std::str::FromStr> PriceOracle<T> for BitcoinPrice {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/ticker/btc"
    }
}

/// ETH price oracle.
pub struct EthereumPrice;
impl<T: sp_std::str::FromStr> PriceOracle<T> for EthereumPrice {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/ticker/eth"
    }
}

/// Common transaction type.
#[cfg_attr(feature = "std", derive(Eq, Encode))]
#[derive(RuntimeDebug, PartialEq, Clone, Decode)]
pub struct Transaction {
    /// Transaction sender address.
    pub sender: Vec<u8>,
    /// Transaction recipient address.
    pub recipient: Vec<u8>,
    /// Value in currency units.
    pub value: u128,
    /// Execution script (for Ethereum it's `data` field).
    pub script: Vec<u8>,
    /// Confirmations in blocks
    pub confirmations: u64,
}

/// HTTP source of blockchain transactions.
pub trait ChainOracle {
    /// HTTP request URI
    fn uri() -> &'static str;

    /// Fetch transaction data from source by given hash.
    /// Note: this method requires off-chain worker context.
    fn fetch<Hash: AsRef<[u8]>>(transaction_hash: Hash) -> Result<Transaction, ()> {
        let uri = [Self::uri(), hex::encode(transaction_hash).as_str()].join("/");
        let request = Request::get(uri.as_ref()).send().map_err(|_| ())?;
        let response = request.wait().map_err(|_| ())?;
        let body = hex::decode(response.body().collect::<Vec<_>>()).map_err(|_| ())?;
        Transaction::decode(&mut &body[..]).map_err(|_| ())
    }
}

/// Bitcoin chain transactions oracle.
pub struct BitcoinChain;
impl ChainOracle for BitcoinChain {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/tx/btc"
    }
}

/// Ethereum chain transactions oracle.
pub struct EthereumChain;
impl ChainOracle for EthereumChain {
    fn uri() -> &'static str {
        "http://127.0.0.1:34347/tx/eth"
    }
}
