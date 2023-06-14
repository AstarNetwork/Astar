pragma solidity ^0.8.0;

/**
 * @title XCM interface.
 */
interface XCM {
    /**
     * @dev Withdraw assets using PalletXCM call.
     * @param asset_id - list of XC20 asset addresses
     * @param asset_amount - list of transfer amounts (must match with asset addresses above)
     * @param recipient_account_id - SS58 public key of the destination account
     * @param is_relay - set `true` for using relay chain as reserve
     * @param parachain_id - set parachain id of reserve parachain (when is_relay set to false)
     * @param fee_index - index of asset_id item that should be used as a XCM fee
     * @return bool confirmation whether the XCM message sent.
     *
     * How method check that assets list is valid:
     * - all assets resolved to multi-location (on runtime level)
     * - all assets has corresponded amount (lenght of assets list matched to amount list)
     */
    function assets_withdraw(
        address[] calldata asset_id,
        uint256[] calldata asset_amount,
        bytes32   recipient_account_id,
        bool      is_relay,
        uint256   parachain_id,
        uint256   fee_index
    ) external returns (bool);

    /**
     * @dev Withdraw assets using PalletXCM call.
     * @param asset_id - list of XC20 asset addresses
     * @param asset_amount - list of transfer amounts (must match with asset addresses above)
     * @param recipient_account_id - ETH address of the destination account
     * @param is_relay - set `true` for using relay chain as reserve
     * @param parachain_id - set parachain id of reserve parachain (when is_relay set to false)
     * @param fee_index - index of asset_id item that should be used as a XCM fee
     * @return bool confirmation whether the XCM message sent.
     *
     * How method check that assets list is valid:
     * - all assets resolved to multi-location (on runtime level)
     * - all assets has corresponded amount (lenght of assets list matched to amount list)
     */
    function assets_withdraw(
        address[] calldata asset_id,
        uint256[] calldata asset_amount,
        address   recipient_account_id,
        bool      is_relay,
        uint256   parachain_id,
        uint256   fee_index
    ) external returns (bool);

    /**
     * @dev Execute a transaction on a remote chain.
     * @param parachain_id - destination parachain Id (ignored if is_relay is true)
     * @param is_relay - if true, destination is relay_chain, if false it is parachain (see previous argument)
     * @param payment_asset_id - ETH address of the local asset derivate used to pay for execution in the destination chain
     * @param payment_amount - amount of payment asset to use for execution payment - should cover cost of XCM instructions + Transact call weight.
     * @param call - encoded call data (must be decodable by remote chain)
     * @param transact_weight - max weight that the encoded call is allowed to consume in the destination chain
     * @return bool confirmation whether the XCM message sent.
     */
    function remote_transact(
        uint256 parachain_id,
        bool is_relay,
        address payment_asset_id,
        uint256 payment_amount,
        bytes calldata call,
        uint64 transact_weight
    ) external returns (bool);

    /**
     * @dev Reserve transfer assets using PalletXCM call.
     * @param asset_id - list of XC20 asset addresses
     * @param asset_amount - list of transfer amounts (must match with asset addresses above)
     * @param recipient_account_id - SS58 public key of the destination account
     * @param is_relay - set `true` for using relay chain as destination
     * @param parachain_id - set parachain id of destination parachain (when is_relay set to false)
     * @param fee_index - index of asset_id item that should be used as a XCM fee
     * @return A boolean confirming whether the XCM message sent.
     *
     * How method check that assets list is valid:
     * - all assets resolved to multi-location (on runtime level)
     * - all assets has corresponded amount (lenght of assets list matched to amount list)
     */
    function assets_reserve_transfer(
        address[] calldata asset_id,
        uint256[] calldata asset_amount,
        bytes32   recipient_account_id,
        bool      is_relay,
        uint256   parachain_id,
        uint256   fee_index
    ) external returns (bool);

    /**
     * @dev Reserve transfer using PalletXCM call.
     * @param asset_id - list of XC20 asset addresses
     * @param asset_amount - list of transfer amounts (must match with asset addresses above)
     * @param recipient_account_id - ETH address of the destination account
     * @param is_relay - set `true` for using relay chain as destination
     * @param parachain_id - set parachain id of destination parachain (when is_relay set to false)
     * @param fee_index - index of asset_id item that should be used as a XCM fee
     * @return A boolean confirming whether the XCM message sent.
     *
     * How method check that assets list is valid:
     * - all assets resolved to multi-location (on runtime level)
     * - all assets has corresponded amount (lenght of assets list matched to amount list)
     */
    function assets_reserve_transfer(
        address[] calldata asset_id,
        uint256[] calldata asset_amount,
        address   recipient_account_id,
        bool      is_relay,
        uint256   parachain_id,
        uint256   fee_index
    ) external returns (bool);
}
