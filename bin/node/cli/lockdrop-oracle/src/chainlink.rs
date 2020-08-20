use web3::contract::{Contract, Options};
///! Chainlink smart contract based price oracle.
use web3::futures::Future;
use web3::types::Address;

const ETHUSD: &str = "5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";
const BTCUSD: &str = "F4030086522a5bEEa4988F8cA5B36dbC97BeE88c";

pub fn query(endpoint: &str, contract: Address) -> u128 {
    let (_eloop, transport) = web3::transports::Http::new(endpoint).unwrap();
    let web3 = web3::Web3::new(transport);
    let contract = Contract::from_json(
        web3.eth(),
        contract,
        include_bytes!("../abis/chainlink.json"),
    )
    .unwrap();
    contract
        .query("latestAnswer", (), None, Options::default(), None)
        .wait()
        .unwrap()
}

pub fn eth_usd(endpoint: &str) -> u128 {
    query(endpoint, ETHUSD.parse().unwrap())
}

pub fn btc_usd(endpoint: &str) -> u128 {
    query(endpoint, BTCUSD.parse().unwrap())
}
