# Pallet dapps-staking RPC API
This document describes the interface for the pallet-dapps-staking.

Table of Contents:
1. [Terminology](#Terminology)
1. [Types](#Types)
1. [Events](#Events)
1. [Errors](#Errors)
1. [Calls](#Calls)
1. [Storage](#Storage)
1. [Referent implementatio](#Referent)
1. [FAQ](#FAQ)

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
- `era`: Period of time. After it ends, rewards can be claimed. It is defined by the number of produced blocks. Duration of an era for this pallet is around 1 day. The exact duration depends on block production duration.
- `claim`: Claim ownership of the rewards from the contract's reward pool.
- `bond`: Freeze funds to gain rewards.
- `stake`: In this pallet a staker stakes bonded funds on a smart contract .
- `unstake`: Unfreeze bonded funds and stop gaining rewards.
- `wasm`: Web Assembly.
- `contracts's reward pool`: Sum of unclaimed rewards on the contract. Including developer and staker parts.


---

## Types
### SmartContract

```
SmartContract: {
    _enum: {
    Evm: 'H160',
    Wasm: 'AccountId'
    },
}
```
### EraIndex

`EraIndex: 'u32'`

### EraStakingPoints

```
EraStakingPoints: {
    total: 'Balance',
    stakers: 'BTreeMap<AccountId, Balance>',
    _formerStakedEra: 'EraIndex',
    claimedRewards: 'Balance'
}
```
### EraRewardAndStake
```
EraRewardAndStake {
    rewards: 'Balance',
    staked: 'Balance'
}
```



---
## Events

* `BondAndStake(AccountId, SmartContract, Balance):` Account has bonded and staked funds on a smart contract.
* `UnbondUnstakeAndWithdraw(AccountId, SmartContract, Balance):` Account has unbonded, unstaked and withdrawn funds.
* `NewContract(AccountId, SmartContract):` New contract added for staking.
* `ContractRemoved(AccountId, SmartContract):` Contract removed from dapps staking.
* `NewDappStakingEra(EraIndex):` New dapps staking era. Distribute era rewards to contracts.
* `ContractClaimed(SmartContract, EraIndex, Balance):` The contract's reward has been claimed for an era
* `Reward(AccountId, SmartContract, EraIndex, Balance):` Reward paid to staker or developer.


---
## Errors
* `StakingWithNoValue` Can not stake with zero value.
* `InsufficientValue`, Can not stake with value less than minimum staking value.
* `MaxNumberOfStakersExceeded`, Number of stakers per contract exceeded.
* `NotOperatedContract`, Targets must be operated contracts
* `NotStakedContract`, Contract isn't staked.
* `UnstakingWithNoValue`, Unstaking a contract with zero value.
* `AlreadyRegisteredContract`, The contract is already registered by other account.
* `ContractIsNotValid`, User attempts to register with address which is not contract.
* `AlreadyUsedDeveloperAccount`, This account was already used to register contract.
* `NotOwnedContract`, Contract not owned by the account.
* `UnknownEraReward`, Report issue on github if this is ever emitted.
* `NotStaked`, Contract hasn't been staked on in this era.
* `AlreadyClaimedInThisEra`, Contract already claimed in this era and reward is distributed.
* `EraOutOfBounds`, Era parameter is out of bounds.
* `RequiredContractPreApproval`, To register a contract, pre-approval is needed for this address.
* `AlreadyPreApprovedDeveloper`, Developer's account is already part of pre-approved list.

---
## Calls
### Register
`register(origin: OriginFor<T>, contract_id: T::AccountId) -> DispatchResult {}`
1. Registers contract as a staking target.
1. The dispatch origin for this call must be _Signed_ by the developers's account.
3. Prior to registering, a contract needs to be deployed on the network. The contract address where the contract is deployed is used as the argument in this call.
4. The `dapps-staking` pallet supports both contract types, EVM and Wasm. The Shiden Network supports only EVM at the moment.
5. The type for contract address will be `SmartContract`, which abstracts EVM and Wasm address types.
6. The Developer who is registering the contract has to reserve `RegisterDeposit`.
7. There will be a pre-approved list of developers. This pre-approval could be enabled or disabled. The pre-approval requires sudo call.

Event:
* `NewContract(developer's account, contract_id)`

Errors:
* AlreadyRegisteredContract
* AlreadyUsedDeveloperAccount
* ContractIsNotValid
* RequiredContractPreApproval

### Unregister
`register(origin: OriginFor<T>, contract_id: T::AccountId) -> DispatchResult {}`
1. Unregisters contract from dapps staking.
1. The dispatch origin for this call must be _Signed_ by the developers's account.
3. Prior to unregistering, all rewards for that contract must be claimed.
4. The`RegisterDeposit` is returned to the developer.

Event:
* `ContractRemoved(developer's account, contract_id)`

Errors:
* NotOwnedContract
* ContractIsNotValid

---
### Bonding and Staking Funds
```
pub fn bond_and_stake(
            origin: OriginFor<T>,
            contract_id: SmartContract<T::AccountId>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {}
```
1. The dispatch origin for this call must be _Signed_ by the staker's account.
2. Staked funds will be considered for reward after the end of the current era.
3. The Staker shall use one address for this call.
4. This call is used for both initial staking and for possible additional stakings.
5. The Staker shall stake on only one contract per call
6. The Staker can stake on an unlimited number of contracts but one at the time.
7. The number of stakers per contract is limited to `MaxNumberOfStakersPerContract`
8. Staking will always leave a predefined minimum transferable amount on users account.


Events:
```
BondAndStake(
                staker,
                contract_id,
                value_to_stake
            )
```

Errors:
* NotOperatedContract
* StakingWithNoValue
* MaxNumberOfStakersExceeded
* InsufficientValue

---
### Unbonding, Unstaking and Funds Withdrawal
```
pub fn unbond_unstake_and_withdraw(
    origin: OriginFor<T>,
    contract_id: SmartContract<T::AccountId>,
    value: BalanceOf<T>,
) -> DispatchResultWithPostInfo {}
```
1. The dispatch origin for this call must be _Signed_ by the staker.
2. The unbonded funds shall be available for withdrawal after `UnbondingDuration` of eras.

:::info
:bulb: **info:** initially unbonding will be immediate
`UnbondingDuration = 0 EraIndex`
:::

Events:
`UnbondUnstakeAndWithdraw(
                staker,
                contract_id,
                value_to_unstake
            )`

Errors:
* NotOperatedContract
* UnstakingWithNoValue
* NotStakedContract

---
### Claim Rewards
```
pub fn claim(
    origin: OriginFor<T>,
    contract_id: T::SmartContract,
    era: EraIndex,
) -> DispatchResultWithPostInfo {}
```
1. Any account can initiate this call.
1. All stakers and the developer of this contract_id will be paid out.
1. The rewards are paid out, they are transferable and they are NOT automatically re-staked.
1. If an era for a contract is out of bounds `[CurrentEra - HistoryDepth, CurrentEra-1]` then error `EraOutOfBounds` is emitted
1. The event `Reward` shall be emitted for each staker in this era and for the developer
1. The event `ContractClaimed` shall be emitted after all stakers and the developer are paid out for this era.

Event:
`ContractClaimed(
                contract_id,
                claimer,
                era,
            )`

Error:
* NothingToClaim
* AlreadyClaimedInThisEra
* EraOutOfBounds
* Reward
* ContractClaimed

---
## Storage
* `Ledger = StorageMap( key:AccountId, value:Balance)`: Bonded amount for the staker
* `CurrentEra = StorageValue( EraIndex )`: The current era index.
* `BlockRewardAccumulator = StorageValue( Balance )`: Accumulator for block rewards during an era. It is reset at every new era.
* `RegisteredDevelopers = StorageMap( key:AccountId, value:SmartContract )`: Registered developer accounts points to coresponding contract.
* `RegisteredDapps = StorageMap( key:SmartContract, value:AccountId )`: Registered dapp points to the developer who registered it.
* `EraRewardsAndStakes = StorageMap( key:EraIndex, value:EraRewardAndStake)`: Total block rewards for the pallet per era and total staked funds.
* `ContractEraStake = StorageDoubleMap( key1: SmartContract, key2:EraIndex, value:EraStakingPoints )`: Stores amount staked and stakers for a contract per era.

---
## Referent API implementation
https://github.com/PlasmNetwork/astar-apps

---
## FAQ

### When do the projects/developers get their rewards?
The earned rewards need to be claimed by calling claim() function. Once the claim() function is called all stakers on the contract and the developer of the contract get their rewards. This function can be called from any account. Recommended is that it is called by the projects/developers on a daily or at most weekly basis.

### What happens if nobody calls the claim function for longer than 'history_depth' days?
The un-claimed rewards older than 'history_depth' days will be burnt.

### When developers register their dApp, which has no contract yet, what kind of address do they need to input?
There has to be a contract. Registration can’t be done without the contract.

### Can projects/developers change contract address once it is registered for dApps staking?
The contract address can't be changed for the dApps staking. However, if the project needs to deploy new version of the contract, they can still use old (registered) contract address for dApp staking purposes.

### How do projects/developers (who joins dApps staking) get their stakers' address and the amount staked?
```
ContractEraStake(contract_id, era).stakers
```
This will give the vector of all staker' accounts and how much they have staked.

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
