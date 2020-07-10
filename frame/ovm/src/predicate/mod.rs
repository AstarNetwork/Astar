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
pub struct OvmExecutable {
    /// "is_valid_challenge", "decide_true", etc...
    entrypoint_name: &'static str,
    prefab_module: PrefabOvmModule,
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
    type Executable = OvmExecutable;

    fn load_main(&self, code_hash: &PredicateHash<T>) -> Result<OvmExecutable, &'static str> {
        let prefab_module = code_cache::load::<T>(code_hash, self.schedule)?;
        Ok(OvmExecutable {
            entrypoint_name: "call",
            prefab_module,
        })
    }
}

pub struct ExecutionContext<'a, T: Trait + 'a, Err, V, L, Ext> {
    pub caller: Option<&'a ExecutionContext<'a, T, Err, V, L, Ext>>,
    pub self_account: T::AccountId,
    pub depth: usize,
    // pub deferred: Vec<DeferredAction<T>>,
    pub config: &'a Config,
    pub vm: &'a V,
    pub loader: &'a L,
}

impl<'a, T, Err, E, V, L, Ex> ExecutionContext<'a, T, Err, V, L, Ex>
where
    T: Trait,
    L: Loader<T, Executable = E>,
    V: Vm<T, Err, Executable = E>,
    Err: From<&'static str>,
    Ex: Ext<T, Err>,
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

    fn nested<'b, 'c: 'b>(&'c self, dest: T::AccountId) -> ExecutionContext<'b, T, Err, V, L, Ex> {
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

    fn new_call_context<'b>(&'b self, caller: T::AccountId) -> Ex {
        Ex::new(self, caller)
    }
}

/// Implementation of `Vm` that takes `PredicateOvm` and executes it.
pub struct PredicateOvm<'a> {
    schedule: &'a Schedule,
}

impl<'a> PredicateOvm<'a> {
    pub fn new(schedule: &'a Schedule) -> Self {
        PredicateOvm { schedule }
    }
}

impl<'a, T: Trait, E: Ext<T = T>> Vm<T, E> for PredicateOvm<'a> {
    type Executable = CompiledExecutable<'a, ext::ExternalCallImpl<T, E>>;

    fn execute(&self, exec: &Self::Executable, ext: E, input_data: Vec<u8>) -> ExecResult {
        let ext_impl = ext::ExternalCallImpl::<T, E>::new(ext);
        CompiledExecutable::execute(&exec, input_data)
    }
}
