# dApp Staking v3

## Introduction

Astar and Shiden networks provide a unique way for developers to earn rewards by developing products that native token holders decide to support.

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

Stakes are **only** valid throughout a period. When new period starts, all stakes are reset to **zero**. This helps prevent projects remaining staked due to intertia of stakers, and makes for a more dynamic staking system. Staker doesn't need to do anything for this to happen, it is automatic.

Even though stakes are reset, locks (or freezes) of tokens remain.

#### Voting

When `Voting` subperiod starts, all _stakes_ are reset to **zero**.
Projects participating in dApp staking are expected to market themselves to (re)attract stakers.

Stakers must assess whether the project they want to stake on brings value to the ecosystem, and then `vote` for it.
Casting a vote, or staking, during the `Voting` subperiod makes the staker eligible for bonus rewards. so they are encouraged to participate.

`Voting` subperiod length is expressed in _standard_ era lengths, even though the entire voting subperiod is treated as a single _voting era_.
E.g. if `voting` subperiod lasts for **5 eras**, and each era lasts for **100** blocks, total length of the `voting` subperiod will be **500** blocks.
* Block 1, Era 1 starts, Period 1 starts, `Voting` subperiod starts
* Block 501, Era 2 starts, Period 1 continues, `Build&Earn` subperiod starts

Neither stakers nor dApps earn rewards during this subperiod - no new rewards are generated after `voting` subperiod ends.

#### Build&Earn

`Build&Earn` subperiod consits of one or more eras, therefore its length is expressed in eras.

After each _era_ ends, eligible stakers and dApps can claim the rewards they earned. Rewards are **only** claimable for the finished eras.

It is still possible to _stake_ during this period, and stakers are encouraged to do so since this will increase the rewards they earn.
The only exemption is the **final era** of the `build&earn` subperiod - it's not possible to _stake_ then since the stake would be invalid anyhow (stake is only valid from the next era which would be in the next period).

To continue the previous example where era length is **100** blocks, let's assume that `Build&Earn` subperiod lasts for 10 eras:
* Block 1, Era 1 starts, Period 1 starts, `Voting` subperiod starts
* Block 501, Era 2 starts, Period 1 continues, `Build&Earn` subperiod starts
* Block 601, Era 3 starts, Period 1 continues, `Build&Earn` subperiod continues
* Block 701, Era 4 starts, Period 1 continues, `Build&Earn` subperiod continues
* ...
* Block 1401, Era 11 starts, Period 1 continues, `Build&Earn` subperiod enters the final era
* Block 1501, Era 12 starts, Period 2 starts, `Voting` subperiod starts
* Block 2001, Era 13 starts, Period 2 continues, `Build&Earn` subperiod starts

### dApps & Smart Contracts

Protocol is called dApp staking, but internally it essentially works with smart contracts, or even more precise, smart contract addresses.

Throughout the code, when addressing a particular dApp, it's addressed as `smart contract`. Naming of the types & storage more closely follows `dApp` nomenclature.

#### Registration

Projects, or _dApps_, must be registered into protocol to participate.
Only a privileged `ManagerOrigin` can perform dApp registration.
The pallet itself does not make assumptions who the privileged origin is, and it can differ from runtime to runtime.

Once dApp has been registered, stakers can stake on it immediatelly.

When contract is registered, it is assigned a unique compact numeric Id - 16 bit unsigned integer. This is important for the inner workings of the pallet, and is not directly exposed to the users.

There is a limit of how many smart contracts can be registered at once. Once the limit is reached, any additional attempt to register a new contract will fail.

#### Reward Beneficiary & Ownership

After a dApp has been registered, it is possible to modify reward beneficiary or even the owner of the dApp. The owner can perform reward delegation and can further transfer ownership.

#### Unregistration

dApp can be removed from the procotol by unregistering it.
This is a privileged action that only `ManagerOrigin` can perform.

After a dApp has been unregistered, it's no longer eligible to receive rewards.
It's still possible however to claim past unclaimed rewards.

