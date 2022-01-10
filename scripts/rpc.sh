#!/bin/bash
set -e

FILE="astar-rpc"
SERVICE="astar-rpc"

function usage {
   cat << EOF
Usage: rpc.sh -auth <ngrok auth token>

Runs RPC node from docker with ngrok tunnel setting
EOF
   exit 1
}

if [ $# -ne 2 ]; then
   usage;
fi

if [ "$1" != "-auth" ]; then
  usage;
fi

AUTH=$2

# get container
sudo docker pull staketechnologies/astar-collator

# command to run RPC node - do not forget to change NAME to whatever you like
docker run -m 5G --name Shiden -p 30334:30334 -p 30333:30333 -p 9933:9933 -p 9944:9944 \
-v "/var/lib/astar/shiden-db:/data" \
-u $(id -u ${USER}):$(id -g ${USER}) -d --network=host staketechnologies/astar-collator \
astar-collator --name NAME --base-path /data --port 30333 --rpc-port 9933 --unsafe-rpc-external --unsafe-ws-external --pruning archive \
--state-cache-size 1 --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
-l evm=debug,ethereum=debug,rpc=debug \

sudo snap install ngrok

ngrok authtoken $AUTH
ngrok http 9933
