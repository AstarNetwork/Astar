//! Web-server helper for Lockdrop runtime module.

#![feature(proc_macro_hygiene)]

use codec::Decode;
use pallet_plasm_lockdrop::LockCheck;
use tide::{http::StatusCode, Response};
use web3::futures::Future;

mod btc_utils;
mod cli;
mod eth_utils;

pub use cli::Config;

const COINGECKO_BTC_API: &'static str = "https://api.coingecko.com/api/v3/coins/bitcoin";
const COINGECKO_ETH_API: &'static str = "https://api.coingecko.com/api/v3/coins/ethereum";

pub async fn start(config: Config) {
    let mut app = tide::with_state(config);

    app.at("/btc/ticker").get(|_| async {
        let ticker: serde_json::Value = reqwest::blocking::get(COINGECKO_BTC_API)?.json()?;
        Ok(ticker["market_data"]["current_price"]["usd"].to_string())
    });

    app.at("/eth/ticker").get(|_| async {
        let ticker: serde_json::Value = reqwest::blocking::get(COINGECKO_ETH_API)?.json()?;
        Ok(ticker["market_data"]["current_price"]["usd"].to_string())
    });

    app.at("/btc/lock")
        .post(|mut req: tide::Request<Config>| async move {
            let body = req.body_bytes().await?;
            let lock = LockCheck::decode(&mut &body[..])?;
            log::debug!(
                target: "lockdrop-oracle",
                "BTC lock check request: {:#?}", lock
            );

            let uri = format!(
                "{}/{}",
                req.state().bitcoin_endpoint,
                hex::encode(lock.tx_hash)
            );
            let tx: serde_json::Value = reqwest::blocking::get(uri.as_str())?.json()?;
            log::debug!(
                target: "lockdrop-oracle",
                "BTC tx at {}: {}", lock.tx_hash, tx.to_string()
            );

            let tx_sender = tx["inputs"][0]["addresses"].to_string();
            let tx_value = tx["outputs"][0]["value"].as_u64().unwrap_or(0) as u128;
            let tx_script = hex::decode(tx["outputs"][0]["script"].to_string())?;
            let tx_confirmations = tx["confirmations"].as_u64().unwrap_or(0);

            // check transaction confirmations
            if tx_confirmations < req.state().safe_btc_confirmations {
                log::debug!(target: "lockdrop-oracle", "transaction isn't confirmed yet");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            // check transaction value
            if tx_value != lock.value {
                log::debug!(target: "lockdrop-oracle", "lock value mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            let lock_sender = btc_utils::to_address(&lock.public_key);
            log::debug!(
                target: "lockdrop-oracle",
                "BTC address for public key {}: {}",
                lock.public_key,
                lock_sender,
            );
            // check transaction sender address
            if tx_sender != lock_sender {
                log::debug!(target: "lockdrop-oracle", "sender address mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            // assembly bitcoin script for given params
            let blocks = (lock.duration / 600) as u32;
            let lock_script = btc_utils::lock_script(&lock.public_key, blocks)
                .map_err(|e| tide::Error::from_str(tide::StatusCode::BadRequest, e))?;
            log::debug!(
                target: "lockdrop-oracle",
                "Lock script address for public ({}), duration({}): {}",
                hex::encode(lock.public_key),
                lock.duration,
                hex::encode(lock_script.as_bytes()),
            );
            // check script code
            if tx_script != lock_script.into_bytes() {
                log::debug!(target: "lockdrop-oracle", "lock script mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            Ok(Response::new(StatusCode::Ok))
        });

    app.at("/eth/lock")
        .post(|mut req: tide::Request<Config>| async move {
            let body = req.body_bytes().await?;
            let lock = LockCheck::decode(&mut &body[..])?;
            log::debug!(
                target: "lockdrop-oracle",
                "ETH lock check request: {:#?}", lock
            );

            let (_eloop, transport) =
                web3::transports::Http::new(req.state().ethereum_endpoint.as_str()).unwrap();
            let web3 = web3::Web3::new(transport);
            let block_number = web3.eth().block_number().wait()?;
            let tx = web3
                .eth()
                .transaction(web3::types::TransactionId::Hash(lock.tx_hash))
                .wait()?
                .unwrap();
            log::debug!(
                target: "lockdrop-oracle",
                "Ethereum transaction at {}: {:#?}", lock.tx_hash, tx
            );

            // check transaction value
            if tx.value != lock.value.into() {
                log::debug!(target: "lockdrop-oracle", "lock value mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            let tx_block_number = tx.block_number.unwrap_or(Default::default());
            let tx_confirmations = if block_number > tx_block_number {
                block_number - tx_block_number
            } else {
                Default::default()
            };
            log::debug!(
                target: "lockdrop-oracle",
                "Transaction confirmations: {}", tx_confirmations
            );
            if tx_confirmations < req.state().safe_eth_confirmations.into() {
                log::debug!(target: "lockdrop-oracle", "transaction isn't confirmed yet");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            let sender =
                eth_utils::to_address(lock.public_key.as_ref()).unwrap_or(Default::default());
            log::debug!(
                target: "lockdrop-oracle",
                "ETH address for public key {}: {}",
                lock.public_key, sender
            );
            // check sender address
            if tx.from != sender {
                log::debug!(target: "lockdrop-oracle", "sender address mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            // check that destination is lockdrop smart contract
            if tx.to != Some(req.state().lockdrop_contract.parse()?) {
                log::debug!(target: "lockdrop-oracle", "contract address mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            // check smart contract method input
            if !eth_utils::lock_method_check(tx.input.0.as_ref(), lock.duration) {
                log::debug!(target: "lockdrop-oracle", "lock method mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            Ok(Response::new(StatusCode::Ok))
        });

    app.listen("127.0.0.1:34347")
        .await
        .map_err(|e| log::error!("oracle web-server error: {}", e))
        .unwrap_or(());
}
