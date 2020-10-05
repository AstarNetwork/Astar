///! Chainlink smart contract based price oracle.
use web3::contract::{Contract, Error, Options};

// ROPSTEN
const ETHUSD: &str = "30B5068156688f818cEa0874B580206dFe081a03";

pub async fn eth_usd<T: web3::Transport>(web3: web3::Web3<T>) -> Result<u128, Error> {
    let contract = ETHUSD.parse().expect("correct oracle address");

    let contract = Contract::from_json(
        web3.eth(),
        contract,
        include_bytes!("../abis/chainlink.json"),
    )
    .unwrap();
    contract
        .query("latestAnswer", (), None, Options::default(), None)
        .await
}
