# RPC tests

This project aims to create a testing framework for the astar RPC node. It achieves this by spawning a local network with `zombienet`, a Simple CLI tool to launch a local Polkadot test network. [zombienet](https://github.com/paritytech/zombienet) takes configuration for the test network in a `rpc-tests.toml` file, which describes the config of relaychains and parachains that the test network needs in order to start.

After the test network is started, tests can be performed by connecting to it using `polkadot.js` or `web3.js`.

The test suite has global setup and teardown steps for the test network with zombienet. It runs the tests with the `rpc-tests.zndsl` test suite runner.

## Requirements

- node.js 18+
Download from `https://nodejs.org/en/`
- yarn
To install, run: `npm install -g yarn`

## Usage

Build astar collator:

```sh
cargo build --release
```

Copy the binary in rpc-tests/bin folder:

```sh
mkdir -p rpc-tests/bin
cp target/release/astar-collator rpc-tests/bin/astar-collator
```

Download and copy the latest polkadot binary from https://github.com/paritytech/polkadot/releases to the `rpc-tests/bin` folder.

Download and copy the latest zombienet binary from https://github.com/paritytech/zombienet/releases to the `rpc-tests/bin` folder.

To start the test suite:

```sh
cd rpc-tests
```

For all runtime:

```sh
yarn test
```
