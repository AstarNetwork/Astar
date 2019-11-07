#![cfg_attr(not(feature = "std"), no_std)]

use support::{decl_module, decl_storage, decl_event, dispatch::Result, Parameter};
use system::{ensure_signed, RawOrigin};
use sr_primitives::traits::{Member, MaybeDisplay, MaybeSerialize};
use rstd::collections::btree_set::BTreeSet;
use contract::{BalanceOf, Gas, CodeHash, ContractAddressFor};

#[cfg(test)]
mod tests;

/// The module's configuration trait.
pub trait Trait: contract::Trait {
	type Parameters: Parameter + Member + MaybeSerialize + MaybeDisplay + Default + rstd::hash::Hash;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Operator {
		/// A mapping from operators to operated contracts by them.
		pub OperatorHasContracts: map T::AccountId => BTreeSet<T::AccountId>;
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
			let contract = T::DetermineContractAddress::contract_address_for(&code_hash, &data, &operator);
			contract::Module::<T>::instantiate(RawOrigin::Signed(operator.clone()).into(), endowment, gas_limit, code_hash, data)?;

			// add operator to contracts
			<OperatorHasContracts<T>>::mutate(&operator, {|tree| (*tree).insert(contract.clone()) });
			// add contract to operator
			<ContractHasOperator<T>>::insert(&contract, operator.clone());
			// add contract to paramters
			<ContractParameters<T>>::insert(&contract, parameters);

			// issue an event operator -> contract
			Self::deposit_event(RawEvent::SetOperator(operator, contract));
			Ok(())
		}

		/// Updates parameters for an identified contact.
		pub fn update_parameters(origin, paramters: T::Parameters) -> Result {
			Ok(())
		}

		/// Changes an operator for identified contracts.
		pub fn change_operator(origin, contracts: Vec<T::AccountId>, new_operator: T::AccountId) -> Result {
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T>
		where
		AccountId = <T as system::Trait>::AccountId,
		Parameters = <T as Trait>::Parameters
	{
		/// When operator changed,
		/// it is issued that 1-st Operator AccountId and 2-nd Contract AccountId.
		SetOperator(AccountId, AccountId),

		/// When contract's parameters changed,
		/// it is issued that 1-st Contract AccountId and 2-nd the contract's new parameters.
		SetParameter(AccountId, Parameters),
	}
);
