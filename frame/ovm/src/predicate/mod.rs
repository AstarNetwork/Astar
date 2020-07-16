use super::*;
use crate::traits::*;
use ovmi::executor::{CompiledExecutor, OvmExecutor};
use sp_std::marker::PhantomData;
mod call;
mod code_cache;
mod ext;
mod prepare;

pub use self::code_cache::save as save_code;
pub use call::CallContext;
use ovmi::predicates::CompiledExecutable;

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
    payout: T::AccountId,
    address_inputs: BTreeMap<T::Hash, T::AccountId>,
    bytes_inputs: BTreeMap<T::Hash, Vec<u8>>,
}

/// Loader which fetches `OvmExecutable` from the code cache.
pub struct PredicateLoader {
    schedule: Rc<Schedule>,
}

impl PredicateLoader {
    pub fn new(schedule: Rc<Schedule>) -> Self {
        PredicateLoader { schedule }
    }
}

impl<T: Trait> Loader<T> for PredicateLoader {
    type Executable = OvmExecutable<T>;

    fn load_main(
        &self,
        predicate: PredicateContractOf<T>,
    ) -> Result<OvmExecutable<T>, &'static str> {
        let prefab_module = code_cache::load::<T>(&predicate.predicate_hash, &self.schedule)?;
        let code = Decode::decode(&mut &prefab_module.code[..])
            .map_err(|_| "Predicate code cannot decode error.")?;
        let (payout, address_inputs, bytes_inputs) = Decode::decode(&mut &predicate.inputs[..])
            .map_err(|_| "Constructor inputs cannot decode error.")?;
        Ok(OvmExecutable {
            code,
            payout,
            address_inputs,
            bytes_inputs,
        })
    }
}

#[derive(Clone)]
pub struct ExecutionContext<T: Trait> {
    pub self_account: T::AccountId,
    pub depth: usize,
    // pub deferred: Vec<DeferredAction<T>>,
    pub config: Rc<Config>,
    pub vm: Rc<PredicateOvm<T>>,
    pub loader: Rc<PredicateLoader>,
}

impl<T> ExecutionContext<T>
where
    T: Trait,
{
    /// Create the top level execution context.
    ///
    /// The specified `origin` address will be used as `sender` for. The `origin` must be a regular
    /// account (not a contract).
    pub fn top_level(
        origin: T::AccountId,
        cfg: Rc<Config>,
        vm: Rc<PredicateOvm<T>>,
        loader: Rc<PredicateLoader>,
    ) -> Self {
        ExecutionContext {
            self_account: origin,
            depth: 0,
            // deferred: Vec::new(),
            config: cfg,
            vm: vm,
            loader: loader,
        }
    }

    fn nested(&self, dest: T::AccountId) -> ExecutionContext<T> {
        ExecutionContext {
            self_account: dest,
            depth: self.depth + 1,
            // deferred: Vec::new(),
            config: Rc::clone(&self.config),
            vm: Rc::clone(&self.vm),
            loader: Rc::clone(&self.loader),
        }
    }

    /// Make a call to the specified address, optionally transferring some funds.
    pub fn call(&self, dest: T::AccountId, input_data: Vec<u8>) -> ExecResult<T> {
        if self.depth == self.config.max_depth as usize {
            return Err("reached maximum depth, cannot make a call".into());
        }

        // Assumption: `collect_rent` doesn't collide with overlay because
        // `collect_rent` will be done on first call and destination contract and balance
        // cannot be changed before the first call
        let predicate = match <Predicates<T>>::get(&dest) {
            Some(predicate) => predicate,
            None => return Err("predicate not found".into()),
        };

        let caller = self.self_account.clone();
        let nested = self.nested(dest);
        let executable = nested
            .loader
            .load_main(predicate)
            .map_err(<ExecError<T::AccountId>>::from)?;
        nested
            .vm
            .execute(executable, nested.new_call_context(caller), input_data)
    }

    fn new_call_context(&self, caller: T::AccountId) -> T::ExternalCall {
        T::ExternalCall::new(Rc::new(self.clone()), caller)
    }
}

/// Implementation of `Vm` that takes `PredicateOvm` and executes it.
pub struct PredicateOvm<T: Trait> {
    _schedule: Rc<Schedule>,
    _phantom: PhantomData<T>,
}

impl<T: Trait> PredicateOvm<T> {
    pub fn new(_schedule: Rc<Schedule>) -> Self {
        PredicateOvm {
            _schedule,
            _phantom: PhantomData,
        }
    }
}

impl<T: Trait> Vm<T> for PredicateOvm<T> {
    type Executable = OvmExecutable<T>;

    fn execute(
        &self,
        exec: Self::Executable,
        ext: T::ExternalCall,
        input_data: Vec<u8>,
    ) -> ExecResult<T> {
        let ext_impl = ext::ExternalCallImpl::<T>::new(&ext);
        let executable = ovmi::prepare::executable_from_compiled(
            &ext_impl,
            exec.code,
            exec.payout,
            exec.address_inputs.clone(),
            exec.bytes_inputs.clone(),
        );
        let call_input_data =
            ovmi::predicates::PredicateCallInputs::<T::AccountId>::decode(&mut &input_data[..])
                .map_err(|_| <ExecError<T::AccountId>>::from("Call inputs cannot decode error."))?;
        CompiledExecutor::<CompiledExecutable<ext::ExternalCallImpl<T>>, ext::ExternalCallImpl<T>>::execute(
            executable,
            call_input_data,
        )
    }
}
