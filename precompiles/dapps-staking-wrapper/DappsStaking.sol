// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

interface DappsStaking {
    /// @dev Get current era.
    /// Selector: d7be3896
    /// @return The current era
    function current_era() external view returns (uint256);

    /// @dev Get Reward and Stakeed amount for given era.
    /// Selector: b9b70e8e
    /// @return The Reward and Staked amount for given era
    function era_reward_and_stake(uint32 era) external view returns (uint128, uint128);
}
