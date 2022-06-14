use sp_runtime::DispatchError;
use sp_std::vec::Vec;

use frame_support::log::{error, trace};
// use pallet_dapps_staking::Call as DappsStaking;
use codec::Encode;

use crate::extension_traits::AstarChainExtension;

pub struct DappsStakingExtension;
impl AstarChainExtension for DappsStakingExtension {
    fn execute_func(func_id: u32) -> Result<Vec<u8>, DispatchError> {
        match func_id {
            // DappsStaking - current_era()
            1 => {
                let current_era = crate::DappsStaking::current_era();
                let current_era_encoded = current_era.encode();
                trace!(
                    target: "runtime",
                    "[ChainExtension]|call|func_id:{:} current_era:{:?}",
                    func_id,
                    &current_era_encoded
                );

                return Ok(current_era_encoded);
            }

            // DappsStaking - general_era_info()
            // 2 => {
            //     let era_info = DappsStaking::general_era_info(arg)
            //         .ok_or(DispatchError::Other("general_era_info call failed"));
            //     let era_info_encoded = era_info.encode();
            //     trace!(
            //         target: "runtime",
            //         "[ChainExtension]|call|func_id:{:} era_info_encoded:{:?}",
            //         func_id,
            //         &era_info_encoded
            //     );

            //     return Ok(era_info_encoded)
            // }
            _ => {
                error!("Called an unregistered `func_id`: {:}", func_id);
                return Err(DispatchError::Other(
                    "DappsStakingExtension: Unimplemented func_id",
                ));
            }
        }
    }
}
