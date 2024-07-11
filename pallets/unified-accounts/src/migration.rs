use super::{Config, Pallet, Weight};
use astar_primitives::evm::EvmAddress;
use frame_support::{
    pallet_prelude::OptionQuery,
    storage_alias,
    traits::{Get, OnRuntimeUpgrade},
    Blake2_128Concat,
};

#[storage_alias]
type EvmToNative<T: Config> = StorageMap<
    Pallet<T>,
    Blake2_128Concat,
    <T as frame_system::Config>::AccountId,
    EvmAddress,
    OptionQuery,
>;

#[storage_alias]
type NativeToEvm<T: Config> = StorageMap<
    Pallet<T>,
    Blake2_128Concat,
    EvmAddress,
    <T as frame_system::Config>::AccountId,
    OptionQuery,
>;

/// Remove all corrupted mappings.
pub struct ClearCorruptedUnifiedMappings<T>(core::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for ClearCorruptedUnifiedMappings<T> {
    fn on_runtime_upgrade() -> Weight {
        let healthy_count = crate::EvmToNative::<T>::iter().count() as u64 * 2;
        log::info!("Total healthy entries: {healthy_count}");

        let mut count = 0;
        // translate will fail to decode valid entries and therefore will skip it,
        // so this will remove only corrupt entries
        EvmToNative::<T>::translate(|key, value: EvmAddress| {
            log::debug!("Remove corrupt key: {key:?} with value: {value:?}");
            count += 1;
            None
        });
        NativeToEvm::<T>::translate(|key, value: T::AccountId| {
            log::debug!("Remove corrupt key: {key:?} with value: {value:?}");
            count += 1;
            None
        });
        log::info!("Removed {count} corrupt entries");
        T::DbWeight::get().reads_writes(healthy_count + count, count)
    }
}
