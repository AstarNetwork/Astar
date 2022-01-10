#!/bin/bash
set -e

FILE="astar-graph"
SERVICE="astar-graph"

function usage {
   cat << EOF
Usage: graph.sh -chain <chain name> -rpc-url <RPC url>

Runs Graph node from docker-compose settings configured from input
EOF
   exit 1
}

if [ $# -ne 4 ]; then
   usage;
fi

if [ "$1" != "-chain" ]; then
  usage;
fi

if [ "$3" != "-rpc-url"]; then
  usage;
fi

Chain=$2
RPC=$4

git clone https://github.com/graphprotocol/graph-node/ \
&& cd graph-node/docker
sudo bash ./setup.sh

sed -Ei "s|mainnet:http://172.19.0.1:8545|$Chain:$RPC|g" docker-compose.yml

sudo docker-compose up
