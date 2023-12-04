#!/bin/sh

targets=("register" "unregister" "set_dapp_reward_beneficiary" "set_dapp_owner" "maintenance_mode" \
        "lock" "unlock" "claim_unlocked" "relock_unlocking" \
        "stake" "unstake" "claim_staker_rewards" "claim_bonus_reward" "claim_dapp_reward" \
        "unstake_from_unregistered" "cleanup_expired_entries" "force" )

for target in "${targets[@]}"
do
  cargo tarpaulin -p pallet-dapp-staking-v3 -o=html --output-dir=./coverage/$target -- test::tests::$target
done