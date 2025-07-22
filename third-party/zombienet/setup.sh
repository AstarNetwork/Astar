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
for i in {1..3}; do
    zombienet setup polkadot -y & SETUP_PID=$!
    while ps $SETUP_PID > /dev/null ; do
        sleep 1
    done

    if [ -f "polkadot" ] && [ -f "polkadot-execute-worker" ] && [ -f "polkadot-prepare-worker" ]; then
        break
    else
        sleep $((i * 10))
        if [ $i -eq 3 ]; then
            echo "❌ Failed to setup Zombienet and polkadot binaries after 3 attempts"
            exit 1
        fi
    fi
done
chmod +x polkadot polkadot-execute-worker polkadot-prepare-worker
