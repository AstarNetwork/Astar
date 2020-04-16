//! Oracle traits that used in Lockdrop module.

use frame_support::{debug, traits::Get};
use sp_runtime::offchain::http::Request;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;
use simple_json::*;

/// HTTP source of currency price.
pub trait PriceOracle<T> {
    /// HTTP request URI
    type Uri: Get<&'static str>;

    /// This method should parse HTTP response and return dollar price.
    fn parse(response: Vec<u8>) -> Result<T, ()>;

    /// Fetch price data, parse it and return raw dollar rate.
    /// Note: this method requires off-chain worker context.
    fn fetch() -> Result<T, ()> {
        let uri = Self::Uri::get();
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
        Self::parse(response.body().collect::<Vec<_>>())
    }
}

/// CoinGecko(https://coingecko.com) API
pub struct CoinGecko<T: Get<&'static str>>(sp_std::marker::PhantomData<T>);

fn jget(object: json::JsonValue, key: &str) -> Result<json::JsonValue, ()> {
    let key_chars: Vec<char> = key.chars().collect();
    if let json::JsonValue::Object(obj) = object { 
        obj.iter()
           .find(|&(k, _)| *k == key_chars)
           .map(|(_, v)| v.clone())
           .ok_or(())
    } else {
        Err(())
    }
}

fn jarr(array: json::JsonValue, key: usize) -> Result<json::JsonValue, ()> {
    if let json::JsonValue::Array(arr) = array {
        arr.get(key)
           .map(|v| v.clone())
           .ok_or(())
    } else {
        Err(())
    }
}

impl<T> PriceOracle<u128> for CoinGecko<T> where
    T: Get<&'static str>,
{
    type Uri = T;
    fn parse(response: Vec<u8>) -> Result<u128, ()> {
        let str_response = sp_std::str::from_utf8(&response.as_slice()).map_err(|_| ())?;
        let ticker = parse_json(str_response).map_err(|_| ())?; 
        let market_data = jget(ticker, "market_data")?;
        let current_price = jget(market_data, "current_price")?;
        match jget(current_price, "usd")? {
            json::JsonValue::Number(usd) => Ok(usd.integer as u128),
            _ => Err(()),
        }
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
    fn parse(response: Vec<u8>) -> Result<Transaction, ()>;

    /// Fetch transaction data from source by given hash.
    /// Note: this method requires off-chain worker context.
    fn fetch(transaction_hash: Hash) -> Result<Transaction, ()> {
        let uri = [
            Self::Uri::get(),
            hex::encode(transaction_hash).as_str(),
        ].join("/");
        let request = Request::get(uri.as_ref()).send().map_err(|_| ())?;
        let response = request.wait().map_err(|_| ())?;
        Self::parse(response.body().collect::<Vec<_>>())
    }
}

pub trait AddressDecoder {
    fn decode(input: Vec<char>) -> Result<Vec<u8>, ()>;
}

fn encode_ascii(input: Vec<char>) -> Vec<u8> {
    input.iter().map(|b| {
        let mut buf = [0; 2];
        b.encode_utf8(&mut buf);
        buf[0]
    }).collect()
}

/// Standard bitcoin address decoder.
pub struct BitcoinAddress;
impl AddressDecoder for BitcoinAddress {
    fn decode(input: Vec<char>) -> Result<Vec<u8>, ()> {
        // input is ascii string
        bs58::decode(encode_ascii(input))
            .into_vec()
            .map_err(|_| ())
    }
}

/// Standard ethereum address decoder.
pub struct EthereumAddress;
impl AddressDecoder for EthereumAddress {
    fn decode(input: Vec<char>) -> Result<Vec<u8>, ()> {
        hex::decode(encode_ascii(input))
            .map_err(|_| ())
    }
}

/// BlockCypher(https://www.blockcypher.com/) API 
pub struct BlockCypher<T: Get<&'static str>, D: AddressDecoder>
    (sp_std::marker::PhantomData<(T, D)>);

impl<T, D, H> ChainOracle<H> for BlockCypher<T, D> where
    T: Get<&'static str>,
    H: AsRef<[u8]>,
    D: AddressDecoder,
{
    type Uri = T;
    fn parse(response: Vec<u8>) -> Result<Transaction, ()> {
        let str_response = sp_std::str::from_utf8(&response.as_slice()).map_err(|_| ())?;
        let tx = parse_json(str_response).map_err(|_| ())?;
        let inputs = jget(tx.clone(), "inputs")?;
        let outputs = jget(tx.clone(), "outputs")?;


        let sender    = match jget(jarr(inputs, 0)?, "addresses")? {
            json::JsonValue::String(sender) => D::decode(sender)?,
            _ => Err(())?,
        };
        let recipient = match jget(jarr(outputs.clone(), 0)?, "addresses")? {
            json::JsonValue::String(recipient) => D::decode(recipient)?,
            _ => Err(())?,
        };
        let value     = match jget(jarr(outputs.clone(), 0)?, "value")? {
            json::JsonValue::Number(value) => value.integer as u128,
            _ => Err(())?
        };
        let script = match jget(jarr(outputs, 0)?, "script")? {
            json::JsonValue::String(script) => hex::decode(encode_ascii(script)).map_err(|_| ())?,
            _ => Err(())?
        };
        let confirmations = match jget(tx, "confirmations")? {
            json::JsonValue::Number(confirmations) => confirmations.integer as u64,
            _ => Err(())?
        };

        Ok(Transaction {
            sender,
            recipient,
            value,
            script,
            confirmations,
        })
    }
}
