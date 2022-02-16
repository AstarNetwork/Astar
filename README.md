![astar-cover](https://user-images.githubusercontent.com/40356749/135799652-175e0d24-1255-4c26-87e8-447b192fd4b2.gif)

<div align="center">

[![Integration Action](https://github.com/AstarNetwork/Astar/workflows/Integration/badge.svg)](https://github.com/AstarNetwork/Astar/actions)
[![GitHub tag (latest by date)](https://img.shields.io/github/v/tag/AstarNetwork/Astar)](https://github.com/AstarNetwork/Astar/tags)
[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-brightgreen?logo=Parity%20Substrate)](https://substrate.dev/)
[![License](https://img.shields.io/github/license/AstarNetwork/Astar?color=green)](https://github.com/AstarNetwork/Astar/blob/production/shiden/LICENSE)
 <br />
[![Twitter URL](https://img.shields.io/twitter/follow/AstarNetwork?style=social)](https://twitter.com/AstarNetwork)
[![Twitter URL](https://img.shields.io/twitter/follow/ShidenNetwork?style=social)](https://twitter.com/ShidenNetwork)
[![YouTube](https://img.shields.io/youtube/channel/subscribers/UC36JgEF6gqatVSK9xlzzrvQ?style=social)](https://www.youtube.com/channel/UC36JgEF6gqatVSK9xlzzrvQ)
[![Docker](https://img.shields.io/docker/pulls/staketechnologies/astar-collator?logo=docker)](https://hub.docker.com/r/staketechnologies/astar-collator)
[![Discord](https://img.shields.io/badge/Discord-gray?logo=discord)](https://discord.gg/Z3nC9U4)
[![Telegram](https://img.shields.io/badge/Telegram-gray?logo=telegram)](https://t.me/PlasmOfficial)
[![Medium](https://img.shields.io/badge/Medium-gray?logo=medium)](https://medium.com/astar-network)

</div>

Astar Network is an interoperable blockchain based the Substrate framework and the hub for dApps within the Polkadot Ecosystem.
With Astar Network and Shiden Network, people can stake their tokens to a Smart Contract for rewarding projects that provide value to the network.

For contributing to this project, please read our [Contribution Guideline](./CONTRIBUTING.md).

## Building From Source

> This section assumes that the developer is running on either macOS or Debian-variant operating system. For Windows, although there are ways to run it, we recommend using [WSL](https://docs.microsoft.com/en-us/windows/wsl/install-win10) or from a virtual machine for stability.

Execute the following command from your terminal to set up the development environment and build the node runtime.

```bash
# install Substrate development environment via the automatic script
$ curl https://getsubstrate.io -sSf | bash -s -- --fast

# clone the Git repository
$ git clone --recurse-submodules https://github.com/AstarNetwork/Astar.git

# change current working directory
$ cd Astar

# compile the node
# note: you may encounter some errors if `wasm32-unknown-unknown` is not installed, or if the toolchain channel is outdated
$ cargo build --release

# show list of available commands
$ ./target/release/astar-collator --help
```

### Building with Nix

```bash
# install Nix package manager:
$ curl https://nixos.org/nix/install | sh

# run from root of the project folder (`Astar/` folder)
$ nix-shell -I nixpkgs=channel:nixos-21.05 third-party/nix/shell.nix --run "cargo build --release"
```

## Running a Collator Node

To set up a collator node, you must have a fully synced node with the proper arguments, which can be done with the following command.

```bash
# start the Shiden collator node with
$ ./target/release/astar-collator \
  --base-path <path to save blocks> \
  --name <node display name> \
  --port 30333 \
  --ws-port 9944 \
  --rpc-port 9933 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --rpc-cors all \
  --validator
```

Now, you can obtain the node's session key by sending the following RPC payload.

```bash
# send `rotate_keys` request
$ curl -H 'Content-Type: application/json' --data '{ "jsonrpc":"2.0", "method":"author_rotateKeys", "id":1 }' localhost:9933

# should return a long string of hex, which is your session key
{"jsonrpc":"2.0","result":"<session key in hex>","id":1}
```

After this step, you should have a validator node online with a session key for your node.
For key management and validator rewards, consult our [validator guide online](https://docs.astar.network/build/validator-guide/configure-node).

## Further Reading

* [Official Documentation](https://docs.astar.network/)
* [Whitepaper](https://github.com/AstarNetwork/plasmdocs/blob/master/wp/en.pdf)
* [Whitepaper(JP)](https://github.com/AstarNetwork/plasmdocs/blob/master/wp/jp.pdf)
* [Subtrate Developer Hub](https://substrate.dev/docs/en/)
* [Substrate Glossary](https://substrate.dev/docs/en/knowledgebase/getting-started/glossary)
* [Substrate Client Library Documentation](https://polkadot.js.org/docs/)
