#![cfg_attr(not(feature = "std"), no_std)]

use contracts::{BalanceOf, CodeHash, ContractAddressFor, Gas};
use sp_runtime::{
    traits::{MaybeDisplay, MaybeSerialize, Member},
    DispatchError,
};
use sp_std::prelude::*;
use support::{decl_event, decl_module, decl_storage, Parameter};
use system::{ensure_signed, RawOrigin};

pub mod parameters;
#[cfg(test)]
mod tests;

use crate::parameters::Verifiable;

pub trait ContractFinder<AccountId, Parameter> {
    fn is_exists_contract(contract_id: &AccountId) -> bool;
    fn operator(contract_id: &AccountId) -> Option<AccountId>;
    fn parameters(contract_id: &AccountId) -> Option<Parameter>;
}

pub trait OperatorFinder<AccountId: Parameter> {
    fn contracts(operator_id: &AccountId) -> Vec<AccountId>;
}

pub trait TransferOperator<AccountId: Parameter>: OperatorFinder<AccountId> {
    /// Changes an operator for identified contracts with verify.
    fn transfer_operator(
        current_operator: AccountId,
        contracts: Vec<AccountId>,
        new_operator: AccountId,
    ) -> Result<(), DispatchError> {
        Self::verify_transfer_operator(&current_operator, &contracts)?;
        Self::force_transfer_operator(current_operator, contracts, new_operator);
        Ok(())
    }

    fn verify_transfer_operator(
        current_operator: &AccountId,
        contracts: &Vec<AccountId>,
    ) -> Result<(), DispatchError> {
        let operate_contracts = Self::contracts(current_operator);

        // check the actually operate the contract.
        if !contracts.iter().all(|c| operate_contracts.contains(c)) {
            Err(DispatchError::Other(
                "The sender don't operate the contracts address.",
            ))?
        }
        Ok(())
    }

    /// Force Changes an operator for identified contracts without verify.
    fn force_transfer_operator(
        current_operator: AccountId,
        contracts: Vec<AccountId>,
        new_operator: AccountId,
    );
}

/// The module's configuration trait.
pub trait Trait: contracts::Trait {
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
        pub OperatorHasContracts get(fn operator_has_contracts): map hasher(blake2_128_concat)
                                                                 T::AccountId => Vec<T::AccountId>;
        /// A mapping from operated contract by operator to it.
        pub ContractHasOperator get(fn contract_has_operator): map hasher(blake2_128_concat)
                                                               T::AccountId => Option<T::AccountId>;
        /// A mapping from contract to it's parameters.
        pub ContractParameters get(fn contract_parameters): map hasher(blake2_128_concat)
                                                            T::AccountId => Option<T::Parameters>;
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
            parameters: T::Parameters) {
            let operator = ensure_signed(origin)?;

            // verify parameters.
            parameters.verify()?;

            let contract = T::DetermineContractAddress::contract_address_for(&code_hash, &data, &operator);
            contracts::Module::<T>::instantiate(RawOrigin::Signed(operator.clone()).into(), endowment, gas_limit, code_hash, data)?;

            // add operator to contracts
            <OperatorHasContracts<T>>::mutate(&operator, {|tree| (*tree).push(contract.clone()) });
            // add contract to operator
            <ContractHasOperator<T>>::insert(&contract, operator.clone());
            // add contract to parameters
            <ContractParameters<T>>::insert(&contract, parameters);

            // issue an event operator -> contract
            Self::deposit_event(RawEvent::SetOperator(operator, contract));
        }

        /// Updates parameters for an identified contact.
        pub fn update_parameters(origin, contract: T::AccountId, parameters: T::Parameters) {
            let operator = ensure_signed(origin)?;

            // verify parameters
            parameters.verify()?;

            let contracts = <OperatorHasContracts<T>>::get(&operator);

            // check the actually operate the contract.
            if !contracts.contains(&contract) {
                Err("The sender don't operate the contract address.")?
            }

            // update parameters
            <ContractParameters<T>>::insert(&contract, parameters.clone());
            // issue set parameter events
            Self::deposit_event(RawEvent::SetParameters(contract, parameters));
        }

        /// Changes an operator for identified contracts.
        pub fn change_operator(origin, contracts: Vec<T::AccountId>, new_operator: T::AccountId) {
            let operator = ensure_signed(origin)?;
            Self::transfer_operator(operator, contracts, new_operator)?;
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

impl<T: Trait> ContractFinder<T::AccountId, T::Parameters> for Module<T> {
    fn is_exists_contract(contract_id: &T::AccountId) -> bool {
        <ContractHasOperator<T>>::contains_key(contract_id)
    }
    fn operator(contract_id: &T::AccountId) -> Option<T::AccountId> {
        <ContractHasOperator<T>>::get(contract_id)
    }
    fn parameters(contract_id: &T::AccountId) -> Option<T::Parameters> {
        <ContractParameters<T>>::get(contract_id)
    }
}
impl<T: Trait> OperatorFinder<T::AccountId> for Module<T> {
    fn contracts(operator_id: &T::AccountId) -> Vec<T::AccountId> {
        <OperatorHasContracts<T>>::get(operator_id)
    }
}

impl<T: Trait> TransferOperator<T::AccountId> for Module<T> {
    /// Force Changes an operator for identified contracts without verify.
    fn force_transfer_operator(
        current_operator: T::AccountId,
        contracts: Vec<T::AccountId>,
        new_operator: T::AccountId,
    ) {
        // remove origin operator to contracts
        <OperatorHasContracts<T>>::mutate(&current_operator, |tree| {
            *tree = tree
                .iter()
                .filter(|&x| !contracts.contains(x))
                .cloned()
                .collect()
        });

        // add new_operator to contracts
        <OperatorHasContracts<T>>::mutate(&new_operator, |tree| {
            for c in contracts.iter() {
                (*tree).push(c.clone());
            }
        });
        for c in contracts.iter() {
            // add contract to new_operator
            <ContractHasOperator<T>>::insert(&c, new_operator.clone());
            // issue an event operator -> contract
            Self::deposit_event(RawEvent::SetOperator(new_operator.clone(), c.clone()));
        }
    }
}
