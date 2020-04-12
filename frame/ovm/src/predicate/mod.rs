use super::*;
use crate::traits::*;

mod code_cache;
mod prepare;

pub use self::code_cache::save as save_code;

/// Reason why a predicate call failed
#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize))]
pub enum PredicateError {
    /// Some error occurred.
    Other(#[codec(skip)] &'static str),
}

impl From<&'static str> for PredicateError {
    fn from(err: &'static str) -> PredicateError {
        PredicateError::Other(err)
    }
}

/// An error indicating some failure to execute a contract call or instantiation. This can include
/// VM-specific errors during execution (eg. division by 0, OOB access, failure to satisfy some
/// precondition of a system call, etc.) or errors with the orchestration (eg. out-of-gas errors, a
/// non-existent destination contract, etc.).
#[cfg_attr(test, derive(sp_runtime::RuntimeDebug))]
pub struct ExecError {
    pub reason: PredicateError,
    /// This is an allocated buffer that may be reused. The buffer must be cleared explicitly
    /// before reuse.
    pub buffer: Vec<u8>,
}

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

pub type ExecResult = Result<bool, ExecError>;

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

pub struct ExecutionContext<'a, T: Trait + 'a, V, L> {
    pub caller: Option<&'a ExecutionContext<'a, T, V, L>>,
    pub self_account: T::AccountId,
    pub depth: usize,
    // pub deferred: Vec<DeferredAction<T>>,
    pub config: &'a Config<T>,
    pub vm: &'a V,
    pub loader: &'a L,
    pub timestamp: MomentOf<T>,
    pub block_number: T::BlockNumber,
}

impl<'a, T, E, V, L> ExecutionContext<'a, T, V, L>
where
    T: Trait,
    L: Loader<T, Executable = E>,
    V: Vm<T, Executable = E>,
{
    /// Create the top level execution context.
    ///
    /// The specified `origin` address will be used as `sender` for. The `origin` must be a regular
    /// account (not a contract).
    pub fn top_level(origin: T::AccountId, cfg: &'a Config<T>, vm: &'a V, loader: &'a L) -> Self {
        ExecutionContext {
            caller: None,
            self_account: origin,
            depth: 0,
            // deferred: Vec::new(),
            config: &cfg,
            vm: &vm,
            loader: &loader,
            timestamp: T::Time::now(),
            block_number: <frame_system::Module<T>>::block_number(),
        }
    }

    fn nested<'b, 'c: 'b>(&'c self, dest: T::AccountId) -> ExecutionContext<'b, T, V, L> {
        ExecutionContext {
            caller: Some(self),
            self_account: dest,
            depth: self.depth + 1,
            // deferred: Vec::new(),
            config: self.config,
            vm: self.vm,
            loader: self.loader,
            timestamp: self.timestamp.clone(),
            block_number: self.block_number.clone(),
        }
    }

    /// Make a call to the specified address, optionally transferring some funds.
    pub fn call(&mut self, dest: T::AccountId, input_data: Vec<u8>) -> ExecResult {
        if self.depth == self.config.max_depth as usize {
            return Err(ExecError {
                reason: "reached maximum depth, cannot make a call".into(),
                buffer: input_data,
            });
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
        let mut nested = self.nested(dest);
        let executable = try_or_exec_error!(
            nested.loader.load_main(&predicate.predicate_hash),
            input_data
        );
        nested
            .vm
            .execute(&executable, nested.new_call_context(caller), input_data)
    }

    fn new_call_context<'b>(&'b mut self, caller: T::AccountId) -> CallContext<'b, 'a, T, V, L> {
        let timestamp = self.timestamp.clone();
        let block_number = self.block_number.clone();
        CallContext {
            ctx: self,
            caller,
            timestamp,
            block_number,
        }
    }
}

pub struct CallContext<'a, 'b: 'a, T: Trait + 'b, V: Vm<T> + 'b, L: Loader<T>> {
    ctx: &'a mut ExecutionContext<'b, T, V, L>,
    caller: T::AccountId,
    timestamp: MomentOf<T>,
    block_number: T::BlockNumber,
}

/// An interface that provides access to the external environment in which the
/// predicate-contract is executed similar to a smart-contract.
///
/// This interface is specialized to an account of the executing code, so all
/// operations are implicitly performed on that account.
impl<'a, 'b: 'a, E, T, V, L> Ext for CallContext<'a, 'b, T, V, L>
where
    T: Trait + 'b,
    V: Vm<T, Executable = E>,
    L: Loader<T, Executable = E>,
{
    type T = T;

    /// Call (possibly other predicate) into the specified account.
    fn call(&mut self, to: &AccountIdOf<Self::T>, input_data: Vec<u8>) -> ExecResult {
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

    /// Returns a reference to the timestamp of the current block
    fn now(&self) -> &MomentOf<Self::T> {
        &self.timestamp
    }

    /// Returns a random number for the current block with the given subject.
    fn random(&self, subject: &[u8]) -> SeedOf<Self::T> {
        T::Randomness::random(subject)
    }

    /// Deposit an event with the given topics.
    ///
    /// There should not be any duplicates in `topics`.
    // fn deposit_event(&mut self, topics: Vec<TopicOf<Self::T>>, data: Vec<u8>) {
    //     self.ctx.deferred.push(DeferredAction::DepositEvent {
    //         topics,
    //         event: RawEvent::ContractExecution(self.ctx.self_account.clone(), data),
    //     });
    // }

    /// Returns the current block number.
    fn block_number(&self) -> BlockNumberOf<Self::T> {
        self.block_number
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

impl<'a, T: Trait> Vm<T> for PredicateOvm<'a> {
    type Executable = OvmExecutable;

    fn execute<E: Ext<T = T>>(
        &self,
        exec: &Self::Executable,
        ext: E,
        input_data: Vec<u8>,
    ) -> ExecResult {
        // TODO: make sandbox environments(Is it needed?)
        // let mut imports = sp_sandbox::EnvironmentDefinitionBuilder::new();

        // TODO: runtime setup.
        // runtime::Env::impls(&mut |name, func_ptr| {
        //     imports.add_host_func("env", name, func_ptr);
        // });
        //
        // let mut runtime = Runtime::new(
        //     &mut ext,
        //     input_data,
        //     &self.schedule,
        // );

        // TODO: instantiate vm and execute and get results.
        // Instantiate the instance from the instrumented module code and invoke the contract
        // entrypoint.
        // let result = sp_sandbox::Instance::new(&exec.prefab_module.code, &imports, &mut runtime)
        //     .and_then(|mut instance| instance.invoke(exec.entrypoint_name, &[], &mut runtime));
        // to_execution_result(runtime, result)

        Ok(true)
    }
}

// address()

// // predicate must be derive to:
// #[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
// pub struct BaseAtomicPredicate;
//
// impl BaseAtomicPredicate {
//     pub fn new() -> BaseAtomicPredicate {
//         Self
//     }
// }
//
// impl AtomicPredicate for BaseAtomicPredicate {
//     fn decide_true(inputs: Vec<u8>) -> Result {
//         if Self::decide(_inputs) != Decision::True {
//             // error: "must decide true"
//         }
//         property = Property {
//             predicate_address: Self::address(),
//             inputs: inputs,
//         };
//
//         T::set_predicate_decision(utils.getPropertyId(property), true)
//     }
//     fn decide(_inputs: Vec<u8>) -> Decision {
//         Decision::False
//     }
// }
//
// impl DecidablePredicate for BaseAtomicPredicate {
//     fn decide_with_witness(inputs: Vec<u8>, _witness: Vec<u8>) -> Decision {
//         Self::decide(inputs)
//     }
// }
