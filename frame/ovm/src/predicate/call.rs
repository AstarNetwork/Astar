//! This CallContext is the mock ExternalCall implementation.
//! **Note that it is not used in the production environment.**
//!
//! When Plasma module is used, it is implemented by injecting CallContext of Plasma module as Ext of ExecutionContext.

use crate::predicate::*;
use crate::traits::{Ext, Loader, Vm};
use crate::{AccountIdOf, Decision, PropertyOf, Trait};

pub struct CallContext<
    'a,
    'b: 'a,
    T: Trait + 'b,
    Err: From<&'static str>,
    V: Vm<T, Err> + 'b,
    L: Loader<T>,
> {
    ctx: &'a ExecutionContext<'b, T, Err, V, L>,
    caller: T::AccountId,
}

/// An interface that provides access to the external environment in which the
/// predicate-contract is executed similar to a smart-contract.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
impl<'a, 'b: 'a, E, T, Err, V, L> Ext<T, Err> for CallContext<'a, 'b, T, Err, V, L>
where
    T: Trait + 'b,
    Err: From<&'static str>,
    V: Vm<T, Err, Executable = E>,
    L: Loader<T, Executable = E>,
{
    fn new<'c, 'd: 'c, F, U, Frr, W, N>(
        ctx: &'c ExecutionContext<'d, U, Frr, W, N>,
        caller: AccountIdOf<T>,
    ) -> Self
    where
        U: Trait + 'd,
        Frr: From<&'static str>,
        W: Vm<T, Err, Executable = F>,
        N: Loader<T, Executable = F>,
    {
        CallContext { ctx, caller }
    }

    fn call(&self, to: &AccountIdOf<T>, input_data: Vec<u8>) -> ExecResult<Err> {
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

    fn set_predicate_decision(&self, _game_id: T::Hash, _decision: bool) -> Result<bool, Err> {
        Ok(true)
    }
}
