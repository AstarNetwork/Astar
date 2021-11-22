// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

/// Interface to the precompiled contract on Shibuya/Shiden/Astar
/// Predeployed at the address 0x0000000000000000000000000000000000005001
interface AstarBase {

    /// @dev Read Staked amount for the staker
    /// @return Staked amount for the staker
    function is_in_astarbase(address staker) external view returns (uint128);

}
