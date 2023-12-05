// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.8.0;

/// Predeployed at the address 0x0000000000000000000000000000000000005001
/// For better understanding check the source code:
/// repo: https://github.com/AstarNetwork/Astar
/// code: frame/dapps-staking-v3
interface DappsStaking {

    // TODO: make a custom struct to represent protocol state?

    // TODO: Create a wrapper for smart contract enum, so we can support more than just plain EVM contracts.

    // Storage getters

    /// Describes the subperiod in which the protocol currently is.
    enum Subperiod {Voting, BuildAndEarn}

    /// @notice Get the current protocol state.
    /// @return (current era, current period number, current subperiod type).
    function protocol_state() external view returns (uint256, uint256, Subperiod);

    /// @notice Get the unlocking period expressed in the number of blocks.
    /// @return period: The unlocking period expressed in the number of blocks.
    function unlocking_period() external view returns (uint256);


    // Extrinsic calls

    /// @notice Lock the given amount of tokens into dApp staking protocol.
    function lock(uint128) external;

    /// @notice Start the unlocking process for the given amount of tokens.
    function unlock(uint128) external;

    /// @notice Claims unlocked tokens.
    function claim_unlocked() external;

    /// @notice Stake the given amount of tokens on the specified smart contract.
    function stake(address, uint128) external;

    /// @notice Unstake the given amount of tokens from the specified smart contract.
    function unstake(address, uint128) external;

    /// @notice Claims one or more pending staker rewards.
    function claim_staker_rewards() external;

    /// @notice Claim the bonus reward for the specified smart contract.
    function claim_bonus_reward(address) external;

    /// @notice Claim dApp reward for the specified smart contract & era.
    function claim_dapp_reward(address, uint128) external;

    /// @notice Unstake all funds from the unregistered smart contract.
    function unstake_from_unregistered(address) external;

    /// @notice Used to cleanup all expired contract stake entries from the caller.
    function cleanup_expired_entries() external;
}
