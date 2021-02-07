#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::Parameter;
use sp_runtime::DispatchError;
use sp_std::prelude::*;

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
