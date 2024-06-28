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
