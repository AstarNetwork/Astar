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
    formerStakedEra: 'EraIndex',
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
* `ContractClaimed(SmartContract, AccountId, EraIndex, EraIndex):` The contract's reward have been claimed, by an account, from era, until era



---
## Errors
* `StakingWithNoValue` Can not stake with zero value.
* `InsufficientStakingValue`, Can not stake with value less than minimum staking value.
* `MaxNumberOfStakersExceeded`, Number of stakers per contract exceeded.
* `NotOperatedContract`, Targets must be operated contracts
* `NotStakedContract`, Contract isn't staked.
* `UnstakingWithNoValue`, Unstaking a contract with zero value.
* `AlreadyRegisteredContract`, The contract is already registered by other account.
* `ContractIsNotValid`, User attempts to register with address which is not contract.
* `ContractNotRegistered`, Contract not registered for dapps staking.
* `AlreadyUsedDeveloperAccount`, This account was already used to register contract.
* `NotOwnedContract`, Contract not owned by the account.
* `UnexpectedState`, Unexpected state error, used to abort transaction. Used for situations that 'should never happen'. Report issue on github if this is ever emitted.
* `UnknownStartStakingData`, Report issue on github if this is ever emitted.
* `UnknownEraReward`, Report issue on github if this is ever emitted.
* `NothingToClaim`, There are no funds to reward the contract. Or already claimed in that era.
* `AlreadyClaimedInThisEra`, Contract already claimed in this era and reward is distributed.
* `RequiredContractPreApproval`, To register a contract, pre-approval is needed for this address.
* `AlreadyPreApprovedContract`, Contract is already part of pre-appruved list of contracts.
* `ContractRewardsNotClaimed`, Attempting to unregister contract which has unclaimed rewards. Claim them first before unregistering.

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
* AlreadyUsedDeveloperAccount
* ContractIsNotValid
* RequiredContractPreApproval

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
* InsufficientStakingValue

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
    _origin: OriginFor<T>,
    contract_id: T::AccountId,
) -> DispatchResultWithPostInfo {}
```
1. Any account can initiate this call. 
1. All stakers and the developer of this contract_id will be paid out.
1. The rewards are paid out, they are transferable and they are NOT automatically re-staked.
2. if an era for a contract is CurrentEra-ContractLastClaimed >= HistoryDepth, then all unclaimed rewards for that contract shall be sent to Treasury

Event:
`ContractClaimed(
                contract_id,
                claimer,
                start_from_era,
                current_era,
            )`

Error:
* ContractNotRegistered
* NothingToClaim
* AlreadyClaimedInThisEra

---
## Storage
* `Ledger = StorageMap( key:AccountId, value:Balance)`: Bonded amount for the staker
* `HistoryDepth = StorageValue( u32 )`: Number of eras to keep in history.
* `CurrentEra = StorageValue( EraIndex )`: The current era index.
* `BlockRewardAccumulator = StorageValue( Balance )`: Accumulator for block rewards during an era. It is reset at every new era.
* `RegisteredDevelopers = StorageMap( key:AccountId, value:SmartContract )`: Registered developer accounts points to coresponding contract.
* `RegisteredDapps = StorageMap( key:SmartContract, value:AccountId )`: Registered dapp points to the developer who registered it.
* `EraRewardsAndStakes = StorageMap( key:EraIndex, value:EraRewardAndStake)`: Total block rewards for the pallet per era and total staked funds.
* `RewardsClaimed = StorageDoubleMap( key1:SmartContract, key2:AccountId, value:Balance )`: Reward counter for individual stakers and the developer.
* `ContractEraStake = StorageDoubleMap( key1: SmartContract, key2:EraIndex, value:EraStakingPoints )`: Stores amount staked and stakers for a contract per era.
* `ContractLastClaimed = StorageMap( key:SmartContract, value:EraIndex )`: Marks an Era when a contract is last claimed.
* `ContractLastStaked = StorageMap( key:SmartContract, value:EraIndex )`: Marks an Era when a contract is last (un)staked.

---
## Referent API implementation
https://github.com/PlasmNetwork/astar-apps
