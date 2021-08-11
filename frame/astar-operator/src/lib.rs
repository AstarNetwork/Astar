#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*; // Import various types used in the pallet definition
    use frame_system::pallet_prelude::*; // Import some system helper types.

    /// X-VM pointer to smart contract instance.
    #[cfg_attr(feature = "std", derive(Debug, Eq))]
    #[derive(Clone, Encode, Decode, PartialEq)]
    pub enum SmartContract<AccountId> {
        /// Wasm smart contract instance.
        Wasm(AccountId),
        /// EVM smart contract instance.
        Evm(sp_core::H160),
    }

    #[pallet::config]
    #[pallet::disable_frame_system_supertrait_check]
    pub trait Config: pallet_contracts::Config + pallet_evm::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Given address isn't a smart contract.
        NotContract,
        /// For given operator contract already assigned.
        OperatorHasContract,
        /// For given contract operator already assigned.
        ContractHasOperator,
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Contract assigned to operator: [operator, contract].
        ContractClaimed(T::AccountId, SmartContract<T::AccountId>),
    }

    /// A mapping from operators to operated contract
    #[pallet::storage]
    #[pallet::getter(fn get_contract)]
    pub(super) type ContractOf<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, SmartContract<T::AccountId>>;

    /// A mapping from operated contract by operator to it.
    #[pallet::storage]
    #[pallet::getter(fn get_operator)]
    pub(super) type OperatorOf<T: Config> =
        StorageMap<_, Blake2_128Concat, SmartContract<T::AccountId>, T::AccountId>;

    #[pallet::pallet]
    #[pallet::generate_store(trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the owner for the given smart contract.
        /// TODO: weight
        #[pallet::weight(1)]
        pub fn claim_contract(
            origin: OriginFor<T>,
            address: SmartContract<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            ensure!(Self::is_contract(&address), Error::<T>::NotContract);
            ensure!(
                !<OperatorOf<T>>::contains_key(&address),
                Error::<T>::ContractHasOperator
            );
            ensure!(
                !<ContractOf<T>>::contains_key(&sender),
                Error::<T>::OperatorHasContract
            );

            <ContractOf<T>>::insert(&sender, address.clone());
            <OperatorOf<T>>::insert(&address, sender.clone());
            Self::deposit_event(Event::ContractClaimed(sender, address));

            Ok(().into())
        }
    }

    // The main implementation block for the module.
    impl<T: Config> Pallet<T> {
        pub fn is_contract(address: &SmartContract<T::AccountId>) -> bool {
            match address {
                SmartContract::Wasm(account) => {
                    <pallet_contracts::ContractInfoOf<T>>::get(&account).is_some()
                }
                SmartContract::Evm(account) => {
                    pallet_evm::Module::<T>::account_codes(&account).len() > 0
                }
            }
        }
    }
}
