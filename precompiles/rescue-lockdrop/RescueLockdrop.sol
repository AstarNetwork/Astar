pragma solidity ^0.8.0;

/**
 * @title Rescue Lockdrop interface.
 */

/// Interface to the rescue lockdrop precompiled contract
/// Predeployed at the address 0x0000000000000000000000000000000000005007
interface RescueLockdrop {
    function claim_lock_drop_account(
        bytes32 accountId,
        bytes signature
    ) external returns (bool);
}
