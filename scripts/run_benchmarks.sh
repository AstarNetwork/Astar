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

while getopts 'bc:fo:p:v' flag; do
  case "${flag}" in
    b)
      # Skip build.
      skip_build='true'
      ;;
    c)
      chains=$(echo ${OPTARG} | tr '[:upper:]' '[:lower:]')
      chains_default=("astar" "shiden" "shibuya")
      for chain in ${chains//,/ }; do
        if [[ ! " ${chains_default[*]} " =~ " ${chain} " ]]; then
          echo "Chain input is invalid. ${chain} not included in ${chains_default[*]}"
          exit 1
        fi
      done
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
    o)
      # output folder path
      output_path="${OPTARG}"
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
      echo ${flag}
      exit 1
      ;;
  esac
done


if [ "$skip_build" != true ]
then
  echo "[+] Compiling astar-collator benchmarks..."
  CARGO_PROFILE_RELEASE_LTO=true RUSTFLAGS="-C codegen-units=1" cargo build --release --verbose --features=runtime-benchmarks \
  -p astar-runtime -p shiden-runtime -p shibuya-runtime
fi

# The executable to use.
BENCHMARK_TOOL="frame-omni-bencher v1"

# Manually exclude some pallets.
EXCLUDED_PALLETS=(
  # Pallets without automatic benchmarking
)

# Load all pallet names in an array.
ALL_PALLETS=($(
  $BENCHMARK_TOOL benchmark pallet --list --runtime ./target/release/wbuild/${chain}-runtime/${chain}_runtime.compact.compressed.wasm |\
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

ERR_RC=0
ERR_FILES=""
for chain in ${chains//,/ }; do
    mkdir $output_path/$chain
    # Define the error file.
    ERR_FILE="$output_path/$chain/bench_errors.txt"
    # Delete the error file before each run.
    rm -f $ERR_FILE

    # Benchmark each pallet.
    for PALLET in "${PALLETS[@]}"; do
      NAME_PRFX=${PALLET#*_}
      NAME=${NAME_PRFX//::/_}
      # WEIGHT_FILE="./weights/${FOLDER}/weights.rs"
      WEIGHT_FILE="$output_path/$chain/${NAME}_weights.rs"
      echo "[+] Benchmarking $PALLET with weight file $WEIGHT_FILE";

      OUTPUT=$(
        $BENCHMARK_TOOL benchmark pallet \
        --runtime ./target/release/wbuild/${chain}-runtime/${chain}_runtime.compact.compressed.wasm \
        --steps=50 \
        --repeat=20 \
        --pallet="$PALLET" \
        --extrinsic="*" \
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

    # Disabled for now - command isn't available (yet).
    # With latest changes we also benchmark the HW each time the client starts.
    #
    # echo "[+] Benchmarking the machine..."
    # OUTPUT=$(
    #   $BENCHMARK_TOOL benchmark machine --runtime ./target/release/wbuild/astar-runtime/${chain}_runtime.compact.compressed.wasm 2>&1
    # )
    # if [ $? -ne 0 ]; then
    #   echo "[-] Failed the machine benchmark"
    #   echo "$OUTPUT" >> "$ERR_FILE"
    # else
    #   echo "$OUTPUT" >> "$output_path/$chain/$chain-machine-bench.txt"
    # fi

    # Check if the error file exists.
    if [ -f "$ERR_FILE" ]; then
      ERR_FILES="$ERR_FILES $ERR_FILE"
      ERR_RC=1
    fi
done

if [ $ERR_RC -ne 0 ]; then
    echo "[-] Benchmarks failed: $ERR_FILES"
else
    echo "[+] All benchmarks passed."
fi
