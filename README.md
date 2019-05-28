<div align="center"><img width="798" alt="plasm" src="https://user-images.githubusercontent.com/6259384/56867192-8b967500-6a1d-11e9-898d-f73f4e2a387c.png"></div>

[![Build Status](https://travis-ci.org/stakedtechnologies/Plasm.svg?branch=master)](https://travis-ci.org/stakedtechnologies/Plasm)

Plasm is a Substrate Runtime Module Library which allows developers to add Plasma functions to their Substrate chain easily and seamlessly. Since Plasm is an SRML, developers can also make both plasma parent chains and plasma child chains with Substrate. 

__WARNING__: This is a proof-of-concept prototype. This implementation is NOT ready for production use. 

## Table of Contents
- [Demo](https://github.com/stakedtechnologies/Plasm/tree/master#demo)
- [Introduction](https://github.com/stakedtechnologies/Plasm/tree/master#introduction)
- [Background](https://github.com/stakedtechnologies/Plasm/tree/master#background)
- Plasm
    - [Plasm-UTXO](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-utxo)
    - [Plasm-Parent](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-parent)
    - [Plasm-Child](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-child)
- [How to install](https://github.com/stakedtechnologies/Plasm/tree/master#how-to-install)

## Demo
![plasm_demo](https://user-images.githubusercontent.com/6259384/58473625-091be500-8184-11e9-9f65-1fd986f5adc0.gif)

Demo application tutorial is [here](https://medium.com/staked-technologies/lets-make-a-plasma-chain-with-plasm-and-substrate-39cbd868022d).

## Introduction
Plasm is Staked Technologies' product that enables to import Plasma functions to your Substrate chain. Since Plasm is SRML, we can also make both Plasma parent chains and child chains. You can see the demo from [here](https://drive.google.com/file/d/1qg6SyEDM0D_hJPsun4ykkNyH-B5W8Yi6/view?usp=sharing).

## Background
Today, there are many derived Plasmas, like 

- Plasma-MVP: Proposed by Vitalik Buterin.
- Plasma-Cash: Users only need to download the histories of and watch the tokens they want to track.
- Plasma-XT: Plasma-Cash derivative.
- Plasma-Prime: Plasma-Cash derivative.
- Plasma-Chamber: Cryptoeconomics Lab's opensource project inspired by Prime. 
- Plasma-Snapps: implemented ZK-S[T|N]ARKs

Plasm provides a Plasma-abstract data structure which is a combination of Plasma solutions. Also, Plasm provides Rust implementations of Plasma solutions.

Substrate developers can import one of the Plasm Libraries and make their own plasma chain depending on their use case. Plasm consists of 3 (or 4) libraries, Plasm-UTXO, Plasm-Parent, and Plasm-Child. Plasm-UTXO has a UTXO like data structure to manage the deposited tokens. 

Plasma needs to have all transactions in order to validate and detect a malicious transaction when it is exited to the parent chain. 

- Plasm-UTXO: implements the UTXO model which is abstracted and concreted for each Plasma solution.
- Plasm-Parent: provides modules to make a parent chain.  
- Plasm-Child: provides modules to make a child chain.


## Plasm-UTXO
Plasm-UTXO provides the transactions' specification which is suitable for each Plasma solution. Along with that, Plasm-UTXO can deal with UTXO-like data structures cyclopaedically. Merkle Tree is also removable.


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
```toml
[dependencies.child]
git = 'https://github.com/stakedtechnologies/Plasm.git'
package = 'plasm-child'
version = '0.1.0' 
```

## Example Trait
Please see [here](https://github.com/stakedtechnologies/Plasm/blob/master/runtime/src/lib.rs).

## Maintainers
- [@public_sate](https://twitter.com/public_sate)

* * *
Plasm is licensed under the Apache License, Version2.0 by Staked Technologies Inc.
