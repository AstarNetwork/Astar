#!/bin/bash

set -e

if [ $(arch) != "x86_64" ]
then
  echo "Runs only on x86_64 architecture"
  exit 1
fi

if ! [ -x ./astar-collator ] && ( ! [ -x ./astar-collator-1 ] || ! [ -x ./astar-collator-2 ] )
then
  echo "No astar-collator binary found: expected ./astar-collator or both ./astar-collator-1 and ./astar-collator-2"
  exit 1
fi

ZOMBIENET_VERSION=v1.3.133

if ! command -v zombienet &> /dev/null
then
    echo "Install zombienet $ZOMBIENET_VERSION"
    mkdir -p $HOME/.local/bin
    wget -q -O $HOME/.local/bin/zombienet https://github.com/paritytech/zombienet/releases/download/$ZOMBIENET_VERSION/zombienet-linux-x64
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
