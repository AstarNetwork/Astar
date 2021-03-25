// the overlap with existing functionality isn’t required and better to escape it for future compatibility. I mean implementing additional one method for contract instance creation. Rather than let’s make a method for assign exist contract instance with an operator address. For this the special trait like IsContract will be required and implemented for the runtime.
// EVM contract operators also should be presented. For this reason IsContract instance could be extended for H160 smart contract addresses.
// contract transfers is implemented in trading module and shouldn’t be here.
// set_parameters is also overlap and not required more.
// The final concept is:

// pub trait IsContract {
// fn is_contract(&Self) -> bool;
// }

// instance IsContract H160 {
// ...
// }

// instance IsContract AccountId {
// ...
// }

// And method spec for set operator is:

// pub enum SmartContract {
// Wasm(AccountId),
// EVM(H160),
// }

// fn claim_operator(origin, contract: SmartContract<T::AccountId>)

// where claim_operator assign smart contract operator from new smart contract (contract without an operator) to transaction sender (it used for escaping operator address validation).


#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*; // reexport in crate namespace for `construct_runtime!`
use sp_std::{prelude::*};

#[frame_support::pallet]
// NOTE: The name of the pallet is provided by `construct_runtime` and is used as
// the unique identifier for the pallet's storage. It is not defined in the pallet itself.
pub mod pallet {
    use frame_support::pallet_prelude::*; // Import various types used in the pallet definition
    use frame_system::pallet_prelude::*; // Import some system helper types.
    use sp_runtime::DispatchError;
    use sp_core::{H160};
    use super::*;
    
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
    pub enum SmartContract<T: pallet::Config>
    {
        Wasm(T::AccountId),
        EVM(H160),
    }
    #[pallet::pallet]
    #[pallet::generate_store(trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::config]
	pub trait Config: frame_system::Config {
        /// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }
    
    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}
    
	#[pallet::call]
	impl<T: Config> Pallet<T> {
        /// Sets the owner for the given smart contract.
        #[pallet::weight(0)] // TODO
        pub(super) fn claim_contract(
            origin: OriginFor<T>,
            contract: SmartContract<T>,
        ) -> DispatchResultWithPostInfo {
            let operator = ensure_signed(origin)?;
            if !<ContractHasOperator<T>>::contains_key(&contract) &&
                !<OperatorHasContract<T>>::contains_key(&operator) {
                    // add owner of the contracts
                    <OperatorHasContract<T>>::insert(&operator, contract.clone());
                    // assigns the contract to owner for staking purposes
                    <ContractHasOperator<T>>::insert(&contract, operator.clone());
                    
                    Self::deposit_event(Event::ContractClaimed(operator, contract));
                }
            Ok(().into())
        }
	}

    // Declare pallet Event enum.
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        /// Claim Contract (Owner AccountId, Contract AccountId).
        ContractClaimed(T::AccountId, SmartContract<T>),

	}

    /// A mapping from operators to operated contract
	#[pallet::storage]
	#[pallet::getter(fn get_contract_wasm)]
    pub(super) type OperatorHasContract<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, SmartContract<T> >; 

    /// A mapping from operated contract by operator to it.
    #[pallet::storage]
	#[pallet::getter(fn get_operator)]
    pub(super) type ContractHasOperator<T: Config> = StorageMap<_, Blake2_128Concat, SmartContract<T>, T::AccountId >;
        
    pub trait IsContract<T: Config> {
        //type SmartContract;

        fn claim_contract(origin: OriginFor<T>, contract: SmartContract<T>) -> Result<bool, DispatchError>;
    }
    /// implement isContract for SmartContract 
    impl<T: Config> IsContract<T> for SmartContract<T> {

        fn claim_contract(origin: OriginFor<T>, contract: SmartContract<T>) -> Result<bool, DispatchError>{
            let operator = ensure_signed(origin)?;
            if !<ContractHasOperator<T>>::contains_key(&contract) &&
                !<OperatorHasContract<T>>::contains_key(&operator) {
                    // add owner of the contracts
                    <OperatorHasContract<T>>::insert(&operator, contract.clone());
                    // assigns the contract to owner for staking purposes
                    <ContractHasOperator<T>>::insert(&contract, operator.clone());
                    
                    Self::deposit_event(Event::ContractClaimed(operator, contract));
                }
            Ok(true)
        }
    }
}

// 
// The main implementation block for the module.
impl<T: Config> Pallet<T> {
}


