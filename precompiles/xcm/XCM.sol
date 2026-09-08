pragma solidity ^0.8.0;

/**
 * @title XCM interface (v1).
 *
 * @dev Every method in this interface is operational. `assets_withdraw` and
 * `assets_reserve_transfer` differ in one respect only: the latter reads the zero address as the
 * native token.
 */
interface XCM {

    /**
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
     *
     * @dev The beneficiary is an `AccountKey20` on the destination. Substrate-native chains
     * generally cannot resolve one - prefer the `bytes32` overload unless the destination accepts
     * it.
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
     * @param parachain_id - destination parachain Id (ignored if is_relay is true)
     * @param is_relay - if true, destination is relay_chain, if false it is parachain (see previous argument)
     * @param payment_asset_id - ETH address of the local asset derivate used to pay for execution in the destination chain, or the zero address for the native token
     * @param payment_amount - amount of payment asset to use for execution payment - should cover cost of XCM instructions + Transact call weight.
     * @param call - encoded call data (must be decodable by remote chain)
     * @param transact_weight - max weight that the encoded call is allowed to consume in the destination chain
     * @return bool confirmation whether the XCM message sent.
     *
     * @dev Sibling parachains only: passing `is_relay = true` reverts. The relay-bound queue is
     * proven in full by every block, so it stays reachable by `Root` alone. `call` is capped at
     * 64 KiB, inside the limit the XCMP transport enforces. `transact_weight` sets the call's
     * ref_time only; its proof size is fixed at 256 KiB, and XCM v5 destinations ignore both.
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
     * @param asset_id - list of XC20 asset addresses, or the zero address for the native token
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
     *
     * @dev As `assets_withdraw`, except the zero address is read as the native token.
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
     *
     * @dev As `assets_reserve_transfer` above, with an `AccountKey20` beneficiary.
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