Important to note that even if dApp has been unregistered, it still occupies a _slot_
in the dApp staking protocol and counts towards maximum number of registered dApps.
This will be improved in the future when dApp data will be cleaned up after some time.

### Stakers

#### Locking Tokens

In order for users to participate in dApp staking, the first step they need to take is lock (or freeze) some native currency. Reserved tokens cannot be locked, but tokens locked by another lock can be re-locked into dApp staking (double locked).

**NOTE:** Locked funds cannot be used for paying fees, or for transfer.

In order to participate, user must have a `MinimumLockedAmount` of native currency locked. This doesn't mean that they cannot lock _less_ in a single call, but total locked amount must always be equal or greater than `MinimumLockedAmount`.

In case amount specified for locking is greater than what user has available, only what's available will be locked.

#### Unlocking Tokens

User can at any time decide to unlock their tokens. However, it's not possible to unlock tokens which are staked, so user has to unstake them first.

Once _unlock_ is successfully executed, the tokens aren't immediately unlocked, but instead must undergo the unlocking process. Once unlocking process has finished, user can _claim_ their unlocked tokens into their free balance.

There is a limited number of `unlocking chunks` a user can have at any point in time. If limit is reached, user must claim existing unlocked chunks, or wait for them to be unlocked before claiming them to free up space for new chunks.

In case calling unlocking some amount would take the user below the `MinimumLockedAmount`, **everything** will be unlocked.

For users who decide they would rather re-lock their tokens then wait for the unlocking process to finish, there's an option to do so. All currently unlocking chunks are consumed, and added back into locked amount.

#### Staking Tokens

Locked tokens, which aren't being used for staking, can be used to stake on a dApp. This translates to _voting_ or _nominating_ a dApp to receive rewards derived from the inflation. User can stake on multiple dApps if they want to.

The staked amount **must be precise**, no adjustment will be made by the pallet in case a too large amount is specified.

The staked amount is only eligible for rewards from the next era - in other words, only the amount that has been staked for the entire era is eligible to receive rewards.

It is not possible to stake if there are unclaimed rewards from past eras. User must ensure to first claim their pending rewards, before staking. This is also beneficial to the users since it allows them to lock & stake the earned rewards as well.

User's stake on a contract must be equal or greater than the `MinimumStakeAmount`. This is similar to the minimum lock amount, but this limit is per contract.

Although user can stake on multiple smart contracts, the amount is limited. To be more precise, amount of database entries that can exist per user is limited.

The protocol keeps track of how much was staked by the user in `voting` and `build&earn` subperiod. This is important for the bonus reward calculation.

It is not possible to stake on a dApp that has been unregistered.
However, if dApp is unregistered after user has staked on it, user will keep earning
rewards for the staked amount.

#### Unstaking Tokens

User can at any time decide to unstake staked tokens. There's no _unstaking_ process associated with this action.

Unlike stake operation, which stakes from the _next_ era, unstake will reduce the staked amount for the current and next era if stake exists.

Same as with stake operation, it's not possible to unstake anything until unclaimed rewards have been claimed. User must ensure to first claim all rewards, before attempting to unstake. Unstake amount must also be precise as no adjustment will be done to the amount.

The amount unstaked will always first reduce the amount staked in the ongoing subperiod. E.g. if `voting` subperiod has stake of **100**, and `build&earn` subperiod has stake of **50**, calling unstake with amount **70** during `build&earn` subperiod will see `build&earn` stake amount reduced to **zero**, while `voting` stake will be reduced to **80**.

If unstake would reduce the staked amount below `MinimumStakeAmount`, everything is unstaked.

Once period finishes, all stakes are reset back to zero. This means that no unstake operation is needed after period ends to _unstake_ funds - it's done automatically.

If dApp has been unregistered, a special operation to unstake from unregistered contract must be used.

#### Claiming Staker Rewards

Stakers can claim rewards for passed eras during which they were staking. Even if multiple contracts were staked, claim reward call will claim rewards for all of them.

Only rewards for passed eras can be claimed. It is possible that a successful reward claim call will claim rewards for multiple eras. This can happen if staker hasn't claimed rewards in some time, and many eras have passed since then, accumulating pending rewards.

