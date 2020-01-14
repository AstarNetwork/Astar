<div align="center"><img width="798" alt="plasm" src="https://user-images.githubusercontent.com/6259384/56867192-8b967500-6a1d-11e9-898d-f73f4e2a387c.png"></div>

[![Build Status](https://travis-ci.org/staketechnologies/Plasm.svg?branch=master)](https://travis-ci.org/staketechnologies/Plasm)

Plasm is a Substrate Runtime Module Library which allows developers to add Plasma functions to their Substrate chain easily and seamlessly. Since Plasm is an SRML, developers can also make both plasma parent chains and plasma child chains with Substrate. 

__WARNING__: This is a proof-of-concept prototype. This implementation is NOT ready for production use. 

Whitepaper
----------

* [Whitepaper](https://github.com/stakedtechnologies/plasmdocs/blob/master/wp/en.pdf)
* [Whitepaper(JP)](https://github.com/stakedtechnologies/plasmdocs/blob/master/wp/jp.pdf)

Community
--------- 

* [Telegram](https://t.me/PlasmOfficial)
* [Discord](https://discord.gg/Z3nC9U4)
* [Twitter](https://twitter.com/Plasm_Network)

Table of Contents
-----------------

* [Introduction](https://github.com/stakedtechnologies/Plasm/tree/master#introduction)
* [Install Plasm](https://github.com/stakedtechnologies/Plasm/tree/master#install-plasm)
* [Plasm Validator Program](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-validator-program)
* [Examples](https://github.com/stakedtechnologies/Plasm/tree/master#examples)

Introduction
============

Plasm is a Substrate Runtime Module Library which allows developers to add Plasma functions to their Substrate chain. By adding a Plasm Substrate Runtime Module Library, you can get scalable blockchains within a few minutes.

Some people might not know [Substrate Runtime Module Library](https://docs.substrate.dev/docs/srml-overview). Basically speaking, Substrate consists of 2 components, Substrate Core and Substrate Runtime Module Library aka SRML. We can customize Substrate Core with SRML and make an original Substrate chain.

Other people might not know Plasma. Plasma is a layer2 scaling solution which makes it possible for scalable computation by structuring economic incentives to operate the blockchain autonomously without the operator’s management. Ideally, it brings infinite scalability into your blockchain.

Based on the above, Plasm has some features.
- **The first Rust implementation of Plasma SRML.**
- **Plasm is a simple but versatile SRML and makes it easier for developers to make a Plasma chain with Substrate.**
- **Plasm deals with many types of “Plasmas” in the future. Currently, we are providing UTXO models.**
- **Substrate chain can be both a plasma parent chain and a plasma child chain.**

Since we are making an SRML, we can also make a scalable chain with Substrate. Once Polkadot is launched, we will connect our root chain to Polkadot, and we aim to be one of the parachains.
<img width="1330" alt="vision" src="https://user-images.githubusercontent.com/29359048/59095564-cdd3a000-8953-11e9-85bb-d273ce05f509.png">

We call this chain Plasm Network. Plasm Network is a scaling DApps Platform based on Substrate. The point is Polkadot Relaychain doesn’t support smart contracts by design. So, people in the Polkadot ecosystem need Parachains that support smart contracts well. From the developer’s perspective, he needs to choose on which Parachain his decentralized application should be built. Scalability must be one of the most important criteria for him to choose which Parachain to use. This is where Plasm Network comes in.

Install Plasm 
=============

* Plasm node binaries [releases](https://github.com/stakedtechnologies/Plasm/releases).
* Node [custom types](https://github.com/staketechnologies/Plasm/tree/master/bin/node/cli/res/custom_types.json). 

> Latest version you can try to build from source.

Building from source
--------------------

Ensure you have Rust and the support software installed:

    curl https://sh.rustup.rs -sSf | sh
    # on Windows download and run rustup-init.exe
    # from https://rustup.rs instead

    rustup update nightly
    rustup target add wasm32-unknown-unknown --toolchain nightly
    cargo install --git https://github.com/alexcrichton/wasm-gc

You will also need to install the following packages:

* Linux: `sudo apt install cmake git clang libclang-dev build-essential`
* Mac: `brew install cmake git llvm`
* Windows: Download and install the Pre Build Windows binaries of LLVM from http://releases.llvm.org/download.html

Install Plasm node from git source:

    cargo install --force --git https://github.com/stakedtechnologies/Plasm --tag v0.7.1 plasm-cli

Run node in [Plasm testnet](https://telemetry.polkadot.io/#/PlasmTestnet%20v1):

    plasm-node

Or run in your local development network:

    plasm-node --dev

Building with Nix
-----------------

Install Nix package manager:

    curl https://nixos.org/nix/install | sh

Run in Nix shell:

    git clone https://github.com/stakedtechnologies/Plasm && cd Plasm
    nix-shell nix/shell.nix --run "cargo run --release"

Plasm Validator Program
=======================
Since we launched our Plasm Network testnet, we are looking for around 50 validators all over the world. This is a testnet like Ethereum Rinkeby, Kovan, and Ropsten. So, PLM (Plasm Network native token called PLUM) doesn’t have any values. Therefore, there is no incentive to be a validator on the testnet. To solve this problem,
We will provide you with a right to be the first validator during the PoA term (between Lockdrop1 and Lockdrop2 described below) on the mainnet if you are a validator on the Plasm testnet.

<img width="1287" alt="Screen Shot 2019-11-16 at 22 24 37" src="https://user-images.githubusercontent.com/29359048/68994354-799af780-08c5-11ea-9a6f-7e9080ddc893.png">

1. Run a node on [Plasm testnet](https://telemetry.polkadot.io/#/PlasmTestnet%20v1):

    plasm-node --validator

2. Apply the validator program:

* https://docs.google.com/forms/d/1g0XGDQ0qg-YipwmHlmrnszF8BI0E85xY42pMZpF0knI/viewform

Examples
========

You can see our demo: 
* [Version1](https://www.youtube.com/watch?v=T70iEgyuXbw&feature=youtu.be): 2019/04/25 CLI Demo 
* [Version2](https://youtu.be/5MoO3Epgvv0): 2019/05/22 UI Demo No explanations yet.

Future Works
------------

![1_MsvI5mbUlwMYAnzHxOTwuw](https://user-images.githubusercontent.com/29359048/68994260-57ed4080-08c4-11ea-8659-3a0b066661bc.png)

Contacts
--------

**Maintainers**

* [Public_Sate](https://twitter.com/public_sate)
* [Task Ohmori](https://twitter.com/taskooh?lang=en)
* [Aleksandr Krupenkin](https://github.com/akru)
* [Sota Watanabe](https://twitter.com/WatanabeSota)

* * *

Plasm is licensed under the GPLv3.0 by Stake Technologies Inc.
