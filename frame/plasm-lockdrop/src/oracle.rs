//! Oracle traits that used in Lockdrop module.

use frame_support::traits::Get;
use sp_runtime::offchain::http::Request;
use sp_runtime::RuntimeDebug;

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

/// CoinGecko(https://coingecko.com) API
pub struct CoinGecko<T: Get<&'static str>>(sp_std::marker::PhantomData<T>);

impl<T, R> PriceOracle<R> for CoinGecko<T> where
    T: Get<&'static str>,
    R: sp_std::str::FromStr,
{
    type Uri = T;
    fn parse(response: Vec<u8>) -> Result<R, String> {
        let ticker: serde_json::Value = serde_json::from_slice(&response.as_slice())
            .map_err(|e| format!("JSON parsing error: {}", e))?;
        let usd = ticker["market_data"]["current_price"]["usd"].to_string();
        let s: Vec<&str> = usd.split_terminator('.').collect();
        s[0].parse()
            .map_err(|_| "Ticker fields parsing error".to_owned())
    }
}


/// Common transaction type.
#[cfg_attr(feature = "std", derive(Eq))]
#[derive(RuntimeDebug, PartialEq, Clone)]
pub struct Transaction {
    /// Transaction sender address.
    pub sender: String,
    /// Transaction recipient address.
    pub recipient: String,
    /// Value in currency units.
    pub value: u128,
    /// Execution script (for Ethereum it's `data` field).
    pub script: Vec<u8>,
    /// Confirmations in blocks
    pub confirmations: u64,
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

/// BlockCypher(https://www.blockcypher.com/) API 
pub struct BlockCypher<T: Get<&'static str>>(sp_std::marker::PhantomData<T>);

impl<T, H> ChainOracle<H> for BlockCypher<T> where
    T: Get<&'static str>,
    H: AsRef<[u8]>,
{
    type Uri = T;
    fn parse(response: Vec<u8>) -> Result<Transaction, String> {
        let tx: serde_json::Value = serde_json::from_slice(&response.as_slice())
            .map_err(|e| format!("JSON parsing error: {}", e))?;
        let sender = tx["inputs"][0]["addresses"]
            .to_string();
        let recipient = tx["outputs"][0]["addresses"][0]
            .to_string();
        let value = tx["outputs"][0]["value"]
            .to_string()
            .parse()
            .map_err(|_| "Transaction `value` field parsing error".to_owned())?;
        let script = hex::decode(tx["outputs"][0]["script"].to_string())
            .map_err(|_| "Unable to decode script field".to_owned())?;
        let confirmations = tx["configurations"]
            .to_string()
            .parse()
            .map_err(|_| "Transaction `confirmations` field parsing error".to_owned())?;
        Ok(Transaction {
            sender,
            recipient,
            value,
            script,
            confirmations,
        })
    }
}
