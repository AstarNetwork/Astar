#!/bin/bash
db=$1

if [[ "$OSTYPE" == "linux-gnu" ]]; then
  echo "Clearing local data from home dir: $HOME/.local/share/plasm-node"
	if [[ "$db" == "staging" ]]; then
		rm -rf ~/.local/share/plasm-node/chains/staging_testnet/db/
	elif [[ "$db" == "dev" ]]; then
		rm -rf ~/.local/share/plasm-node/chains/dev/db/
		rm -rf ~/.local/share/plasm-node/chains/development/db/
	elif [[ "$db" == "plasm-node" ]]; then
		rm -rf ~/.local/share/plasm-node/chains/plasm-node/db/
		rm -rf ~/.local/share/plasm-node/chains/plasm-node_testnet/db/
	else
		db="all"
		rm -rf ~/.local/share/plasm-node/chains/dev/db/
		rm -rf ~/.local/share/plasm-node/chains/development/db/
		rm -rf ~/.local/share/plasm-node/chains/plasm-node/db/
		rm -rf ~/.local/share/plasm-node/chains/plasm-node_testnet/db/
		rm -rf ~/.local/share/plasm-node/chains/staging_testnet/db/
		rm -rf ~/.local/share/plasm-node/chains/local_testnet/db/
	fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
  echo "Clearing local data from home dir: $HOME/Library/Application Support/plasm-node"
	if [[ "$db" == "staging" ]]; then
		rm -rf ~/Library/Application\ Support/plasm-node/chains/staging_testnet/db/
	elif [[ "$db" == "dev" ]]; then
		rm -rf ~/Library/Application\ Support/plasm-node/chains/dev/db/
		rm -rf ~/Library/Application\ Support/plasm-node/chains/development/db/
	elif [[ "$db" == "plasm-node" ]]; then
		rm -rf ~/Library/Application\ Support/plasm-node/chains/plasm-node/db/
		rm -rf ~/Library/Application\ Support/plasm-node/chains/plasm-node_testnet/db/
	else
		db="all"
		rm -rf ~/Library/Application\ Support/plasm-node/chains/dev/db/
		rm -rf ~/Library/Application\ Support/plasm-node/chains/development/db/
		rm -rf ~/Library/Application\ Support/plasm-node/chains/plasm-node/db/
		rm -rf ~/Library/Application\ Support/plasm-node/chains/plasm-node_testnet/db/
		rm -rf ~/Library/Application\ Support/plasm-node/chains/staging_testnet/db/
		rm -rf ~/Library/Application\ Support/plasm-node/chains/local_testnet/db/
	fi
else
  echo "Clearing local data from home dir: $HOME/.local/share/plasm-node"
	if [[ "$db" == "staging" ]]; then
		rm -rf ~/.local/share/plasm-node/chains/staging_testnet/db/
	elif [[ "$db" == "dev" ]]; then
		rm -rf ~/.local/share/plasm-node/chains/dev/db/
		rm -rf ~/.local/share/plasm-node/chains/development/db/
	elif [[ "$db" == "plasm-node" ]]; then
		rm -rf ~/.local/share/plasm-node/chains/plasm-node/db/
		rm -rf ~/.local/share/plasm-node/chains/plasm-node_testnet/db/
	else
		db="all"
		rm -rf ~/.local/share/plasm-node/chains/dev/db/
		rm -rf ~/.local/share/plasm-node/chains/development/db/
		rm -rf ~/.local/share/plasm-node/chains/plasm-node/db/
		rm -rf ~/.local/share/plasm-node/chains/plasm-node_testnet/db/
		rm -rf ~/.local/share/plasm-node/chains/staging_testnet/db/
		rm -rf ~/.local/share/plasm-node/chains/local_testnet/db/
	fi
fi

echo "Deleted $db databases"
