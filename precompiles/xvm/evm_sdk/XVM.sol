pragma solidity ^0.8.0;

/**
 * @title XVM interface.
 */
interface XVM {
    /**
     * @dev Execute external VM call
     * @param context - execution context
     * @param to - call recepient
     * @param input - SCALE-encoded call arguments
     * @return success - operation outcome
     * @return data - output data if successful, error data on error
     */
    function xvm_call(
        bytes calldata context,
        bytes calldata to,
        bytes calldata input
    ) external returns (bool success, bytes memory data);
}
