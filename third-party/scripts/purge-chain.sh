#!/bin/bash
db=$1

if [[ "$OSTYPE" == "linux-gnu" ]]; then
  echo "Clearing local data from home dir: $HOME/.local/share/astar-collator"
	if [[ "$db" == "dev" ]]; then
		rm -rf ~/.local/share/astar-collator/chains/dev/db/
		rm -rf ~/.local/share/astar-collator/chains/development/db/
	elif [[ "$db" == "shiden" ]]; then
		rm -rf ~/.local/share/astar-collator/chains/shiden/db/
	else
		db="all"
		rm -rf ~/.local/share/astar-collator/chains/dev/db/
		rm -rf ~/.local/share/astar-collator/chains/development/db/
		rm -rf ~/.local/share/astar-collator/chains/shiden/db/
		rm -rf ~/.local/share/astar-collator/chains/astar/db/
	fi
elif [[ "$OSTYPE" == "darwin"* ]]; then
  echo "Clearing local data from home dir: $HOME/Library/Application Support/astar-collator"
	if [[ "$db" == "dev" ]]; then
		rm -rf ~/Library/Application\ Support/astar-collator/chains/dev/db/
		rm -rf ~/Library/Application\ Support/astar-collator/chains/development/db/
	elif [[ "$db" == "shiden" ]]; then
		rm -rf ~/Library/Application\ Support/astar-collator/chains/shiden/db/
	else
		db="all"
		rm -rf ~/Library/Application\ Support/astar-collator/chains/dev/db/
		rm -rf ~/Library/Application\ Support/astar-collator/chains/development/db/
		rm -rf ~/Library/Application\ Support/astar-collator/chains/shiden/db/
		rm -rf ~/Library/Application\ Support/astar-collator/chains/astar/db/
	fi
else
  echo "Clearing local data from home dir: $HOME/.local/share/astar-collator"
	if [[ "$db" == "dev" ]]; then
		rm -rf ~/.local/share/astar-collator/chains/dev/db/
		rm -rf ~/.local/share/astar-collator/chains/development/db/
	elif [[ "$db" == "shiden" ]]; then
		rm -rf ~/.local/share/astar-collator/chains/shiden/db/
	else
		db="all"
		rm -rf ~/.local/share/astar-collator/chains/dev/db/
		rm -rf ~/.local/share/astar-collator/chains/development/db/
		rm -rf ~/.local/share/astar-collator/chains/shiden/db/
		rm -rf ~/.local/share/astar-collator/chains/astar/db/
	fi
fi

echo "Deleted $db databases"
