# RPC tests

This project aim to create tesing framework for astar rpc node. It achieve this by spawning a local network with zombienet a Simple CLI tool to launch a local Polkadot test network. [zombienet](https://github.com/paritytech/zombienet) take configuration for the test network in a `rpc-tests.toml` which describes the config of relaychains and parachains that needs to be started for test network to start.

After the test network is started tests can be performed by connecting to it using polkadot.js or web3.js.t

The test suite has a global setup and teardown steps for the test network with zombienet. Then it runs the tests with `rpc-tests.zndsl` test suite runner.

## Requirements

- node.js 18+
Download from https://nodejs.org/en/
- yarn
To install `npm install -g yarn`

## Usage

Build astar collator.

```sh
cargo build --release
```

Copy the binary in rpc-tests/bin folder.

```sh
mkdir -p rpc-tests/bin
cp target/release/astar-collator rpc-tests/bin/astar-collator
```

Download and copy latest polkadot binary from https://github.com/paritytech/polkadot/releases to rpc-tests/bin folder

Download and copy latest zombinet binary from https://github.com/paritytech/zombinet/releases to rpc-tests/bin folder

To start the test suite.

```sh
cd rpc-tests
```

For all runtime.

```sh
yarn test
```
