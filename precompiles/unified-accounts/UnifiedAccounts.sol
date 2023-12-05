pragma solidity ^0.8.0;

/**
 * @title UA interface.
 */

/// Interface to the precompiled contract
/// Predeployed at the address 0x0000000000000000000000000000000000005006
/// For better understanding check the source code:
/// repo: https://github.com/AstarNetwork/astar
/// code: pallets/unified-accounts/src/lib.rs
interface UnifiedAccounts {
    /// Gets the evm address associated with given account id. If no mapping exists,
    /// then return the default account id.
    /// @param accountId: The account id for which you want the evm address for.
    /// @return (mapped_address, true) if there is a mapping found otherwise (default_address, false)
    function get_evm_address_or_default(
        bytes32 accountId
    ) external view returns (address, bool);

    /// Gets the account id associated with given evm address. If no mapping exists,
    /// then return the default evm address.
    /// @param evmAddress: The evm address for which you want the account id for.
    /// @return (mapped_account, true) if there is a mapping found otherwise (default_account, false)
    function get_native_address_or_default(
        address evmAddress
    ) external view returns (bytes32, bool);
}
