pragma solidity ^0.8.0;

/**
 * @title UA interface.
 */

/// TODO: Correct this
/// Interface to the precompiled contract on Shibuya/Shiden/Astar
/// Predeployed at the address 0x0000000000000000000000000000000000005001
/// For better understanding check the source code:
/// repo: https://github.com/AstarNetwork/astar
/// code: pallets/unified-accounts/src/lib.rs
interface XCM {

    //TODO: change it to correct info
    // Gets the evm address associated with given account id, if mapped else None.
    function get_evm_address(bytes calldata account_id) external view returns (address,bool);

    // Gets the evm address associated with given account id. If no mapping exists, then return the default account id.
    function get_evm_address_or_default(bytes calldata account_id) external view returns (address,bool);

    // Gets the account id associated with given evm address, if mapped else None.
    function get_native_address(address evmAddress) external view returns (bytes,bool);
    
    //Gets the account id associated with given evm address. If no mapping exists, then return the default evm address.
    function get_native_address_or_default(address evmAddress) external view returns (bytes,bool);
}