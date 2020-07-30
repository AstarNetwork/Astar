#!/usr/bin/env bash

cd balances
cargo contract build
cargo test
cd ../payout/
cargo contract build
cargo test
