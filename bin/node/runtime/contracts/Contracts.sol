// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

interface Contracts {
    function call(bytes32 dest, bytes calldata param) external;
}
