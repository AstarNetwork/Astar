#!/usr/bin/env bash

set -eux

# Enable warnings about unused extern crates
export RUSTFLAGS=" -W unused-extern-crates"

# Install rustup and the specified rust toolchain.
curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain=stable -y
# Load cargo environment. Specifically, put cargo into PATH.
source ~/.cargo/env
# Install supported nightly
# https://github.com/rustwasm/wasm-bindgen/issues/2009
export NIGHTLY=nightly-2020-02-01
rustup install $NIGHTLY 

rustc --version
rustup --version
cargo --version

case $TARGET in
	"native")

		sudo apt-get -y update
		sudo apt-get install -y cmake libclang-dev

        # Add wasm
        rustup target add wasm32-unknown-unknown --toolchain $NIGHTLY 

        # Install wasm-gc. It's useful for stripping slimming down wasm binaries.
        command -v wasm-gc || \
	        cargo install --git https://github.com/alexcrichton/wasm-gc --force

		cargo test
		;;

	"wasm")

		# Install prerequisites and build all wasm projects
		cargo install pwasm-utils-cli --bin wasm-prune --force

#		cd ./contracts/balances && ./build.sh && cargo test
#		cd ./contracts/cash && ./build.sh && cargo test
#		cd ../commitment && cargo test
#		cd ../deposit && cargo test
#		cd ../predicate && cargo test
		;;

esac
