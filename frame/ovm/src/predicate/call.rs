//! This CallContext is the mock ExternalCall implementation.
//! **Note that it is not used in the production environment.**
//!
//! When Plasma module is used, it is implemented by injecting CallContext of Plasma module as Ext of ExecutionContext.

use crate::predicate::*;
use crate::traits::{Ext, NewCallContext};
use crate::{AccountIdOf, Decision, PropertyOf, Trait};

pub struct CallContext<T: Trait> {
    ctx: Rc<ExecutionContext<T>>,
    caller: T::AccountId,
}

impl<T: Trait> NewCallContext<T> for CallContext<T> {
    fn new(ctx: Rc<ExecutionContext<T>>, caller: AccountIdOf<T>) -> Self {
        CallContext { ctx: ctx, caller }
    }
}

/// An interface that provides access to the external environment in which the
/// predicate-contract is executed similar to a smart-contract.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
impl<T> Ext<T> for CallContext<T>
where
    T: Trait,
{
    fn call(&self, to: &AccountIdOf<T>, input_data: Vec<u8>) -> ExecResult<T> {
        self.ctx.call(to.clone(), input_data)
    }

    fn caller(&self) -> &AccountIdOf<T> {
        &self.caller
    }

    fn address(&self) -> &AccountIdOf<T> {
        &self.ctx.self_account
    }

    fn is_stored(&self, _address: &AccountIdOf<T>, _key: &[u8], _value: &[u8]) -> bool {
        true
    }

    fn verify_inclusion_with_root(
        &self,
        _leaf: T::Hash,
        _token_address: AccountIdOf<T>,
        _range: &[u8],
        _inclusion_proof: &[u8],
        _root: &[u8],
    ) -> bool {
        true
    }

    fn is_decided(&self, property: &PropertyOf<T>) -> bool {
        Decision::True == <Module<T>>::is_decided(property)
    }

    fn is_decided_by_id(&self, id: T::Hash) -> bool {
        Decision::True == <Module<T>>::is_decided_by_id(id)
    }

    fn set_predicate_decision(
        &self,
        _game_id: T::Hash,
        _decision: bool,
    ) -> Result<bool, ExecError<T::AccountId>> {
        Ok(true)
    }
}
