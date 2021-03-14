# Plasm Contract Operator


## Overview

FRAME pallet to manage operator developing smart contracts [`more (??? where)...`](https://docs.plasmnet.io)

- [`operator::Trait`](./trait.Trait.html)
- [`Call`](./enum.Call.html)
- [`Module`](./struct.Module.html)

## Interface

### Dispatchable Functions

* `instantiate` - Deploys a contact and insert relation of a contract and an operator to mapping
* `update_parameters` - Updates parameters for an identified contact
* `change_operator` - Changes an operator for identified contracts

### Traits
* `ContractFinder`
* `OperatorFinder`

## Storage 

* `OperatorHasContracts` - A mapping from operators to operated contracts by them
    * map `T::AccountId => Vec<T::AccountId>`
* `ContractHasOperator` - A mapping from operated contract by operator to it
    * map `T::AccountId => Option<T::AccountId>`
* `ContractParameters` - A mapping from contract to it's parameters
    * map `T::AccountId => Option<T::Parameters>`

## Related Modules

- [Plasm-Support](../plasm-support/README.md): The Plasm helper module.



* * *

Plasm is licensed under the GPLv3.0 by Stake Technologies Inc.