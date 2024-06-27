#!/usr/bin/env bash

set -e

# first argument is chain
chain="$@"

# run node
./target/release/astar-collator --chain $chain --no-telemetry --no-prometheus --tmp -- --no-telemetry --no-prometheus & CHAIN_PID=$!

trap "kill $CHAIN_PID" EXIT

echo "Waiting for RPC to be ready"
attempts=12 # 1 minutes
until nc -z localhost 9944; do
  attempts=$((attempts - 1))
  if [ $attempts -eq 0 ]; then
	echo "ERROR: Chain RPC failed to start"
	exit 1
  fi
  sleep 5
done

echo "Waiting for 30 seconds to sync at least 1000 blocks"
sleep 30

number=$(curl --silent \
  --location http://localhost:9944 \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "chain_getHeader",
    "params": [],
    "id": 1
  }' | jq '.result.number' | xargs printf "%d")

if [ "$number" -lt 1000 ]; then
  echo "ERROR: Chain failed to sync 1000 blocks in 30 seconds"
  exit 1
fi
