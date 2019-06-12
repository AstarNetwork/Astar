<div align="center"><img width="798" alt="plasm" src="https://user-images.githubusercontent.com/6259384/56867192-8b967500-6a1d-11e9-898d-f73f4e2a387c.png"></div>

[![Build Status](https://travis-ci.org/stakedtechnologies/Plasm.svg?branch=master)](https://travis-ci.org/stakedtechnologies/Plasm)

Plasm is a Substrate Runtime Module Library which allows developers to add Plasma functions to their Substrate chain easily and seamlessly. Since Plasm is an SRML, developers can also make both plasma parent chains and plasma child chains with Substrate. 

__WARNING__: This is a proof-of-concept prototype. This implementation is NOT ready for production use. 

## Table of Contents
- [Introduction](https://github.com/stakedtechnologies/Plasm/tree/master#introduction)
- [Demo](https://github.com/stakedtechnologies/Plasm/tree/master#demo)
- [Plasm ver0.2.0](https://github.com/stakedtechnologies/Plasm/tree/master#plasm-ver020)
- [Future Works](https://github.com/stakedtechnologies/Plasm/tree/master#future-works)
- [Build Nodes on Docker](https://github.com/stakedtechnologies/Plasm/tree/master#build-nodes-on-docker)
- [Build Nodes for Developer](https://github.com/stakedtechnologies/Plasm/tree/master#build-nodes-for-developer)
- [Example Trait](https://github.com/stakedtechnologies/Plasm/tree/master#example-trait)

## Introduction
Plasm is a Substrate Runtime Module Library which allows developers to add Plasma functions to their Substrate chain. By adding a Plasm Substrate Runtime Module Library, you can get scalable blockchains within a few minutes.

Some people might not know [Substrate Runtime Module Library](https://docs.substrate.dev/docs/srml-overview). Basically speaking, Substrate consists of 2 components, Substrate Core and Substrate Runtime Module Library aka SRML. We can customize Substrate Core with SRML and make an original Substrate chain.

Other people might not know Plasma. Plasma is a layer2 scaling solution which makes it possible for scalable computation by structuring economic incentives to operate the blockchain autonomously without the operator’s management. Ideally, it brings infinite scalability into your blockchain.

Based on the above, Plasm has some features.
- **The first Rust implementation of Plasma SRML.**
- **Plasm is a simple but versatile SRML and makes it easier for developers to make a Plasma chain with Substrate.**
- **Plasm deals with many types of “Plasmas” in the future. Currently, we are providing UTXO models.**
- **Substrate chain can be both a plasma parent chain and a plasma child chain.**

Since we are making an SRML, we can also make a Plasma chain with Substrate. Once Polkadot is launched, we will connect our root chain to Polkadot, and we aim to be one of the parachains.
<img width="1330" alt="vision" src="https://user-images.githubusercontent.com/29359048/59095564-cdd3a000-8953-11e9-85bb-d273ce05f509.png">
In addition, Plasm makes it easier even for other developers to make a Plasma chain.

## Demo
You can see our demo 
- [Version1](https://www.youtube.com/watch?v=T70iEgyuXbw&feature=youtu.be): 2019/04/25 CLI Demo 
- [Version2](https://youtu.be/5MoO3Epgvv0): 2019/05/22 UI Demo No explanations yet. I will add asap.

## Plasm ver0.2.0
Plasm ver0.2.0 is a prototype which has the following functions.

* [Plasma MVP implementation](https://github.com/stakedtechnologies/Plasm)
    * [Plasm-Parent](https://github.com/stakedtechnologies/Plasm/tree/master/core/parent) provides the parent chain’s specification. Mainly, Plasm-Parent has the logic of each exit game.
    * [Plasm-Child](https://github.com/stakedtechnologies/Plasm/tree/master/core/child) provides the child chain’s specification.
    * Plasm [UTXO](https://github.com/stakedtechnologies/Plasm/tree/master/core/utxo)/[Merkle](https://github.com/stakedtechnologies/Plasm/tree/master/core/merkle) provides the data structure which manages transactions.
* [Plasma Client implementation](https://github.com/stakedtechnologies/plasm-client)
    * **Plasm Util** is a wrapper function to call the endpoint of blockchains.
    * **Plasm Operator** is monitoring a parent chain and a child chain to make the deposit/exit successful.
    * **Plasm CLI** is a CLI tool to call the endpoint.
    * **Plasm Wallet** is an application to send, withdraw and receive tokens.

As a next step, let’s make a wallet demo application on your laptop and see what’s happening.

### Step1
Clone Plasm from our GitHub.

```
$ git clone https://github.com/stakedtechnologies/Plasm.githttps://github.com/stakedtechnologies/Plasm.git
$ cd Plasm
$ git checkout v0.2.0
```

### Step2
Build Plasm Node. After a successful build, you can run Plasma nodes.
```
$ cargo build
$ ./target/debug/plasm-node --dev
```

### Step3
Open another terminal and clone Plasm-Client from our GitHub.

```
$ git clone https://github.com/stakedtechnologies/plasm-client.git
$ cd plasm-client
$ git checkout v0.2.0
```

### Step4
Start **Plasm Operator**. The operator is monitoring both the parent chain and the child chain. When the parent chain deposits tokens to the child chain or the child chain exits tokens to the parent chain, the operator writes the root hash of the child chain on the parent chain.

```
$ cd packages/operator
$ cp ../../env.sample .env
$ yarn install
$ yarn start
```

<img width="1425" alt="Screen Shot 2019-06-08 at 1 02 33" src="https://user-images.githubusercontent.com/29359048/59117609-26716000-8989-11e9-8652-d8f9a438cf5d.png">

If you can see the output below, this project has been successful.

<img width="1434" alt="Screen Shot 2019-06-08 at 1 05 04" src="https://user-images.githubusercontent.com/29359048/59117729-79e3ae00-8989-11e9-83d3-9b1661d91a43.png">

Yeah, you made it, but what actually happened? Let me clarify! (You can skip if you want. This is complicated, so we will publish another article focused on this topic.)

> Plasm-Operator gets an event which the plasma child/parent chain issues and processes the following steps.
> Parent chain’s events
> - When an operator receives a Submit event, she finalizes the status of the child chain.
> - When an operator receives a Deposit event, she sends tokens to the issuer.
> - When an operator receives an ExitStart event, she deletes the UTXO which was used for exiting on the child chain.
> Child chain’s events.
> When an operator receives a Submit event, she submits the root hash to the parent chain. The Submit event on a child chain is issued regularly. （You can decide
 this logic. For this time, 1 time in 5 blocks.)

### Step5
Open another terminal and move to plasm-client root directory. Then, start Plasm Wallet UI Demo Application.

```
$ cd packages/wallet
$ yarn install
$ yarn dev
```

<img width="1027" alt="Screen Shot 2019-06-08 at 1 07 31" src="https://user-images.githubusercontent.com/29359048/59117884-d050ec80-8989-11e9-9f27-738df14a1e0c.png">

After that, let’s go to [localhost:8000](http://localhost:8000/) on your browser. We will create 2 different accounts and send/receive tokens by using the wallet application I mentioned above.

### Step6: Account Registration
First, you need to register your demo account. Since a default operator is Alice, you should add //Alice ①. Then, create an account ②. You can check Alice’s balance ③.

<img width="1405" alt="Screen Shot 2019-06-08 at 1 09 00" src="https://user-images.githubusercontent.com/29359048/59117971-07270280-898a-11e9-9a20-08fa4fc85186.png">

To send tokens from Alice to Bob, Alice to Tom and Bob to Tom, generate Bob’s and Tom’s account as well.

<img width="1440" alt="Screen Shot 2019-06-08 at 1 09 44" src="https://user-images.githubusercontent.com/29359048/59118021-21f97700-898a-11e9-9861-392140640467.png">

### Step7: Token Transfer on Parent Chain
As a next step, we will send tokens from Alice to Bob and Alice to Tom on the parent chain.

<img width="1285" alt="Screen Shot 2019-06-08 at 1 11 01" src="https://user-images.githubusercontent.com/29359048/59118123-5a00ba00-898a-11e9-855e-869242cd6d19.png">

<img width="1402" alt="Screen Shot 2019-06-08 at 1 11 22" src="https://user-images.githubusercontent.com/29359048/59118138-5e2cd780-898a-11e9-9aec-8ff285c6879d.png">
Enter the account name and decide the amount of token. Then, click the “Send” button. Keep your eye on the “ParentBalance” next to the account name. After a successful transaction, you will notice that Bob’s amount is increasing. Currently, we collect the exchange fee from the sender on the parent chain. So, Bob and Tom need to have some tokens on the parent chain.

### Step8: Deposit (Deposit tokens from Parent Wallet to Child Wallet.)
<img width="1266" alt="Screen Shot 2019-06-08 at 1 12 40" src="https://user-images.githubusercontent.com/29359048/59118227-8f0d0c80-898a-11e9-8ce8-2624fd41e2a0.png">

Third, we will send tokens from the parent chain to the child chain. For this time, Bob deposits 5,000,000 tokens to the child chain. Just keep in mind, it takes time to increase ChildBalance because the operator checks the event and executes a transaction.

<img width="1398" alt="Screen Shot 2019-06-08 at 1 13 36" src="https://user-images.githubusercontent.com/29359048/59118280-b237bc00-898a-11e9-801c-74f54a232c55.png">

### Step9: Token Transfer on Child Chain

<img width="1270" alt="Screen Shot 2019-06-08 at 1 14 10" src="https://user-images.githubusercontent.com/29359048/59118323-c8457c80-898a-11e9-87eb-f8a31f8e962b.png">

Fourth, let’s send some tokens from Bob to Tom on the child chain.

<img width="1337" alt="Screen Shot 2019-06-08 at 1 15 04" src="https://user-images.githubusercontent.com/29359048/59118365-e3b08780-898a-11e9-8f66-d14f5d156096.png">

Bob has 5,000,000 tokens. He sent 1,000,000 tokens out of 5,000,000 to Tom.

### Step10: Exit Part1（Exit tokens from ChildWallet to ParentWallet.）

<img width="1253" alt="Screen Shot 2019-06-08 at 1 16 22" src="https://user-images.githubusercontent.com/29359048/59118430-10649f00-898b-11e9-8297-bc008a424c78.png">

Exit tokens from Tom’s account on the child chain to his account on the parent chain. If you type your account name, you can find UTXO lists you have. A child chain has all transaction histories you made and tokens are exited based on UTXO.

<img width="1376" alt="Screen Shot 2019-06-08 at 1 16 49" src="https://user-images.githubusercontent.com/29359048/59118461-21151500-898b-11e9-8ee9-7698fbbf4911.png">

Press the ExitStart button so that you can exit your tokens to the parent chain.

<img width="1400" alt="Screen Shot 2019-06-08 at 1 17 32" src="https://user-images.githubusercontent.com/29359048/59118482-3a1dc600-898b-11e9-9a95-b83d770eeee0.png">


BUT, you have to wait about 60 seconds. It is a Plasma challenge period which we decided. Full node holders can challenge the legitimacy of exits in it.

### Step11: Exit Part2（ExitFinalize ChildWallet to ParentWallet.）
Click ExitFinalize.

<img width="1387" alt="Screen Shot 2019-06-08 at 1 18 32" src="https://user-images.githubusercontent.com/29359048/59118517-5a4d8500-898b-11e9-8fec-dfb2b981e15f.png">

Then,

<img width="1266" alt="Screen Shot 2019-06-08 at 1 19 16" src="https://user-images.githubusercontent.com/29359048/59118569-74876300-898b-11e9-9ac7-42877d7728d7.png">

Finally, the exit is successful. Well done!! This is a simple demo, but it’s one giant leap for the Polkadot/Substrate community!!

## Future Works
**ver0.2.0rc1**
: Actually, we just have one node in this tutorial because we used balances SRML, the default setting. We will divide this node into a parent node and a child node using PlasmUtxo SRML.

**ver0.5.0**
: Connect our root chain to Polkadot Testnet.

**v0.7.0**
: Plasma Cash implementation

**v1.0.0**
: Plasma Chamber implementation

**Another Important Task**
: Improve ExitGame implementation

## Build Nodes on Docker
### Parent(Root) Node
```bash
> docker run -p 9944:9944 stakedtechnologies/plasm-node
```

### Child Node
```bash
> docker run -p 9955:9944 stakedtechnologies/plasm-child-node
```

## Build Nodes for Developer
### Parent(Root) Node
```bash
> cd plasm
> ./build
> cargo build
> ./target/debug/plasm-node --base-path /tmp/parent --port 30333 --ws-port 9944 --dev
```

### Child Node
```bash
> cd plasm/child
> ./build
> cargo build
> ./target/debug/plasm-child-node --base-path /tmp/child --port 30334 --ws-port 9955 --dev
```

## Example Trait
Please check [here](https://github.com/stakedtechnologies/Plasm/blob/master/runtime/src/lib.rs).

# Maintainers
* [Public_Sate](https://twitter.com/public_sate)

* * *
Plasm is licensed under the Apache License, Version2.0 by Staked Technologies Inc.