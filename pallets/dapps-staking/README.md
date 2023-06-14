# Pallet dapps-staking RPC API
This document describes the interface for the pallet-dapps-staking.

Table of Contents:
1. [Terminology](#Terminology)
2. [Referent implementatio](#Referent)
3. [FAQ](#FAQ)

## Terminology
### Actors in dApps Staking

- `developer`: a developer or organization who deploys the smart contract
- `staker`: any Astar user who stakes tokens on the developer's smart contract


### Abbreviations and Terminology
- `dApp`: decentralized application, is an application that runs on a distributed network.
- `smart contract`: on-chain part of the dApp
- `contract`: short for smart contract
- `EVM`: Ethereum Virtual Machine. Solidity Smart contract runs on it.
- `ink!`: Smart Contract written in Rust, compiled to WASM.
- `era`: Period of time. After it ends, rewards can be claimed. It is defined by the number of produced blocks. Duration of an era for this pallet is configurable. The exact duration depends on block production duration.
- `claim`: Claim ownership of the rewards from the contract's reward pool.
- `bond`: Freeze funds to gain rewards.
- `stake`: In this pallet a staker stakes bonded funds on a smart contract .
- `unstake`: Unfreeze bonded funds and stop gaining rewards.
- `wasm`: Web Assembly.
- `contracts's reward pool`: Sum of unclaimed rewards on the contract. Including developer and staker parts.

---

---
## Referent API implementation
https://github.com/AstarNetwork/astar-apps

---
## FAQ

### Does it matter which project I stake on?
It matters because this means you're supporting that project.
Project reward is calculated based on how much stakers have staked on that project.
You want to support good projects which bring value to the ecosystem since that will make
the ecosystem more valuable, increasing the value of your tokens as a result.

Use the power you have and make sure to stake on projects you support and find beneficial.

### Does my reward depend on the project I stake on?
No, the reward you get only depends on the total amount you have staked, invariant of dapp(s) on which you staked.
This allows you to select the dapp you like and want to support, without having to worry if you'll be earning less rewards than you
would if you staked on another dapp.

### When do the projects/developers get their rewards?
Rewards will be deposited to beneficiaries once either `claim_staker` or `claim_dapp` is called.
We advise users to use our official portal for claiming rewards since the complexity of the protocol is hidden there.

### What happens if nobody calls the claim function for a long time?
At the moment, there is no history depth limit and your reward will be waiting for you.
However, this will be changed in the future.

### When developers register their dApp, which has no contract yet, what kind of address do they need to input?
There has to be a contract. Registration can’t be done without the contract.

### Can projects/developers change contract address once it is registered for dApps staking?
The contract address can't be changed for the dApps staking. However, if the project needs to deploy new version of the contract, they can still use old (registered) contract address for dApp staking purposes.

### How do projects/developers (who joins dApps staking) get their stakers' address and the amount staked?
`GeneralStakerInfo` storage item can be checked.
This would require developer to fetch all values from the map and find the ones where second key equals that of the contract they are interested in.
If the last staked value is greater than `Zero`, it means staker (first key) is staking on that contract.

### What is the maximum numbers of stakers per dapps?
Please check in the source code constant `MaxNumberOfStakersPerContract`.

### What is the minimum numbers of stakers per dapps?
Please check in the source code constant `MinimumStakingAmount`.

### When developers register their dApp, can they registar WASM contract? (If not, can they update it in the future?)
The developers can register several dApps. But they need to use separate accounts and separate contract addresses.
The rule is

```1 developer <=> 1 contract```

### Does dApps staking supports Wasm contracts?
Yes.
Once the Wasm contracts are enabled on a parachain, Wasm contract could be used for dApps staking.
