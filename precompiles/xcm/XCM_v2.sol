pragma solidity ^0.8.0;

/**
 * @title XCM interface.
 */
interface XCM {
    // A multilocation is defined by its number of parents and the encoded junctions (interior)
    struct Multilocation {
        uint8 parents;
        bytes[] interior;
    }
    
    /**
     * @dev Withdraw assets using PalletXCM call.
     * @param asset_id - list of XC20 asset addresses
     * @param asset_amount - list of transfer amounts (must match with asset addresses above)
     * @param beneficiary - Multilocation of beneficiary in respect to destination parachain
     * @param destination - Multilocation of destination chain
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
        Multilocation memory beneficiary,
        Multilocation memory destination,
        uint256   fee_index
    ) external returns (bool);

    /**
     * @dev Execute a transaction on a remote chain.
     * @param destination - Multilocation of destination chain
     * @param payment_asset_id - ETH address of the local asset derivate used to pay for execution in the destination chain
     * @param payment_amount - amount of payment asset to use for execution payment - should cover cost of XCM instructions + Transact call weight.
     * @param call - encoded call data (must be decodable by remote chain)
     * @param transact_weight - max weight that the encoded call is allowed to consume in the destination chain
     * @return bool confirmation whether the XCM message sent.
     */
    function remote_transact(
        Multilocation memory destination,
        address payment_asset_id,
        uint256 payment_amount,
        bytes calldata call,
        uint64 transact_weight
    ) external returns (bool);

    /**
     * @dev Reserve transfer assets using PalletXCM call.
     * @param asset_id - list of XC20 asset addresses
     * @param asset_amount - list of transfer amounts (must match with asset addresses above)
     * @param beneficiary - Multilocation of beneficiary in respect to destination parachain
     * @param destination - Multilocation of destination chain
     * @param fee_index - index of asset_id item that should be used as a XCM fee
     * @return A boolean confirming whether the XCM message sent.
     * How method check that assets list is valid:
     * - all assets resolved to multi-location (on runtime level)
     * - all assets has corresponded amount (lenght of assets list matched to amount list)
     */
    function assets_reserve_transfer(
        address[] calldata asset_id,
        uint256[] calldata asset_amount,
        Multilocation memory beneficiary,
        Multilocation memory destination,
        uint256   fee_index
    ) external returns (bool);

    /**
     * @dev send xcm using PalletXCM call.
     * @param destination - Multilocation of destination chain where to send this call
     * @param xcm_call - encoded xcm call you want to send to destination
     **/
    function send_xcm(
        Multilocation memory destination,
        bytes memory xcm_call
    ) external returns (bool);
}