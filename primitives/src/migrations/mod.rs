use frame_support::traits::{OnRuntimeUpgrade, StorageVersion};
use frame_support::weights::Weight;
use sp_core::Get;
use sp_std::marker::PhantomData;

pub mod contract_v12_fix;

pub struct ForceContractsVersion<T: pallet_contracts::Config, const V: u16> {
    _phantom: PhantomData<T>,
}

impl<T: pallet_contracts::Config, const V: u16> OnRuntimeUpgrade for ForceContractsVersion<T, V> {
    fn on_runtime_upgrade() -> Weight {
        StorageVersion::new(V).put::<pallet_contracts::Pallet<T>>();
        <T as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
    }
}
