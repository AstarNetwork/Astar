// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

/// Interface to the precompiled contract on Shibuya/Shiden/Astar
/// Predeployed at the address 0x0000000000000000000000000000000000005001
interface DappsStaking {

    // Storage getters

    /// @dev Read current era.
    /// @return The current era
    function read_current_era() external view returns (uint256);

    /// @dev Read unbonding period constant.
    /// @return The unbonding period
    function read_unbonding_period() external view returns (uint256);

    /// @dev Read Total network reward for the given era
    /// @return Total network reward for the given era
    function read_era_reward(uint32 era) external view returns (uint128);

    /// @dev Read Total staked amount for the given era
    /// @return Total staked amount for the given era
    function read_era_staked(uint32 era) external view returns (uint128);

    /// @dev Read Staked amount for the staker
    /// @return Staked amount for the staker
    function read_staked_amount(address staker) external view returns (uint128);

    /// @dev Read the amount staked on contract in the given era
    /// @return The amount staked on contract in the given era
    function read_contract_era_stake(address contract_id, uint32 era) external view returns (uint128);


    // Extrinsic calls

    /// @dev Register provided contract.
    function register(address) external;

    /// @dev Stake provided amount on the contract.
    function bond_and_stake(address, uint128) external;

    /// @dev Start unbonding process and unstake balance from the contract.
    function unbond_and_unstake(address, uint128) external;

    /// @dev Withdraw all funds that have completed the unbonding process.
    function withdraw_unbonded() external;

    /// @dev Claim contract's rewards.
    function claim(address, uint128) external;
}
