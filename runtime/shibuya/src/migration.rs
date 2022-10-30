use frame_support::{traits::StorageVersion, weights::Weight};
use sp_core::Get;
use sp_std::{marker::PhantomData, prelude::Vec, vec};

pub struct ContractsStorageVersionMigration<T: pallet_contracts::Config>(PhantomData<T>);

impl<T: pallet_contracts::Config> frame_support::traits::OnRuntimeUpgrade
    for ContractsStorageVersionMigration<T>
{
    fn on_runtime_upgrade() -> Weight {
        let version = StorageVersion::get::<pallet_contracts::Pallet<T>>();
        let mut weight = Weight::zero();

        if version < 7 {
            StorageVersion::new(7).put::<pallet_contracts::Pallet<T>>();
            weight = weight.saturating_add(T::DbWeight::get().writes(1));
        }

        weight
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        let version = StorageVersion::get::<pallet_contracts::Pallet<T>>();
        log::info!("Pre upgrade StorageVersion: {:?}", version);
        Ok(vec![])
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
        let version = StorageVersion::get::<pallet_contracts::Pallet<T>>();
        log::info!("Post upgrade StorageVersion: {:?}", version);
        Ok(())
    }
}
