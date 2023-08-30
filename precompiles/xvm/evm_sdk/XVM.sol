pragma solidity ^0.8.0;

/**
 * @title XVM interface.
 */
interface XVM {
    /**
     * @dev Execute external VM call
     * @param vm_id - target VM id
     * @param to - call recipient
     * @param input - SCALE-encoded call arguments
     * @param value - value to transfer
     * @param storage_deposit_limit - storage deposit limit, use 0 for unlimited.
     * @return success - operation outcome
     * @return data - output data if successful, error data on error
     */
    function xvm_call(
        uint8 vm_id,
        bytes calldata to,
        bytes calldata input,
        uint256 value,
        uint256 storage_deposit_limit
    ) external payable returns (bool success, bytes memory data);
}
