//! Lockdrop module CLI parameters. 

use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct Config {
    /// Bitcoin node endpoint.
    #[structopt(long, default_value = "https://api.blockcypher.com/v1/btc/test3/txs")]
    pub bitcoin_endpoint: String,
    /// Bitcoin minimum transaction confirmations.
    #[structopt(long, default_value = "8")]
    pub safe_btc_confirmations: u64,
    /// Ethereum node endpoint.
    #[structopt(long, default_value = "https://ropsten.infura.io/v3")]
    pub ethereum_endpoint: String,
    /// Ethereum lockdrop smart contract address.
    #[structopt(long, default_value = "EEd84A89675342fB04faFE06F7BB176fE35Cb168")]
    pub lockdrop_contract: String,
    /// Ethereum minimum transaction confirmations.
    #[structopt(long, default_value = "10")]
    pub safe_eth_confirmations: u64,
}
