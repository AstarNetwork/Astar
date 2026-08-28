
pragma solidity ^0.8.0;


/**
 * @title XCM interface (v2).
 *
 * @dev Both this file and `XCM.sol` document the SAME precompile, registered at `0x...5004` on
 * Astar, Shiden and Shibuya alike - there is no per-network split.
 *
 * Only `transfer(address,uint256,(uint8,bytes[]),(uint64,uint64))` is still operational. Every
 * other method in this interface is DEPRECATED: the selector stays registered so the ABI is
 * unchanged, but calling it always reverts.
 */
interface XCM {
    // A multilocation is defined by its number of parents and the encoded junctions (interior)
    struct Multilocation {
        uint8 parents;
        bytes[] interior;
    }

    struct WeightV2{
        uint64 ref_time;
        uint64 proof_size;
    }

    // A MultiAsset is defined by a multilocation and an amount
    struct MultiAsset {
        Multilocation location;
        uint256 amount;
    }

    // A Currency is defined by address and the amount to be transferred
    struct Currency {
        address currencyAddress;
        uint256 amount;
    }

    /// Transfer a token through XCM based on its address
    ///
    /// @dev The token transfer burns/transfers the corresponding amount before sending
    /// @param currencyAddress The ERC20 address of the currency we want to transfer
    /// @param amount The amount of tokens we want to transfer
    /// @param destination The Multilocation to which we want to send the tokens
    /// @param weight The weight we want to buy in the destination chain, to set the
    /// weightlimit to Unlimited, you should use the value 0 for ref_time
    function transfer(
        address currencyAddress,
        uint256 amount,
        Multilocation memory destination,
        WeightV2 memory weight
    ) external returns (bool);

    /// Transfer a token through XCM based on its address specifying fee
    ///
    /// @dev The token transfer burns/transfers the corresponding amount before sending
    /// @param currencyAddress The ERC20 address of the currency we want to transfer
    /// @param amount The amount of tokens we want to transfer
    /// @param fee The amount to be spent to pay for execution in destination chain
    /// @param destination The Multilocation to which we want to send the tokens
    /// @param weight The weight we want to buy in the destination chain, to set the
    /// weightlimit to Unlimited, you should use the value 0 for ref_time
    /// @custom:deprecated ALWAYS REVERTS. Use `transfer`; the destination charges fees from the transferred asset.
    function transfer_with_fee(
        address currencyAddress,
        uint256 amount,
        uint256 fee,
        Multilocation memory destination,
        WeightV2 memory weight
    ) external returns (bool);

    /// Transfer a token through XCM based on its Location
    ///
    /// @dev The token transfer burns/transfers the corresponding amount before sending
    /// @param asset The asset we want to transfer, defined by its multilocation.
    /// Currently only Concrete Fungible assets
    /// @param amount The amount of tokens we want to transfer
    /// @param destination The Multilocation to which we want to send the tokens
    /// @param weight The weight we want to buy in the destination chain, to set the
    /// weightlimit to Unlimited, you should use the value 0 for ref_time
    /// @custom:deprecated ALWAYS REVERTS. Use `transfer` with the asset's XC20 address.
    function transfer_multiasset(
        Multilocation memory asset,
        uint256 amount,
        Multilocation memory destination,
        WeightV2 memory weight
    ) external returns (bool);

    /// Transfer a token through XCM based on its Location specifying fee
    ///
    /// @dev The token transfer burns/transfers the corresponding amount before sending
    /// @param asset The asset we want to transfer, defined by its multilocation.
    /// Currently only Concrete Fungible assets
    /// @param amount The amount of tokens we want to transfer
    /// @param fee The amount to be spent to pay for execution in destination chain
    /// @param destination The Multilocation to which we want to send the tokens
    /// @param weight The weight we want to buy in the destination chain, to set the
    /// weightlimit to Unlimited, you should use the value 0 for ref_time
    /// @custom:deprecated ALWAYS REVERTS. Use `transfer` with the asset's XC20 address.
    function transfer_multiasset_with_fee(
        Multilocation memory asset,
        uint256 amount,
        uint256 fee,
        Multilocation memory destination,
        WeightV2 memory weight
    ) external returns (bool);

    /// Transfer several tokens at once through XCM based on its address specifying fee
    ///
    /// @dev The token transfer burns/transfers the corresponding amount before sending
    /// @param currencies The currencies we want to transfer, defined by their address and amount.
    /// @param feeItem Which of the currencies to be used as fee
    /// @param destination The Multilocation to which we want to send the tokens
    /// @param weight The weight we want to buy in the destination chain, to set the
    /// weightlimit to Unlimited, you should use the value 0 for ref_time
    /// @custom:deprecated ALWAYS REVERTS. Use `assets_withdraw` from `XCM.sol` for multi-asset transfers.
    function transfer_multi_currencies(
        Currency[] memory currencies,
        uint32 feeItem,
        Multilocation memory destination,
        WeightV2 memory weight
    ) external returns (bool);

    /// Transfer several tokens at once through XCM based on its location specifying fee
    ///
    /// @dev The token transfer burns/transfers the corresponding amount before sending
    /// @param assets The assets we want to transfer, defined by their location and amount.
    /// @param feeItem Which of the currencies to be used as fee
    /// @param destination The Multilocation to which we want to send the tokens
    /// @param weight The weight we want to buy in the destination chain, to set the
    /// weightlimit to Unlimited, you should use the value 0 for ref_time
    /// @custom:deprecated ALWAYS REVERTS. Use `assets_withdraw` from `XCM.sol` for multi-asset transfers.
    function transfer_multi_assets(
        MultiAsset[] memory assets,
        uint32 feeItem,
        Multilocation memory destination,
        WeightV2 memory weight
    ) external returns (bool);

    /**
     * @param destination - Multilocation of destination chain where to send this call
     * @param xcm_call - encoded xcm call you want to send to destination
     *
     * @custom:deprecated ALWAYS REVERTS. It was already unreachable - the runtimes' `SendXcmOrigin` rejects signed origins.
     */
    function send_xcm(
        Multilocation memory destination,
        bytes memory xcm_call
    ) external returns (bool);
}
