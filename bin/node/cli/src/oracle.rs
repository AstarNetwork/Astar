//! Web-server helper for Lockdrop runtime module.

use codec::Encode;
use pallet_plasm_lockdrop::Transaction;

const COINGECKO_BTC_API: &'static str = "https://api.coingecko.com/api/v3/coins/bitcoin";
const COINGECKO_ETH_API: &'static str = "https://api.coingecko.com/api/v3/coins/ethereum";
const BLOCKCYPHER_BTC_API: &'static str = "https://api.blockcypher.com/v1/btc/test3/txs";
const BLOCKCYPHER_ETH_API: &'static str = "https://api.blockcypher.com/v1/eth/main/txs";

pub async fn start() {
    let mut app = tide::new();

    app.at("/ticker/btc").get(|_| async {
        let ticker: serde_json::Value = reqwest::blocking::get(COINGECKO_BTC_API)?.json()?;
        Ok(ticker["market_data"]["current_price"]["usd"].to_string())
    });

    app.at("/ticker/eth").get(|_| async {
        let ticker: serde_json::Value = reqwest::blocking::get(COINGECKO_ETH_API)?.json()?;
        Ok(ticker["market_data"]["current_price"]["usd"].to_string())
    });

    app.at("/tx/btc/:hash")
        .get(|req: tide::Request<()>| async move {
            let hash: String = req.param("hash")?;
            let uri = format!("{}/{}", BLOCKCYPHER_BTC_API, hash);
            let tx: serde_json::Value = reqwest::blocking::get(uri.as_str())?.json()?;
            let encoded = Transaction {
                sender: bs58::decode(tx["inputs"][0]["addresses"].to_string()).into_vec()?,
                recipient: bs58::decode(tx["outputs"][0]["addresses"].to_string()).into_vec()?,
                value: tx["outputs"][0]["value"].as_u64().unwrap_or(0) as u128,
                script: hex::decode(tx["outputs"][0]["script"].to_string())?,
                confirmations: tx["confirmations"].as_u64().unwrap_or(0),
            }
            .encode();
            Ok(hex::encode(encoded))
        });

    app.at("/tx/eth/:hash")
        .get(|req: tide::Request<()>| async move {
            let hash: String = req.param("hash")?;
            let uri = format!("{}/{}", BLOCKCYPHER_ETH_API, hash);
            let tx: serde_json::Value = reqwest::blocking::get(uri.as_str())?.json()?;
            log::info!("Response received: {}", tx);
            let encoded = Transaction {
                sender: hex::decode(tx["inputs"][0]["addresses"].to_string())?,
                recipient: hex::decode(tx["outputs"][0]["addresses"].to_string())?,
                value: tx["outputs"][0]["value"].as_u64().unwrap_or(0) as u128,
                script: hex::decode(tx["outputs"][0]["script"].to_string())?,
                confirmations: tx["confirmations"].as_u64().unwrap_or(0),
            }
            .encode();
            Ok(hex::encode(encoded))
        });

    app.listen("127.0.0.1:34347")
        .await
        .map_err(|e| log::error!("oracle web-server error: {}", e))
        .unwrap_or(());
}
