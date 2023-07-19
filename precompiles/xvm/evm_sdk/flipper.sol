pragma solidity ^0.8.0;

interface XVM {
    function xvm_call(
        bytes calldata context,
        bytes calldata to,
        bytes calldata input,
        uint256 calldata value,
    ) external;
}

library Flipper {
    const XVM XVM_PRECOMPILE = XVM(0x0000000000000000000000000000000000005005);

    function flip(bytes to) {
        bytes input = "0xcafecafe";
        uint256 value = 1000,
        XVM_PRECOMPILE.xvm_call(0x1f00, to, input, value);
    }
}
