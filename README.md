<div align="center"><img width="1007" alt="plasm_web3" src="https://user-images.githubusercontent.com/6259384/72399615-0e1cbb80-378a-11ea-91af-c1dbdde345f5.png"></div>

[![CI](https://github.com/staketechnologies/Plasm/workflows/Integration/badge.svg)](https://github.com/staketechnologies/Plasm/actions)

Our mission is to build a scalable infrastructure for Web3.0. The Plasm Network will be the leading scalable smart contract network on Polkadot, supporting cutting-edge layer 2 solutions, such as Plasma, State Channels and Rollups.

Documentation
----------

* [Documentation](https://docs.plasmnet.io/)
* [Slide Deck](https://drive.google.com/file/d/1V6GlROaK4Thk7FMUCnepCAGwAMM_f9QR/view?usp=sharing)

Whitepaper
----------

* [Whitepaper](https://github.com/staketechnologies/plasmdocs/blob/master/wp/en.pdf)
* [Whitepaper(JP)](https://github.com/staketechnologies/plasmdocs/blob/master/wp/jp.pdf)

Community
--------- 

* Common group: [Telegram](https://t.me/PlasmOfficial)
* Technical group: [Discord](https://discord.gg/Z3nC9U4)
* Subscribe on [Twitter](https://twitter.com/Plasm_Network)

Table of Contents
-----------------

* [Introduction](https://github.com/staketechnologies/Plasm/tree/master#introduction)
* [Install Plasm](https://github.com/staketechnologies/Plasm/tree/master#install-plasm)
* [Plasm Validator Program](https://github.com/staketechnologies/Plasm/tree/master#plasm-validator-program)
* [Examples](https://github.com/staketechnologies/Plasm/tree/master#examples)

Introduction
============

Plasm Network is a scalable and interoperable infrastructure for Web3.0. Since Plasm Network is built with [Parity’s Substrate framework](https://www.substrate.io/), it can be a future [Polkadot](https://polkadot.network/) Parachain that also acts as a scalable smart contract platform. The Polkadot Relaychain, by design, does not support smart contracts. This allows Plasm the opportunity to fill in this gap. Scalability is obviously one of the most crucial demands DApp developers have. Ideally, the developers can build whatever applications on Plasm Network without having to consider its scalability.

Based on the above, Plasm has some features.
- **[Optimistic Virtual Machine](https://docs.plasmnet.io/learn/optimistic-virtual-machine)**
- **[DApps Reward](https://docs.plasmnet.io/learn/dapps-reward)**
- **[Operator Trading](https://docs.plasmnet.io/learn/operator-trading)**
- **[Lockdrop](https://docs.plasmnet.io/learn/lockdrop)**

Once Polkadot is launched, we will connect our root chain to Polkadot, and we aim to be one of the parachains.
<img width="1330" alt="vision" src="https://user-images.githubusercontent.com/29359048/59095564-cdd3a000-8953-11e9-85bb-d273ce05f509.png">

Install Plasm 
=============

* Plasm node binaries [releases](https://github.com/staketechnologies/Plasm/releases).
* Node [custom types](https://github.com/staketechnologies/Plasm/tree/master/bin/node/cli/res/custom_types.json). 

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

Install the Plasm node from git source:

    cargo +nightly install --locked --force --git https://github.com/staketechnologies/Plasm --tag v1.4.0 plasm-cli

Run node on [Dusty Network](https://telemetry.polkadot.io/#list/Dusty):

    plasm-node

Or run on your local development network:

    plasm-node --dev

Building with Nix
-----------------

Install Nix package manager:

    curl https://nixos.org/nix/install | sh

Run on your Nix shell:

    git clone https://github.com/staketechnologies/Plasm && cd Plasm
    nix-shell nix/shell.nix --run "cargo run --release"

Plasm Validator Program
=======================

Currently, we have 2 networks, [Dusty Network](https://telemetry.polkadot.io/#list/Dusty) and [Plasm Network](https://telemetry.polkadot.io/#list/Plasm). Dusty is our canary R&D chain like Kusama. The stable validators on Dusty can be the first validators on Plasm mainnet. We are looking for 100 validators on the Plasm Network. 

If you would like to be the validator, please check out [our tutorial](https://docs.plasmnet.io/workshop-and-tutorial/untitled) and join [Discord tech channel](https://discord.gg/wUcQt3R)

Examples
========

You can see our demo and presentation: 
* [Version1](https://www.youtube.com/watch?v=T70iEgyuXbw&feature=youtu.be): 2019/04/25 CLI Demo 
* [Version2](https://youtu.be/5MoO3Epgvv0): 2019/05/22 UI Demo No explanations yet.
* [Subzero Summit](https://www.youtube.com/watch?v=OyKvA_vx1z0): 2020/04 Presentation at Subzero Summit
* [DOT CON][https://www.youtube.com/watch?v=og0yUFdYyLY]: 2019/10 Presentation at DOT CON

Future Works
------------
Here are the key milestones.

1. Start the 2nd Lockdrop on Dusty Network (July)
1. Start the 2nd Lockdrop on Plasm Network (August)
1. Support the Optimistic Virtual Machine (September to October)
1. Start the 3nd Lockdrop on Plasm Network (During Polkadot Parachain Auction)
1. Start the 3nd Lockdrop on Plasm Network (During Polkadot Parachain Auction)
1. Become a Polkadot Parachain. (TBA)

If you have any questions, please ask us on [Discord](https://discord.gg/Z3nC9U4)

Contacts
--------

**Maintainers**

* [Public_Sate](https://twitter.com/public_sate)
* [Task Ohmori](https://twitter.com/taskooh?lang=en)
* [Aleksandr Krupenkin](https://github.com/akru)
* [Sota Watanabe](https://twitter.com/WatanabeSota)

* * *

Plasm is licensed under the GPLv3.0 by Stake Technologies Inc.
