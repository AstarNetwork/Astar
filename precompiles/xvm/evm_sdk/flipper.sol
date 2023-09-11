pragma solidity ^0.8.0;

interface XVM {
    function xvm_call(
        uint8 vm_id,
        bytes calldata to,
        bytes calldata input,
        uint256 value
    ) external payable returns (bool success, bytes memory data);
}

library Flipper {
    const XVM XVM_PRECOMPILE = XVM(0x0000000000000000000000000000000000005005);

    function flip(bytes to) {
        bytes input = "0xcafecafe";
        XVM_PRECOMPILE.xvm_call(0x1F, to, input, 1000000);
    }
}
