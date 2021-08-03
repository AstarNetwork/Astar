# Astar Network

![astar-cover](https://user-images.githubusercontent.com/40356749/125183345-edbaa980-e226-11eb-92e2-7b666ed56515.png)

[![CI](https://github.com/PlasmNetwork/Plasm/workflows/Integration/badge.svg)](https://github.com/PlasmNetwork/Astar/actions)

Astar Network is a dApps hub on Polkadot with unique features like dApps Staking, supporting mulit-virtual machine (EVM & WAsM), and offers layer2 solutions from day 1. Shiden Network is our sister network on Kusama.

## Documentation

* [Documentation](https://docs.plasmnet.io/)
* [Slide Deck](https://drive.google.com/file/d/1qnx2XZTtU0qCwxBX--FUHCdBmOK7ZIF3/view?usp=sharing)

## Whitepaper

* [Whitepaper](https://github.com/PlasmNetwork/plasmdocs/blob/master/wp/en.pdf)
* [Whitepaper(JP)](https://github.com/PlasmNetwork/plasmdocs/blob/master/wp/jp.pdf)

## Community

* Common group: [Telegram](https://t.me/PlasmOfficial)
* Technical group: [Discord](https://discord.gg/Z3nC9U4)
* Subscribe on [Astar Network Twitter](https://twitter.com/Plasm_Network)
* Subscrive on [Shiden Network Twitter](https://twitter.com/ShidenNetwork)

## Table of Contents

* [Introduction](https://github.com/PlasmNetwork/Astar/tree/development/dusty#introduction)
* [Install Astar](https://github.com/PlasmNetwork/Astar/tree/development/dusty#install-plasm)
* [Plasm Validator Program](https://github.com/PlasmNetwork/Astar/tree/development/dusty#plasm-validator-program)
* [Examples](https://github.com/PlasmNetwork/Astar/tree/development/dusty#examples)

Introduction
============

Astar Network is a scalable and interoperable infrastructure for Web3.0. Since Astar Network is built with [Parity’s Substrate framework](https://www.substrate.io/), it can be a future [Polkadot](https://polkadot.network/) Parachain that also acts as a scalable smart contract platform. The Polkadot Relaychain, by design, does not support smart contracts. This allows Astar the opportunity to fill in this gap. Scalability is obviously one of the most crucial demands DApp developers have. Ideally, the developers can build whatever applications on Astar Network without having to consider its scalability. In addition to that, Astar Network is a multi virtual machines platfrom. Astar supports both Ethereum Virtual Machine and WebAssembly. All devs of Astar Network can deploy Solidity smart contracts by using existing Ethereum tools such as Metamask and Remix.

Based on the above, Astar has some features.
- **[Optimistic Virtual Machine](https://docs.plasmnet.io/learn/optimistic-virtual-machine)**
- **[ZK Rollups](https://github.com/PlasmNetwork/ZKRollups)**
- **[DApps Staking](https://docs.plasmnet.io/learn/dapps-reward)**
- **[Operator Trading](https://docs.plasmnet.io/learn/operator-trading)**
- **[Lockdrop](https://docs.plasmnet.io/learn/lockdrop)**

Once Polkadot is launched, we will connect our root chain to Polkadot, and we aim to be one of the parachains.
<img width="888" alt="Screen Shot 2021-02-01 at 14 15 29" src="https://user-images.githubusercontent.com/29359048/106417721-0b296180-6498-11eb-8a0a-a10a8e387433.png">

Install Astar
=============

* Astar node binaries [releases](https://github.com/PlasmNetwork/Astar/releases).
* Node [custom types](https://github.com/PlasmNetwork/Astar/tree/master/bin/node/cli/res/custom_types.json).

> Latest version you can try to build from source.

Building from source
--------------------

Ensure you have Rust and the support software:

    curl https://sh.rustup.rs -sSf | sh
    # on Windows download and run rustup-init.exe
    # from https://rustup.rs instead

    rustup update nightly
    rustup target add wasm32-unknown-unknown --toolchain nightly

You will also need to install the following dependencies:

* Linux: `sudo apt install cmake git clang libclang-dev build-essential`
* Mac: `brew install cmake git llvm`
* Windows: Download and install the Pre Build Windows binaries of LLVM from http://releases.llvm.org/download.html

Install additional build tools:

    cargo +nightly install --git https://github.com/alexcrichton/wasm-gc

Install the Astar node from git source:
```
    cargo +nightly install --locked --force --git https://github.com/PlasmNetwork/Astar --tag v1.9.0-dusty plasm
    # include the tag above to specify the version you want. Refer to the tags on this repo
```

Run node on [Dusty Network](https://telemetry.polkadot.io/#list/Dusty):

    plasm

Or run on your local development network:

    plasm --dev

Building with Nix
-----------------

Install Nix package manager:

    curl https://nixos.org/nix/install | sh

Run on your Nix shell:

    git clone https://github.com/PlasmNetwork/Plasm && cd Plasm
    nix-shell -I nixpkgs=channel:nixos-21.05 third-party/nix/shell.nix --run "cargo run --release"

Examples
========

You can see our demo and presentation:
* [Version1](https://www.youtube.com/watch?v=T70iEgyuXbw&feature=youtu.be): 2019/04/25 CLI Demo
* [Version2](https://youtu.be/5MoO3Epgvv0): 2019/05/22 UI Demo No explanations yet.
* [Subzero Summit](https://www.youtube.com/watch?v=OyKvA_vx1z0): 2020/04 Presentation at Subzero Summit
* [DOT CON][https://www.youtube.com/watch?v=og0yUFdYyLY]: 2019/10 Presentation at DOT CON

Contributing to Astar
=======================

Currently, we have 2 production networks:

- [Astar Network](https://astar.network/): formerly known as Plasm Network, and our future Parachain on Polkadot Network
- [Shiden Network](https://shiden.astar.network/): Parachain of Kusama Network and our canary release for Astar Network

And 2 test networks:

- [Dusty Network](https://github.com/PlasmNetwork/Astar/tree/development/dusty): an independent test network with the latest cutting edge features before it is introduced to Astar Network
- [Shibuya Network](https://github.com/PlasmNetwork/Astar/tree/development/shibuya): a Parachain test network with the latest cutting edge features related to XMCP, collators, and other cross chain operations that is planned to be rolled out to Shiden Network

In the future we will gradually change this structure to have a single test network.
For details, please read the [Contribution Guidelines](./CONTRIBUTING.md).

If you would like to be the validator, please check out [our tutorial](https://docs.plasmnet.io/build/validator-guide) and join [Discord tech channel](https://discord.gg/wUcQt3R)

Contacts
--------

**Maintainers**

* [Public_Sate](https://twitter.com/public_sate)
* [Task Ohmori](https://twitter.com/taskooh?lang=en)
* [Aleksandr Krupenkin](https://github.com/akru)
* [Sota Watanabe](https://twitter.com/WatanabeSota)
* [Hyungsuk Kang](https://twitter.com/hskang0525)
* [Hoon Kim](https://github.com/hoonsubin)

* * *

Astar is licensed under the GPLv3.0 by Stake Technologies Inc.
