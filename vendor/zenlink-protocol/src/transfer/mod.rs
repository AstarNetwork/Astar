// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

use super::*;

impl<T: Config> Pallet<T> {
    // Check the native currency must be more than ExistentialDeposit,
    // other assets always return true
    pub(crate) fn check_existential_deposit(
        asset_id: AssetId,
        amount: AssetBalance,
    ) -> Option<bool> {
        T::TargetChains::get()
            .iter()
            .find(|(l, _)| *l == make_x2_location(asset_id.chain_id))
            .map(|&(_, minimum_balance)| amount >= minimum_balance || asset_id.asset_type != NATIVE)
    }

    // Make the deposit foreign order
    fn make_deposit_asset_order(recipient: MultiLocation) -> Order<()> {
        Order::DepositAsset { assets: vec![MultiAsset::All], dest: recipient }
    }

    // Transfer zenlink assets which are native to this parachain
    pub(crate) fn make_xcm_lateral_transfer_native(
        location: MultiLocation,
        para_id: ParaId,
        recipient: MultiLocation,
        amount: AssetBalance,
    ) -> Xcm<T::Call> {
        Xcm::WithdrawAsset {
            assets: vec![MultiAsset::ConcreteFungible { id: location, amount }],
            effects: vec![Order::DepositReserveAsset {
                assets: vec![MultiAsset::All],
                dest: make_x2_location(para_id.into()),
                effects: vec![Self::make_deposit_asset_order(recipient)],
            }],
        }
    }
    // Transfer zenlink assets which are foreign to this parachain
    pub(crate) fn make_xcm_lateral_transfer_foreign(
        reserve_chain: ParaId,
        location: MultiLocation,
        para_id: ParaId,
        recipient: MultiLocation,
        amount: AssetBalance,
    ) -> Xcm<T::Call> {
        Xcm::WithdrawAsset {
            assets: vec![MultiAsset::ConcreteFungible { id: location, amount }],
            effects: vec![Order::InitiateReserveWithdraw {
                assets: vec![MultiAsset::All],
                reserve: make_x2_location(reserve_chain.into()),
                effects: vec![if para_id == reserve_chain {
                    Self::make_deposit_asset_order(recipient)
                } else {
                    Order::DepositReserveAsset {
                        assets: vec![MultiAsset::All],
                        dest: make_x2_location(para_id.into()),
                        effects: vec![Self::make_deposit_asset_order(recipient)],
                    }
                }],
            }],
        }
    }

    pub(crate) fn make_xcm_transfer_to_parachain(
        asset_id: &AssetId,
        para_id: ParaId,
        recipient: MultiLocation,
        amount: AssetBalance,
    ) -> Result<Xcm<T::Call>, XcmError> {
        if !asset_id.is_support() {
            return Err(XcmError::FailedToTransactAsset("Invalid AssetId"));
        }

        let asset_location = MultiLocation::X4(
            Junction::Parent,
            Junction::Parachain(asset_id.chain_id),
            Junction::PalletInstance(asset_id.asset_type),
            Junction::GeneralIndex { id: asset_id.asset_index as u128 },
        );

        let seld_chain_id: u32 = T::SelfParaId::get();
        if asset_id.chain_id == seld_chain_id {
            Ok(Self::make_xcm_lateral_transfer_native(asset_location, para_id, recipient, amount))
        } else {
            Ok(Self::make_xcm_lateral_transfer_foreign(
                ParaId::from(asset_id.chain_id),
                asset_location,
                para_id,
                recipient,
                amount,
            ))
        }
    }
}
