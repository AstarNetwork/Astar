// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

interface DappsStaking {
    /*
     * @dev Get current era.
     */
    /// @dev Get current era.
    /// Selector: d7be3896
    /// @return The current era
    function current_era() external view returns (uint256);
}
