// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

interface Staking {
    /*
     * @dev Set session keys of function caller.
     */
    function set_keys(bytes calldata keys) external;

    /*
     * @dev Removes any session keys of the function caller.
     */
    function purge_keys() external;

    /*
     * @dev Register function caller as collation candidate.
     * @note Collation staking deposit will be locked.
     */
    function register_as_candidate() external;
}
