// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

/// Interface to the precompiled contract on Shibuya/Shiden/Astar
/// Predeployed on the address 0x0000000000000000000000000000000000005001
interface DappsStaking {
    /// @dev Get current era.
    /// Selector: d7be3896
    /// @return The current era
    function current_era() external view returns (uint256);

    /// @dev Get Reward and Stakeed amount for given era.
    /// Selector: b9b70e8e
    /// @return The Reward and Staked amount for given era
    function era_reward_and_stake(uint32 era) external view returns (uint128, uint128);

    /// @dev Get Contract address for the provided H160 accountId.
    /// Selector: 192fb256
    /// @return address as uint256, not as H160
    function registered_contract(address developer) external view returns (uint256);

    /// @dev Register provided contract.
    /// Selector: 4420e486
    function register(address) external;

    /// @dev Stake provided amount on the contract.
    /// Selector: 52b73e41
    function bond_and_stake(address, uint128) external;
}
