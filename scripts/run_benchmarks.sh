#!/usr/bin/env bash

# This file is part of Substrate.
# Copyright (C) 2022 Parity Technologies (UK) Ltd.
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Customizing Substrate Apache Licensed source code to our needs.
# See original at https://github.com/paritytech/substrate/blob/master/scripts/run_all_benchmarks.sh

# This script has two parts which all use the astar runtime:
# - Pallet benchmarking to update the pallet weights
# - Machine benchmarking

while getopts 'bfp:v' flag; do
  case "${flag}" in
    b)
      # Skip build.
      skip_build='true'
      ;;
    f)
      # Fail if any sub-command in a pipe fails, not just the last one.
      set -o pipefail
      # Fail on undeclared variables.
      set -u
      # Fail if any sub-command fails.
      set -e
      # Fail on traps.
      set -E
      ;;
    p)
      # pallet to execute. separated by ",". use "all" or no input to execute all pallets
      target_pallets="${OPTARG}"
      ;;
    v)
      # Echo all executed commands.
      set -x
      ;;
    *)
      # Exit early.
      echo "Bad options. Check Script."
      exit 1
      ;;
  esac
done


if [ "$skip_build" != true ]
then
  echo "[+] Compiling astar-collator benchmarks..."
  cargo build --release --features=runtime-benchmarks
fi

# The executable to use.
ASTAR_COLLATOR=./target/release/astar-collator

# Manually exclude some pallets.
EXCLUDED_PALLETS=(
  # Pallets without automatic benchmarking
)

# Load all pallet names in an array.
ALL_PALLETS=($(
  $ASTAR_COLLATOR benchmark pallet --list --chain=astar-dev |\
    tail -n+2 |\
    cut -d',' -f1 |\
    sort |\
    uniq
))

# Filter out the excluded pallets by concatenating the arrays and discarding duplicates.
if [ "$target_pallets" == "" ] || [ "$target_pallets" == "all" ]; then
    PALLETS=($({ printf '%s\n' "${ALL_PALLETS[@]}" "${EXCLUDED_PALLETS[@]}"; } | sort | uniq -u))
else
    PALLETS=($({ printf '%s\n' "${target_pallets//,/ }"; } | sort | uniq -u))
fi

echo "[+] Benchmarking ${#PALLETS[@]} Astar collator pallets."

# Define the error file.
ERR_FILE="benchmarking_errors.txt"
# Delete the error file before each run.
rm -f $ERR_FILE

# Benchmark each pallet.
for PALLET in "${PALLETS[@]}"; do
  NAME="$(echo "${PALLET#*_}" | tr '_' '-')";
  # WEIGHT_FILE="./weights/${FOLDER}/weights.rs"
  WEIGHT_FILE="./${NAME}_weights.rs"
  echo "[+] Benchmarking $PALLET with weight file $WEIGHT_FILE";

  OUTPUT=$(
    $ASTAR_COLLATOR benchmark pallet \
    --chain=astar-dev \
    --steps=50 \
    --repeat=20 \
    --pallet="$PALLET" \
    --extrinsic="*" \
    --execution=wasm \
    --wasm-execution=compiled \
    --heap-pages=4096 \
    --output="$WEIGHT_FILE" \
    --template=./scripts/templates/weight-template.hbs 2>&1
  )
  if [ $? -ne 0 ]; then
    echo "$OUTPUT" >> "$ERR_FILE"
    echo "[-] Failed to benchmark $PALLET. Error written to $ERR_FILE; continuing..."
  fi
done

echo "[+] Benchmarking the machine..."
OUTPUT=$(
  $ASTAR_COLLATOR benchmark machine --chain=astar-dev 2>&1
)
if [ $? -ne 0 ]; then
  # Do not write the error to the error file since it is not a benchmarking error.
  echo "[-] Failed the machine benchmark:\n$OUTPUT"
fi

# Check if the error file exists.
if [ -f "$ERR_FILE" ]; then
  echo "[-] Some benchmarks failed. See: $ERR_FILE"
  exit 1
else
  echo "[+] All benchmarks passed."
  exit 0
fi
