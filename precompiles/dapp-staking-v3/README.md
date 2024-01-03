# dApp Staking Precompile Interface

dApp Staking is at the core of Astar Network, an unique protocol used to incentivize builders to build
great dApps on Astar.

In order to improve it, the feature has undergone several overhauls in past.
This is necessary to keep Astar competitive, and it's likely it will happen again in the future.

Developers should account for this when developing dApps.
Even though the interface compatibility will be kept as much as possible,
as feature is changed & evolved, new functionality will become available, and
interface will have to be expanded.

The logic controlling dApp staking should be upgradable therefore.

## V2 Interface

**ONLY RELEVANT FOR DEVELOPERS WHO USED THE OLD INTERFACE.**

Covers the _so-called_ `dApps staking v2` interface.

Many actions that are done here as a part of a single call are broken down into multiple calls in the `v3` interface.
Best effort is made to keep the new behavior as compatible as possible with the old behavior.
Regardless, in some cases it's simply not possible.

Some examples of this:
* Since all stakes are reset at the end of each period, developers will need to adapt their smart contract logic for this.
* Bonus rewards concept was only introduced from the precompile v3 interface, so there's no equivalent call in v2 interface.
* Composite actions like `bond_and_stake`, `unbond_and_unstake` and `nomination_transfer` are implemented as a series of calls to mimic the old logic.
* Claiming staker rewards is detached from a specific staked dApp (or smart contract), and can result in more than 1 era reward being claimed.
* Periods & subperiods concept only exists from the v3 interface.

## V3 Interface

Contains functions that _mimic_ the interface of the latest `dApp Staking v3`.
Developers are encouraged to use this interface to fully utilize dApp staking functionality.