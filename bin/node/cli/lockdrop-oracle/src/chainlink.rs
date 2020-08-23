use web3::contract::{Contract, Options};
///! Chainlink smart contract based price oracle.
use web3::futures::Future;
use web3::types::Address;

const ETHUSD: &str = "5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";
const BTCUSD: &str = "F4030086522a5bEEa4988F8cA5B36dbC97BeE88c";

pub fn query<T: web3::Transport>(web3: web3::Web3<T>, contract: Address) -> u128 {
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

pub fn eth_usd<T: web3::Transport>(web3: web3::Web3<T>) -> u128 {
    query(web3, ETHUSD.parse().unwrap())
}

pub fn btc_usd<T: web3::Transport>(web3: web3::Web3<T>) -> u128 {
    query(web3, BTCUSD.parse().unwrap())
}
