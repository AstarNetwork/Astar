#![cfg_attr(not(feature = "std"), no_std)]

use contract::{BalanceOf, CodeHash, ContractAddressFor, Gas};
use sp_runtime::traits::{MaybeDisplay, MaybeSerialize, Member};
use support::{decl_event, decl_module, decl_storage, dispatch::Result, Parameter};
use system::{ensure_signed, RawOrigin};
use sp_std::prelude::*;

pub mod parameters;
#[cfg(test)]
mod tests;

use crate::parameters::Verifiable;

/// The module's configuration trait.
pub trait Trait: contract::Trait {
	type Parameters: Parameter
	+ Member
	+ MaybeSerialize
	+ MaybeDisplay
	+ Default
	+ sp_std::hash::Hash
	+ parameters::Verifiable;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Operator {
        /// A mapping from operators to operated contracts by them.
        pub OperatorHasContracts: map T::AccountId => Vec<T::AccountId>;
        /// A mapping from operated contract by operator to it.
        pub ContractHasOperator: map T::AccountId => Option<T::AccountId>;
        /// A mapping from contract to it's parameters.
        pub ContractParameters: map T::AccountId => Option<T::Parameters>;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        /// Deploys a contact and insert relation of a contract and an operator to mapping.
        pub fn instantiate(origin,
            #[compact] endowment: BalanceOf<T>,
            #[compact] gas_limit: Gas,
            code_hash: CodeHash<T>,
            data: Vec<u8>,
            parameters: T::Parameters) -> Result {
            let operator = ensure_signed(origin)?;

            // verify parameters.
            parameters.verify()?;

            let contract = T::DetermineContractAddress::contract_address_for(&code_hash, &data, &operator);
            contract::Module::<T>::instantiate(RawOrigin::Signed(operator.clone()).into(), endowment, gas_limit, code_hash, data)?;

            // add operator to contracts
            <OperatorHasContracts<T>>::mutate(&operator, {|tree| (*tree).push(contract.clone()) });
            // add contract to operator
            <ContractHasOperator<T>>::insert(&contract, operator.clone());
            // add contract to parameters
            <ContractParameters<T>>::insert(&contract, parameters);

            // issue an event operator -> contract
            Self::deposit_event(RawEvent::SetOperator(operator, contract));
            Ok(())
        }

        /// Updates parameters for an identified contact.
        pub fn update_parameters(origin, contract: T::AccountId, parameters: T::Parameters) -> Result {
            let operator = ensure_signed(origin)?;

            // verify parameters
            parameters.verify()?;

            let contracts = <OperatorHasContracts<T>>::get(&operator);

            // check the actually operate the contract.
            if !contracts.contains(&contract) {
                return Err("The sender don't operate the contract address.")
            }

            // update parameters
            <ContractParameters<T>>::insert(&contract, parameters.clone());
            // issue set parameter events
            Self::deposit_event(RawEvent::SetParameters(contract, parameters));
            Ok(())
        }

        /// Changes an operator for identified contracts.
        pub fn change_operator(origin, contracts: Vec<T::AccountId>, new_operator: T::AccountId) -> Result {
            let operator = ensure_signed(origin)?;
            let operate_contracts = <OperatorHasContracts<T>>::get(&operator);

            // check the actually operate the contract.
            if !contracts.iter().all(|c| operate_contracts.contains(&c)) {
                return Err("The sender don't operate the contracts address.")
            }

            // remove origin operator to contracts
            <OperatorHasContracts<T>>::mutate(&operator,
            	|tree| *tree = tree.iter().filter(|&x| !contracts.contains(x)).cloned().collect());

            // add new_operator to contracts
            <OperatorHasContracts<T>>::mutate(&new_operator,
                |tree| for c in contracts.iter() { (*tree).push(c.clone()); }
            );
            for c in contracts.iter() {
                // add contract to new_operator
                <ContractHasOperator<T>>::insert(&c, new_operator.clone());
                // issue an event operator -> contract
                Self::deposit_event(RawEvent::SetOperator(new_operator.clone(), c.clone()));
            }
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Parameters = <T as Trait>::Parameters,
    {
        /// When operator changed,
        /// it is issued that 1-st Operator AccountId and 2-nd Contract AccountId.
        SetOperator(AccountId, AccountId),

        /// When contract's parameters changed,
        /// it is issued that 1-st Contract AccountId and 2-nd the contract's new parameters.
        SetParameters(AccountId, Parameters),
    }
);
