#!/bin/bash

set -e

if [ $(arch) != "x86_64" ]
then
  echo "Runs only on x86_64 architecture"
  exit 1
fi

if ! command -v ./astar-collator &> /dev/null
then
  echo "No executable astar-collator binary in zombienet directory"
  exit 1
fi

ZOMBINET_VERSION=v1.3.106

if ! command -v zombienet &> /dev/null
then
    echo "Install zombienet $ZOMBINET_VERSION"
    mkdir -p $HOME/.local/bin
    wget -q -O $HOME/.local/bin/zombienet https://github.com/paritytech/zombienet/releases/download/$ZOMBINET_VERSION/zombienet-linux-x64
    chmod a+x $HOME/.local/bin/zombienet
    PATH=$HOME/.local/bin:$PATH
    zombienet version
fi

echo "Pull polkadot binaries"
zombienet setup polkadot -y & SETUP_PID=$!
while ps $SETUP_PID > /dev/null ; do
    sleep 1
done
chmod +x polkadot polkadot-execute-worker polkadot-prepare-worker

# default to shibuya-dev
if [[ ! -v CHAIN ]]; then
  export CHAIN="shibuya-dev"
fi

echo "Start zombienet for $CHAIN"
echo "NOTE: Select chain using environmental variable CHAIN=<shibuya-dev|shiden-dev|astar-dev> to change it."
nohup zombienet -p native spawn smoke.toml & ZOMBIENET_PID=$!

# kill zombienet before exit
trap "kill $ZOMBIENET_PID" EXIT

echo "Waiting for RPC to be ready"
attempts=12 # 2 minutes
until nc -z localhost 9944; do
  attempts=$((attempts - 1))
  if [ $attempts -eq 0 ]; then
    echo "ERROR: Chain RPC failed to start"
    exit 1
  fi
  printf "."
  sleep 10
done

echo "RPC is ready"

number=0
attempts=20 # 200s
while [ $number -lt 5 ]; do
  attempts=$((attempts - 1))
  if [ $attempts -eq 0 ]; then
    echo "ERROR: Parachain failed to build 5 blocks in 200s"
    exit 1
  fi

  sleep 10

  number=$(curl --silent \
    --location http://localhost:9944 \
    --header 'Content-Type: application/json' \
    --data '{
      "jsonrpc": "2.0",
      "method": "chain_getHeader",
      "params": [],
      "id": 1
    }' | jq '.result.number' | xargs printf "%d")

  echo "Parachain block number $number"
done
