use sp_runtime::DispatchError;
// use pallet_contracts::{
//     chain_extension::{
//         RetVal, InitState, Environment, Ext, SysConfig, UncheckedFrom,
//     },
// };
use sp_std::vec::Vec;

pub trait AstarChainExtension {
    fn execute_func(func_id: u32) -> Result<Vec<u8>, DispatchError>;
    // where
    // <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
}
