//! Oracle traits that used in Lockdrop module.

use frame_support::traits::Get;
use sp_runtime::offchain::http::Request;
use sp_runtime::RuntimeDebug;
use sp_core::U256;

/// HTTP source of currency price.
pub trait PriceOracle<T> {
    /// HTTP request URI
    type Uri: Get<&'static str>;

    /// This method should parse HTTP response and return dollar price.
    fn parse(response: Vec<u8>) -> Result<T, String>;

    /// Fetch price data, parse it and return raw dollar rate.
    /// Note: this method requires off-chain worker context.
    fn fetch() -> Result<T, String> {
        let request = Request::get(Self::Uri::get()).send()
            .map_err(|e| format!("HTTP request: {:?}", e))?;
        let response = request.wait()
            .map_err(|e| format!("HTTP response: {:?}", e))?;
        Self::parse(response.body().collect::<Vec<_>>())
    }
}

/// Common transaction type.
#[cfg_attr(feature = "std", derive(Eq))]
#[derive(RuntimeDebug, PartialEq, Clone)]
pub struct Transaction {
    /// Transaction sender address.
    pub sender: Vec<u8>,
    /// Transaction recipient address.
    pub recipient: Vec<u8>,
    /// Value in currency units.
    pub value: U256,
    /// Execution script (for Ethereum it's `data` field).
    pub script: Vec<u8>,
    /// Confirmations in blocks
    pub confirmations: u32,
}


/// HTTP source of blockchain transactions. 
/// For example: http://api.blockcypher.com/v1/btc/test3/txs
pub trait ChainOracle<Hash: AsRef<[u8]>> {
    /// HTTP request URI
    type Uri: Get<&'static str>;

    /// Parse response and return transaction data.
    fn parse(response: Vec<u8>) -> Result<Transaction, String>;

    /// Fetch transaction data from source by given hash.
    /// Note: this method requires off-chain worker context.
    fn fetch(transaction_hash: Hash) -> Result<Transaction, String> {
        let uri = format!(
            "{}/{}",
            Self::Uri::get(),
            hex::encode(transaction_hash),
        );
        let request = Request::get(uri.as_ref()).send()
            .map_err(|e| format!("HTTP request: {:?}", e))?;
        let response = request.wait()
            .map_err(|e| format!("HTTP response: {:?}", e))?;
        Self::parse(response.body().collect::<Vec<_>>())
    }
}
