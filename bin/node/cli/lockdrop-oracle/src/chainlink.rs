use web3::contract::{Contract, Options};
///! Chainlink smart contract based price oracle.
use web3::futures::Future;
use web3::types::Address;

const ETHUSD: &str = "F79D6aFBb6dA890132F9D7c355e3015f15F3406F";
const BTCUSD: &str = "F5fff180082d6017036B771bA883025c654BC935";

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
