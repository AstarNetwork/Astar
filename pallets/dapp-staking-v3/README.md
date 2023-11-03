# dApp Staking v3

## Introduction

Astar and Shiden networks provide a unique way for developers to earn rewards by developing products that native token holdes decide to support.

The principle is simple - stakers lock their tokens to _stake_ on a dApp, and if the dApp attracts enough support, it is rewarded in native currency, derived from the inflation.
In turn stakers are rewarded for locking & staking their tokens.

## Functionality Overview

### Eras

Eras are the basic _time unit_ in dApp staking and their length is measured in the number of blocks.

They are not expected to last long, e.g. current live networks era length is roughly 1 day (7200 blocks).
After an era ends, it's usually possible to claim rewards for it, if user or dApp are eligible.

### Periods

Periods are another _time unit_ in dApp staking. They are expected to be more lengthy than eras.

Each period consists of two subperiods:
* `Voting`
* `Build&Earn`

Each period is denoted by a number, which increments each time a new period begins.
Period beginning is marked by the `voting` subperiod, after which follows the `build&earn` period.

#### Voting

When `Voting` starts, all _stakes_ are reset to **zero**.
Projects participating in dApp staking are expected to market themselves to (re)attract stakers.

Stakers must assess whether the project they want to stake on brings value to the ecosystem, and then `vote` for it.
Casting a vote, or staking, during the `Voting` subperiod makes the staker eligible for bonus rewards. so they are encouraged to participate.

`Voting` subperiod length is expressed in _standard_ era lengths, even though the entire voting period is treated as a single _voting era_.
E.g. if `voting` subperiod lasts for **10 eras**, and each era lasts for **100** blocks, total length of the `voting` subperiod will be **1000** blocks.

Neither stakers nor dApps earn rewards during this subperiod - no new rewards are generated after `voting` subperiod ends.

#### Build&Earn

`Build&Earn` subperiod consits of one or more eras, therefore its length is expressed in eras.

After each _era_ ends, eligible stakers and dApps can claim the rewards they earned. Rewards are **only** claimable for the finished eras.

It is still possible to _stake_ during this period, and stakers are encouraged to do so since this will increase the rewards they earn.
The only exemption is the **final era** of the `build&earn` subperiod - it's not possible to _stake_ then since the stake would be invalid anyhow (stake is only valid from the next era which would be in the next period).

### dApps & Smart Contracts

Protocol is called dApp staking, but internally it essentially works with smart contracts, or even more precise, smart contract addresses.

Throughout the code, when addressing a particular dApp, it's addressed as `smart contract`. Naming of the types & storage more closely follows `dApp` nomenclature.

#### Registration

Projects, or _dApps_, must be registered into protocol to participate.
Only a privileged `ManagerOrigin` can perform dApp registration.
The pallet itself does not make assumptions who the privileged origin is, and it can differ from runtime to runtime.

There is a limit of how many smart contracts can be registered at once. Once the limit is reached, any additional attempt to register a new contract will fail.

#### Reward Beneficiary & Ownership

When contract is registered, it is assigned a unique compact numeric Id - 16 bit unsigned integer. This is important for the inner workings of the pallet, and is not directly exposed to the users.

After a dApp has been registered, it is possible to modify reward beneficiary or even the owner of the dApp. The owner can perform reward delegation and can further transfer ownership.

#### Unregistration

dApp can be removed from the procotol by unregistering it.
This is a privileged action that only `ManagerOrigin` can perform.

After a dApp has been unregistered, it's no longer eligible to receive rewards.
It's still possible however to claim past unclaimed rewards.

Important to note that even if dApp has been unregistered, it still occupies a _slot_
in the dApp staking protocol and counts towards maximum number of registered dApps.
This will be improved in the future when dApp data will be cleaned up after the period ends.

### Stakers

#### Locking Funds

In order for users to participate in dApp staking, the first step they need to take is lock some native currency. Reserved tokens cannot be locked, but tokens locked by another lock can be re-locked into dApp staking (double locked).

**NOTE:** Locked funds cannot be used for paying fees, or for transfer.

In order to participate, user must have a `MinimumLockedAmount` of native currency locked. This doesn't mean that they cannot lock _less_ in a single call, but total locked amount must always be equal or greater than `MinimumLockedAmount`.

In case amount specified for locking is greater than what user has available, only what's available will be locked.

#### Unlocking Funds

User can at any time decide to unlock their tokens. However, it's not possible to unlock tokens which are staked, so user has to unstake them first.

Once _unlock_ is successfully executed, the tokens aren't immediately unlocked, but instead must undergo the unlocking process. Once unlocking process has finished, user can _claim_ their unlocked tokens into their free balance.

There is a limited number of `unlocking chunks` a user can have at any point in time. If limit is reached, user must claim existing unlocked chunks, or wait for them to be unlocked before claiming them to free up space for new chunks.

In case calling unlocking some amount would take the user below the `MinimumLockedAmount`, **everything** will be unlocked.

For users who decide they would rather re-lock their tokens then wait for the unlocking process to finish, there's an option to do so. All currently unlocking chunks are consumed, and added back into locked amount.