To achieve this, the pallet's underyling storage organizes **era reward information** into **spans**. A single span covers multiple eras, e.g. from **1** to **16**. In case user has staked during era 1, and hasn't claimed rewards until era 17, they will be eligible to claim 15 rewards in total (from era 2 to 16). All of this will be done in a single claim reward call.

In case unclaimed history has built up past one span, multiple reward claim calls will be needed to claim all of the rewards.

Rewards don't remain available forever, and if not claimed within some time period, they will be treated as expired. This will be a longer period, but will still exist.

Rewards are calculated using a simple formula: `staker_reward_pool * staker_staked_amount / total_staked_amount`.

#### Claiming Bonus Reward

If staker staked on a dApp during the voting subperiod, and didn't reduce their staked amount below what was staked at the end of the voting subperiod, this makes them eligible for the bonus reward.

Bonus rewards need to be claimed per contract, unlike staker rewards.

Bonus reward is calculated using a simple formula: `bonus_reward_pool * staker_voting_subperiod_stake / total_voting_subperiod_stake`.

#### Handling Expired Entries

There is a limit to how much contracts can a staker stake on at once.
Or to be more precise, for how many contract a database entry can exist at once.

It's possible that stakers get themselves into a situation where some number of expired database entries associated to
their account has accumulated. In that case, it's required to call a special extrinsic to cleanup these expired entries.

### Developers

Main thing for developers to do is develop a good product & attract stakers to stake on them.

#### Claiming dApp Reward

If at the end of an build&earn subperiod era dApp has high enough score to enter a tier, it gets rewards assigned to it.
Rewards aren't paid out automatically but must be claimed instead, similar to staker & bonus rewards.

When dApp reward is being claimed, both smart contract & claim era must be specified.

dApp reward is calculated based on the tier in which ended. All dApps that end up in one tier will get the exact same reward.

### Tier System

At the end of each build&earn subperiod era, dApps are evaluated using a simple metric - total value staked on them.
Based on this metric, they are sorted, and assigned to tiers.

There is a limited number of tiers, and each tier has a limited capacity of slots.
Each tier also has a _threshold_ which a dApp must satisfy in order to enter it.

Better tiers bring bigger rewards, so dApps are encouraged to compete for higher tiers and attract staker's support.
For each tier, the reward pool and capacity are fixed. Each dApp within a tier always gets the same amount of reward.
Even if tier capacity hasn't been fully taken, rewards are paid out as if they were.

For example, if tier 1 has capacity for 10 dApps, and reward pool is **500 ASTR**, it means that each dApp that ends up
in this tier will earn **50 ASTR**. Even if only 3 dApps manage to enter this tier, they will still earn each **50 ASTR**.
The rest, **350 ASTR** in this case, won't be minted (or will be _burned_ if the reader prefers such explanation).

If there are more dApps eligible for a tier than there is capacity, the dApps with the higher score get the advantage.
dApps which missed out get priority for entry into the next lower tier (if there still is any).

In the case a dApp doesn't satisfy the entry threshold for any tier, even though there is still capacity, the dApp will simply
be left out of tiers and won't earn **any** reward.

In a special and unlikely case that two or more dApps have the exact same score and satisfy tier entry threshold, but there isn't enough
leftover tier capacity to accomodate them all, this is considered _undefined_ behavior. Some of the dApps will manage to enter the tier, while
others will be left out. There is no strict rule which defines this behavior - instead dApps are encouraged to ensure their tier entry by
having a larger stake than the other dApp(s). Tehnically, at the moment, the dApp with the lower `dApp Id` will have the advantage over a dApp with
the larger Id.

### Reward Expiry

Unclaimed rewards aren't kept indefinitely in storage. Eventually, they expire.
Stakers & developers should make sure they claim those rewards before this happens.

In case they don't, they will simply miss on the earnings.

However, this should not be a problem given how the system is designed.
There is no longer _stake&forger_ - users are expected to revisit dApp staking at least at the
beginning of each new period to pick out old or new dApps on which to stake on.
If they don't do that, they miss out on the bonus reward & won't earn staker rewards.