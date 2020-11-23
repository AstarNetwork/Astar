//! Lockdrop module CLI parameters.

use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct Config {
    /// Ethereum node endpoint.
    #[structopt(long, default_value = "https://ropsten.infura.io/v3")]
    pub ethereum_endpoint: String,
    /// Ethereum lockdrop smart contract address.
    #[structopt(long, default_value = "33251e1298dF5Ff84166E62Abecf85FCCD1A1241")]
    pub lockdrop_contract: String,
    /// Ethereum minimum transaction confirmations.
    #[structopt(long, default_value = "10")]
    pub safe_eth_confirmations: u64,
}
