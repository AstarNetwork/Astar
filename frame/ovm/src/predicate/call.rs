//! This CallContext is the mock ExternalCall implementation.
//! **Note that it is not used in the production environment.**
//!
//! When Plasma module is used, it is implemented by injecting CallContext of Plasma module as Ext of ExecutionContext.

use crate::predicate::*;
use crate::traits::*;
use crate::*;

pub struct CallContext<'a, 'b: 'a, T: Trait + 'b, V: Vm<T, Err> + 'b, L: Loader<T>> {
    ctx: &'a ExecutionContext<'b, T, V, L>,
    caller: T::AccountId,
}

/// An interface that provides access to the external environment in which the
/// predicate-contract is executed similar to a smart-contract.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
impl<'a, 'b: 'a, E, T, V, L> Ext<T> for CallContext<'a, 'b, T, V, L>
where
    T: Trait + 'b,
    V: Vm<T, Err, Executable = E>,
    L: Loader<T, Executable = E>,
{
    fn new<'b, Ctx>(ctx: &'b Ctx, caller: AccountIdOf<T>) -> Self {
        CallContext { ctx, caller }
    }

    fn call(&self, to: &AccountIdOf<Self::T>, input_data: Vec<u8>) -> ExecResultOf<T> {
        self.ctx.call(to.clone(), input_data)
    }

    fn caller(&self) -> &AccountIdOf<Self::T> {
        &self.caller
    }

    fn address(&self) -> &AccountIdOf<Self::T> {
        &self.ctx.self_account
    }

    fn is_stored(&self, _address: &AccountIdOf<T>, _key: &[u8], _value: &[u8]) -> bool {
        true
    }

    fn verify_inclusion_with_root(
        &self,
        _leaf: Self::Hash,
        _token_address: Self::Address,
        _range: &[u8],
        _inclusion_proof: &[u8],
        _root: &[u8],
    ) -> bool {
        true
    }

    fn is_decided(&self, property: &PropertyOf<Self>) -> bool {
        Decision::True == Module::<T>::is_decided(property)
    }
    fn is_decided_by_id(&self, id: Self::Hash) -> bool {
        Decision::True == Module::<T>::is_decided_by_id(id)
    }

    fn ext_set_predicate_decision(
        &self,
        _game_id: Self::Hash,
        _decision: bool,
    ) -> Result<bool, Err> {
        Ok(true)
    }
}
