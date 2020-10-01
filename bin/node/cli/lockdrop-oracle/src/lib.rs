//! Web-server helper for Lockdrop runtime module.

#![feature(proc_macro_hygiene)]

use codec::Decode;
use pallet_plasm_lockdrop::LockCheck;
use tide::{http::StatusCode, Response};

mod chainlink;
mod cli;
mod eth_utils;

pub use cli::Config;

#[derive(Clone)]
struct ServerState {
    pub ethereum_transport: web3::transports::Http,
    pub lockdrop_contract: web3::types::Address,
    pub safe_eth_confirmations: u64,
}

pub async fn start(config: Config) {
    let ethereum_transport =
        web3::transports::Http::new(config.ethereum_endpoint.as_str()).unwrap();
    let lockdrop_contract = config.lockdrop_contract.parse().unwrap();
    let safe_eth_confirmations = config.safe_eth_confirmations;
    let mut app = tide::with_state(ServerState {
        ethereum_transport,
        lockdrop_contract,
        safe_eth_confirmations,
    });

    app.at("/eth/ticker")
        .get(|req: tide::Request<ServerState>| async move {
            let web3 = web3::Web3::new(req.state().ethereum_transport.clone());
            let usd_price = chainlink::eth_usd(web3).await? / 10_u128.pow(8);
            Ok(usd_price.to_string())
        });

    app.at("/eth/lock")
        .post(|mut req: tide::Request<ServerState>| async move {
            let body = req.body_bytes().await?;
            let lock = LockCheck::decode(&mut &body[..])?;
            log::debug!(
                target: "lockdrop-oracle",
                "ETH lock check request: {:#?}", lock
            );

            let web3 = web3::Web3::new(req.state().ethereum_transport.clone());
            let block_number = web3.eth().block_number().await?;
            let tx = web3
                .eth()
                .transaction(web3::types::TransactionId::Hash(lock.tx_hash))
                .await?
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
            if tx.to != Some(req.state().lockdrop_contract) {
                log::debug!(target: "lockdrop-oracle", "contract address mismatch");
                return Ok(Response::new(StatusCode::BadRequest));
            }

            // check smart contract method input
            let lock_method = eth_utils::lock_method(lock.duration);
            if tx.input.0[0..36] == lock_method[0..36] {
                log::debug!(
                    target: "lockdrop-oracle",
                    "lock method mismatch: {} /= {}",
                    hex::encode(tx.input.0),
                    hex::encode(lock_method),
                );
                return Ok(Response::new(StatusCode::BadRequest));
            }

            Ok(Response::new(StatusCode::Ok))
        });

    app.listen("127.0.0.1:34347")
        .await
        .map_err(|e| log::error!("oracle web-server error: {}", e))
        .unwrap_or(());
}
