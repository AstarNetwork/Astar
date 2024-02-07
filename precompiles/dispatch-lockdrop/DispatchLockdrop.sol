pragma solidity ^0.8.0;

/**
 * @title Dispatch Lockdrop calls interface.
 */

/// Interface to dispatch lockdrop calls precompiled contract
/// Pre-deployed at the address 0x0000000000000000000000000000000000005007
interface RescueLockdrop {
    /**
    * @dev Dispatch lockdrop call
    * @param call - SCALE-encoded call arguments
    * @param pubkey - full ECDSA pubkey 64 bytes
    * @return boolean confirming whether the call got successfully dispatched
    */
    function dispatch_lockdrop_call(
        bytes calldata call,
        bytes calldata pubkey
    ) external returns (bool);
}