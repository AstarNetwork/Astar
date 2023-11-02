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

After each _era_ end, eligible stakers and dApps can claim the rewards they earned. Rewards are only claimable for the finished eras.

It is still possible to _stake_ during this period, and stakers are encouraged to do so since this will increase the rewards they earn.
The only exemption is the **final era** of the `build&earn` subperiod - it's not possible to _stake_ then since the stake would be invalid anyhow (stake is only valid from the next era).

