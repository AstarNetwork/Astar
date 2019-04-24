<div align="center"><h1>Plasm</h1></div>

<div align="center"><img width="300" alt="plasm" src="https://user-images.githubusercontent.com/6259384/55708398-cf9ae900-5a20-11e9-859c-3435b55c68a5.png"></div>

[![Build Status](https://travis-ci.org/stakedtechnologies/Plasm.svg?branch=master)](https://travis-ci.org/stakedtechnologies/Plasm)

Plasm is a Substrate Runtime Module Library by which a developer can add Plasma functions to his/her own Substrate chain.

__WARNING__: This is a proof-of-concept prototype. This implementation is NOT ready for production use. 

## Table of Contents
- [Introduction](https://github.com/stakedtechnologies/Plasm/tree/master#introduction)
- [Background](https://github.com/stakedtechnologies/Plasm/tree/master#background)
- Plasm
    - [Plasm-UTXO](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-utxo)
    - [Plasm-Parent](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-parent)
    - [Plasm-Child](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-child)
- [How to install](https://github.com/stakedtechnologies/Plasm/tree/master#how-to-install)

## Introduction
Plasm is Staked Technologies' product that enables to import Plasma functions to your Substrate chain. You can see the demo from [here](https://drive.google.com/file/d/1qg6SyEDM0D_hJPsun4ykkNyH-B5W8Yi6/view?usp=sharing)

## Background
Today, there are many derived Plasmas, like 

- Plasma-MVP: Proposed by Vitalik Buterin.
- Plasma-Cash: Users only need to download the histories of and watch the tokens they want to track.
- Plasma-XT: Plasma-Cash derivative.
- Plasma-Prime: Plasma-Cash derivative.
- Plasma-Chamber: Cryptoeconomics Lab's opensource project inspired by Prime. 
- Plasma-Snapps: implemented ZK-S[T|N]ARKs

Plasm provides a Plasma-abstract data structure which is a combination of Plasma solutions. Also Plasm provides a Rust implementations of Plasma solutions.

Substrate developers can import one of Plasm Libraries and make thier own plasma chain depending on their use case. Plasm consists of 3 (or 4) libraries, Plasm-UTXO, Plasm-Parent and Plasm-Child. Plasm-UTXO has a UTXO like data structure to manage the deposited tokens. 

Plasma needs to have all transactions in order to validate and detect a maricious transaction when it is exited to the parent chain. 

- Plasm-UTXO: implements UTXO model which is abstracted and concreted for each Plasma solution.
- Plasm-Parent: provides modules to make a parent chain.  
- Prasm-Child: provides modules to make a child chain.


## Plasm-UTXO
Plasm-UTXO provides the transactions' specification which is suitable for each Plasma solution. Along with that, Plasm-UTXO can deal with UTXO-like data structures cyclopaedically. Merkle Tree are also removable.


## Plasm-Parent
Plasm-Parent provides the parent chain’s specification. Child chain has been implemented corresponding to the parent chain's solution. Mainly, Plasm-Parent has the logic of each exit game.


## Plasm-Child
Plasm-Child provides the child chain's specification. Parent chain has been implemented corresponding to the child chain's solutions.


By using these solutions together, users can make transactions between the parent chain and the child chain. The logic of "deposit/exit" has been implemented based on Plasm-UTXO.

## How to install

## UTXO
```toml
[dependencies.utxo]
git = 'https://github.com/stakedtechnologies/Plasm.git'
package = 'plasm-utxo'
version = '0.1.0' 
```

## Parent
```toml
[dependencies.parent]
git = 'https://github.com/stakedtechnologies/Plasm.git'
package = 'plasm-utxo'
version = '0.1.0' 
```

## Child
(Child will be wrapping UTXO)Comming soon...

## Example Trait
Please see [here](https://github.com/stakedtechnologies/Plasm/blob/master/runtime/src/lib.rs).

* * *
Plasm is licensed under the Apache License, Version2.0 by Staked Technologies Inc.
