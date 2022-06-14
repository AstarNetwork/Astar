use pallet_contracts::chain_extension::{Environment, Ext, InitState, SysConfig, UncheckedFrom};
use sp_runtime::DispatchError;

pub trait AstarChainExtension {
    fn execute_func<G: Ext>(
        func_id: u32,
        env: Environment<G, InitState>,
    ) -> Result<(), DispatchError>
    where
        <G::T as SysConfig>::AccountId: UncheckedFrom<<G::T as SysConfig>::Hash> + AsRef<[u8]>;
}
