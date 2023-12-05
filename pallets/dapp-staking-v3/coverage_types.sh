#!/bin/sh

targets=("protocol_state" "account_ledger" "dapp_info" "period_info" "era_info" \
        "stake_amount" "singular_staking_info" "contract_stake_amount" "era_reward_span" \
        "period_end_info" "era_stake_pair_iter" "tier_threshold" "tier_params" "tier_configuration" \
        "dapp_tier_rewards" )

for target in "${targets[@]}"
do
  cargo tarpaulin -p pallet-dapp-staking-v3 -o=html --output-dir=./coverage/$target -- $target
done

# Also need to check the coverage when only running extrinsic tests (disable type tests)

# Also need similar approach to extrinsic testing, as above


# NOTE: this script will be deleted before the final release!