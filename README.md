<div align="center"><h1>Plasm</h1></div>

<div align="center"><img width="300" alt="plasm" src="https://user-images.githubusercontent.com/6259384/55708398-cf9ae900-5a20-11e9-859c-3435b55c68a5.png"></div>

Plasm is a Substrate Runtime Module Library by which a developer can add Plasma functions to his/her own Substrate chain.

__WARNING__: This is a proof-of-concept prototype. This implementation is NOT ready for production use. 

## Table of Contents
- [Introduction](https://github.com/stakedtechnologies/Plasm/tree/sota#introduction)
- [Background](https://github.com/stakedtechnologies/Plasm/tree/sota#background)
- Plasm
    - [Plasm-UTXO](https://github.com/stakedtechnologies/Plasm/tree/sota#plasm-utxo)
    - [Plasm-Parent](https://github.com/stakedtechnologies/Plasm/tree/sota#plasm-parent)
    - [Plasm-Child](https://github.com/stakedtechnologies/Plasm/tree/sota#plasm-child)
- [How to install](https://github.com/stakedtechnologies/Plasm/tree/sota#how-to-install)

## Introduction
Plasm is Staked Technologies' product that enables to import Plasma functions to your Substrate chain.

## Background
Today, there are many derived Plasmas, like 

- Plasma-MVP: Proposed by Vitalik Buterin.
- Plasma-Cash: Users only need to download the histories of and watch the tokens they want to track.
- Plasma-XT: Plasma-Cash derivative.
- Plasma-Prime: Plasma-Cash derivative.
- Plasma-Chamber: Cryptoeconomics Lab's opensource project inspired by Prime. 
- Plasma-Snapps: implemented ZK-S[T|N]ARKs

Plasm has Plasma-abstract data structures by which the user can custormize plagable Plasma solutions. In addtion to that, it has the Rust implementations of Plasma solutions. 

Substrate developers can import one of Plasm Libraries and make thier own plasma chain depending on their use casse. Plasm consists of 3 (or 4) libraries, Plasm-UTXO, Plasm-Parent and Plasm-Child. Plasm-UTXO has a UTXO like data structure to manage the deposited tokens. 

In same sence, Plasma needs to have all transactions in order to validate and declare a maricious transaction when it is exited to the parent chain. 

- Plasm-UTXO: Abstracted UTXO model and concreted UTXO model for each Plasma solution are implemented.
- Plasm-Parent: Plasm-Parent provides modules to make a parent chain.  
- Prasm-Child: Plasma-Child provides modules to make a child chain.


## Plasm-UTXO
Plasm-UTXO provides a specification of transactions which is suitable for each Plasma solution. Along with that, Plasm-UTXO can deal with UTXO-like data structures cyclopaedically. Merkle Tree are also removable.


## Plasm-Parent
Plasm-Parent provides a specification of the parent chain. Child chain has been implemented coresponding to thhe parent chain's solution. Mainly, Plasm-Parent has the logic of each exit game.


## Plasm-Child
Plasm-Child provides a specification of the child chain. Parent chain has been implemented corresponding to the child chain's solutions. 


By using these solutions together, the user can make the transactions happen between the parent chain and the child chain. The logic of "deposit/exit" has been implemented based on Plasm-UTXO.

## How to install

## UTXO
```toml
[dependencies.utxo]
git = 'https://github.com/stakedtechnologies/Plasm.git'
package = 'plasm-utxo'
version = '0.1.0' 
```

## Parent
Comming soon...

## Child
Comming soon...

* * *
Plasm is licensed under the Apache License, Version2.0 by Staked Technologies Inc.
