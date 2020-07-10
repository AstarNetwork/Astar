use super::*;
use crate::traits::*;
use snafu::Snafu;

mod call;
mod code_cache;
mod ext;
mod prepare;

pub use self::code_cache::save as save_code;
use ovmi::predicates::CompiledExecutable;

impl From<&'static str> for PredicateError {
    fn from(err: &'static str) -> PredicateError {
        PredicateError::Other(err)
    }
}

pub type ExecResult<Err> = Result<Vec<u8>, Err>;

/// Evaluate an expression of type Result<_, &'static str> and either resolve to the value if Ok or
/// wrap the error string into an ExecutionError with the provided buffer and return from the
/// enclosing function. This macro is used instead of .map_err(..)? in order to avoid taking
/// ownership of buffer unless there is an error.
#[macro_export]
macro_rules! try_or_exec_error {
    ($e:expr, $buffer:expr) => {
        match $e {
            Ok(val) => val,
            Err(reason) => {
                return Err($crate::predicate::ExecError {
                    reason: reason.into(),
                    buffer: $buffer,
                })
            }
        }
    };
}

/// A prepared wasm module ready for execution.
#[derive(Clone, Encode, Decode)]
pub struct PrefabOvmModule {
    /// Version of the schedule with which the code was instrumented.
    #[codec(compact)]
    schedule_version: u32,
    /// Code instrumented with the latest schedule.
    code: Vec<u8>,
}

/// Ovm executable loaded by `OvmLoader` and executed by `OptimisticOvm`.
pub struct OvmExecutable<T: Trait> {
    code: ovmi::compiled_predicates::CompiledPredicate,
    payout: T::AccountIdL2,
    address_inputs: BTreeMap<T::Hash, T::AccountIdL2>,
    bytes_inputs: BTreeMap<T::Hash, Vec<u8>>,
}

/// Loader which fetches `OvmExecutable` from the code cache.
pub struct PredicateLoader<'a> {
    schedule: &'a Schedule,
}

impl<'a> PredicateLoader<'a> {
    pub fn new(schedule: &'a Schedule) -> Self {
        PredicateLoader { schedule }
    }
}

impl<'a, T: Trait> Loader<T> for PredicateLoader<'a> {
    type Executable = OvmExecutable<T>;

    fn load_main(&self, predicate: &PredicateContractOf<T>) -> Result<OvmExecutable, &'static str> {
        let prefab_module = code_cache::load::<T>(predicate.code_, self.schedule)?;
        let code = Decode::decode(&mut &prefab_module.code)
            .map_err(|| "Predicate code cannot decode error.".into())?;
        let (payout, address_inputs, bytes_inputs) = Decode::decode(&mut &predicate.inputs[..])
            .map_err(|| "Constructor inputs cannot decode error.")?;
        Ok(OvmExecutable {
            code,
            payout,
            address_inputs,
            bytes_inputs,
        })
    }
}

pub struct ExecutionContext<'a, T: Trait + 'a, Err, V, L> {
    pub caller: Option<&'a ExecutionContext<'a, T, Err, V, L>>,
    pub self_account: T::AccountId,
    pub depth: usize,
    // pub deferred: Vec<DeferredAction<T>>,
    pub config: &'a Config,
    pub vm: &'a V,
    pub loader: &'a L,
}

impl<'a, T, Err, E, V, L> ExecutionContext<'a, T, Err, V, L>
where
    T: Trait,
    L: Loader<T, Executable = E>,
    V: Vm<T, Err, Executable = E>,
    Err: From<&'static str>,
{
    /// Create the top level execution context.
    ///
    /// The specified `origin` address will be used as `sender` for. The `origin` must be a regular
    /// account (not a contract).
    pub fn top_level(origin: T::AccountId, cfg: &'a Config, vm: &'a V, loader: &'a L) -> Self {
        ExecutionContext {
            caller: None,
            self_account: origin,
            depth: 0,
            // deferred: Vec::new(),
            config: &cfg,
            vm: &vm,
            loader: &loader,
        }
    }

    fn nested<'b, 'c: 'b>(&'c self, dest: T::AccountId) -> ExecutionContext<'b, T, Err, V, L> {
        ExecutionContext {
            caller: Some(self),
            self_account: dest,
            depth: self.depth + 1,
            // deferred: Vec::new(),
            config: self.config,
            vm: self.vm,
            loader: self.loader,
        }
    }

    /// Make a call to the specified address, optionally transferring some funds.
    pub fn call(&self, dest: T::AccountId, input_data: Vec<u8>) -> ExecResult<Err> {
        if self.depth == self.config.max_depth as usize {
            return "reached maximum depth, cannot make a call".into();
        }

        // Assumption: `collect_rent` doesn't collide with overlay because
        // `collect_rent` will be done on first call and destination contract and balance
        // cannot be changed before the first call
        let predicate = match <Predicates<T>>::get(&dest) {
            Some(predicate) => predicate,
            None => {
                return Err(ExecError {
                    reason: "predicate not found".into(),
                    buffer: input_data,
                })
            }
        };

        let caller = self.self_account.clone();
        let nested = self.nested(dest);
        let executable = try_or_exec_error!(
            nested.loader.load_main(&predicate.predicate_hash),
            input_data
        );
        nested
            .vm
            .execute(&executable, nested.new_call_context(caller), input_data)
    }

    fn new_call_context<'b>(&'b self, caller: T::AccountId) -> T::ExternalCall {
        T::ExternalCall::new(self, caller)
    }
}

/// Implementation of `Vm` that takes `PredicateOvm` and executes it.
pub struct PredicateOvm<'a, T: Trait> {
    schedule: &'a Schedule,
}

impl<'a, T: Trait, Err: From<&'static str>> Vm<T> for PredicateOvm<'a, T> {
    type Executable = OvmExecutable<'a, T>;

    fn execute(
        &self,
        exec: Self::Executable,
        ext: T::ExternalCall,
        input_data: Vec<u8>,
    ) -> ExecResult<Err> {
        let ext_impl = ext::ExternalCallImpl::<T>::new(&ext);
        let executable = ovmi::prepare::executable_from_compiled(
            &ext_impl,
            exec.code,
            exec.payout,
            exec.address_inputs.clone(),
            exec.bytes_inputs.clone(),
        );
        let call_input_data =
            ovmi::predicates::PredicateCallInputs::<T::AccountIdL2>::decode(&mut &input_data[..])
                .map_err(|_| "Call inputs cannot decode error.".into())?;
        CompiledExecutor::<Self::Executable, ext::ExternalCallImpl<T>>::execute(
            &executable,
            call_input_data,
        )
    }
}
