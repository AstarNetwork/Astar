#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*; // reexport in crate namespace for `construct_runtime!`

#[frame_support::pallet]
// NOTE: The name of the pallet is provided by `construct_runtime` and is used as
// the unique identifier for the pallet's storage. It is not defined in the pallet itself.
pub mod pallet {
	use frame_support::pallet_prelude::*; // Import various types used in the pallet definition
	use frame_system::pallet_prelude::*; // Import some system helper types.

    #[pallet::pallet]
    #[pallet::generate_store(trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
	pub trait Config: frame_system::Config {
        /// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

    }

    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

        /// Sets an operator for the given contracts.
        #[pallet::weight(0)] // TODO
        pub(super) fn set_operator(
            origin: OriginFor<T>,
            contract: T::AccountId,
            new_operator: T::AccountId
        ) -> DispatchResultWithPostInfo {
            let operator = ensure_signed(origin)?;
            // add operator to contracts
            <OperatorHasContracts<T>>::mutate(&operator, |tree| (*tree)
                                                                .push(contract.clone()));

            // add contract to operator
            <ContractHasOperator<T>>::insert(&contract, operator.clone());

            Self::deposit_event(Event::SetOperator(new_operator, contract));
            Ok(().into())
        }
	}

	// Declare pallet Event enum.
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        /// Operator set (Operator AccountId, Contract AccountId).
        SetOperator(T::AccountId, T::AccountId),
        
	}

    /// A mapping from operators to operated contracts by them.
	#[pallet::storage]
	#[pallet::getter(fn get_contract)]
    pub(super) type OperatorHasContracts<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, Vec<T::AccountId> >;

    /// A mapping from operated contract by operator to it.
    #[pallet::storage]
	#[pallet::getter(fn get_operator)]
    pub(super) type ContractHasOperator<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId >;
        
}