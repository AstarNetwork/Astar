use crate::predicate::*;
use crate::traits::*;

pub struct CallContext<
    'a,
    'b: 'a,
    T: Trait + 'b,
    V: Vm<T> + 'b,
    L: Loader<T>,
    Err: From<&'static str>,
> {
    ctx: &'a ExecutionContext<'b, T, V, L, Err>,
    caller: T::AccountId,
}

/// An interface that provides access to the external environment in which the
/// predicate-contract is executed similar to a smart-contract.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
impl<'a, 'b: 'a, E, T, V, L, Err> Ext<T, Err> for CallContext<'a, 'b, T, V, L, Err>
where
    T: Trait + 'b,
    V: Vm<T, Executable = E, Err>,
    L: Loader<T, Executable = E>,
    Err: From<&'static str>,
{
    type T = T;

    /// Call (possibly other predicate) into the specified account.
    fn call(&self, to: &AccountIdOf<Self::T>, input_data: Vec<u8>) -> Result<Vec<u8>> {
        self.ctx.call(to.clone(), input_data)
    }

    /// Returns a reference to the account id of the caller.
    fn caller(&self) -> &AccountIdOf<Self::T> {
        &self.caller
    }

    /// Returns a reference to the account id of the current contract.
    fn address(&self) -> &AccountIdOf<Self::T> {
        &self.ctx.self_account
    }
}
