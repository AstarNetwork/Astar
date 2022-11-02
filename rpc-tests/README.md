# RPC tests

This project aim to create tesing framework for astar rpc node. It achieve this by spawning a local network with polkadot-launch a Simple CLI tool to launch a local Polkadot test network. [polkadot-launch](https://github.com/paritytech/polkadot-launch) take configuration for the test network in a config.json which describes the config of relaychains and parachains that needs to be started for test network to start.

After the test network is started tests can be performed by connecting to it using polkadot.js or web3.js.t

The test suite has a global setup and teardown steps for the test network in `before` and `after` functions which spawns the test network with polkadot-launch. Then it runs the tests with [mocha](https://mochajs.org/) test suite runner.

## Requirements

- node.js 16+
Download from https://nodejs.org/en/
- yarn
To install `npm install -g yarn`
- polkadot-launch
To install `yarn add polkadot-launch --global`

## Usage

Build astar collator.

```
cargo build --release
```

Copy the binary in rpc-tests/bin folder.

```
mkdir -p rpc-tests/bin
cp target/release/astar-collator rpc-tests/bin/astar-collator
```

Download and copy latest polkadot binary from https://github.com/paritytech/polkadot/releases to rpc-tests/bin folder

To start the test suite.

```
cd rpc-tests
```

For astar runtime.

```
yarn test:astar
```

For shiden runtime

```
yarn test:shiden
```