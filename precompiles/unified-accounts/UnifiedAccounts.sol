pragma solidity ^0.8.0;

/**
 * @title UA interface.
 */

/// Interface to the precompiled contract on Shibuya
/// Predeployed at the address 0x0000000000000000000000000000000000005006
/// For better understanding check the source code:
/// repo: https://github.com/AstarNetwork/astar
/// code: pallets/unified-accounts/src/lib.rs
interface UnifiedAccounts {

    /// Gets the evm address associated with given account id. If no mapping exists, then return the default account id.
    ///
    /// @param accountId: The account id for which you want the evm address for.
    ///
    /// @return value 
    /// If account is mapped to a H160 address:
    /// (mapped address, true)
    /// If not mapped
    /// (default address, false)
    function get_evm_address_or_default(bytes32 calldata accountId) external view returns (address,bool);
    
    /// Gets the account id associated with given evm address. If no mapping exists, then return the default evm address.
    /// @param evmAddress: The evm address for which you want the account id for.
    ///
    /// @return value 
    /// If account is mapped to a SS58 account:
    /// (mapped account bytes, true)
    /// If not mapped
    /// (default account bytes, false)
    function get_native_address_or_default(address evmAddress) external view returns (bytes32,bool);
}