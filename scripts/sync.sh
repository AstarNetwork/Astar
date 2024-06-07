#!/usr/bin/env bash

set -e

# first argument is chain
chain="$@"

# run node
./target/release/astar-collator --chain $chain --no-telemetry --no-prometheus --tmp & CHAIN_PID=$!

printf "Waiting for RPC to be ready"
attempts=12 # 1 minutes
until nc -z localhost 9944; do
  attempts=$((attempts - 1))
  if [ $attempts -eq 0 ]; then
	echo "Chain RPC failed to start"
	exit 1
  fi
  sleep 5
done

printf "Waiting for 30 seconds to sync at least 1000 blocks"
sleep 30

number=$(curl --location http://localhost:9944 \
  --header 'Content-Type: application/json' \
  --data '{
    "jsonrpc": "2.0",
    "method": "chain_getHeader",
    "params": [],
    "id": 1
  }' | jq '.result.number' | xargs | { read hex_number; printf "%d\n" $hex_number; })

if [ "$number" -lt 1000 ]; then
  echo "Chain failed to sync 1000 blocks in 30 seconds"
  exit 1
fi

kill $CHAIN_PID
