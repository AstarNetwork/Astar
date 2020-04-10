use super::*;
use crate::traits::*;

/// Reason why a predicate call failed
#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize))]
pub enum PredicateError {
    /// Some error occurred.
    Other(#[codec(skip)] &'static str),
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
			Err(reason) => return Err(
				$crate::exec::ExecError { reason: reason.into(), buffer: $buffer }
			),
		}
	}
}

pub type ExecResult = Result<bool, ExecError>;

/// A prepared wasm module ready for execution.
#[derive(Clone, Encode, Decode)]
pub struct PrefabOVMModule {
    /// Version of the schedule with which the code was instrumented.
    #[codec(compact)]
    schedule_version: u32,
    /// Code instrumented with the latest schedule.
    code: Vec<u8>,
}

/// OVM executable loaded by `OVMLoader` and executed by `OptimisticVm`.
pub struct OVMExecutable {
    entrypoint_name: &'static str,
    prefab_module: PrefabOVMModule,
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
            return Err(PredicateError {
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
        let nested = self.nested(&dest);
        let executable = try_or_exec_error!(nested.loader.load_main(&predicate.predicate_hash), input_data);
        self.nested(&dest).vm.execute(
            &executable,
            nested.new_call_context(caller),
            input_data,
        )
    }

    // TODO kokokara
    pub fn instantiate(
        &mut self,
        code_hash: &CodeHash<T>,
        input_data: Vec<u8>,
    ) -> Result<T::AccountId, ExecError> {
        if self.depth == self.config.max_depth as usize {
            return Err(ExecError {
                reason: "reached maximum depth, cannot instantiate".into(),
                buffer: input_data,
            });
        }

        if gas_meter
            .charge(self.config, ExecFeeToken::Instantiate)
            .is_out_of_gas()
        {
            return Err(ExecError {
                reason: "not enough gas to pay base instantiate fee".into(),
                buffer: input_data,
            });
        }

        let caller = self.self_account.clone();
        let dest = T::DetermineContractAddress::contract_address_for(
            code_hash,
            &input_data,
            &caller,
        );

        // TrieId has not been generated yet and storage is empty since contract is new.
        let dest_trie_id = None;

        let output = self.with_nested_context(dest.clone(), dest_trie_id, |nested| {
            try_or_exec_error!(
				nested.overlay.instantiate_contract(&dest, code_hash.clone()),
				input_data
			);

            // Send funds unconditionally here. If the `endowment` is below existential_deposit
            // then error will be returned here.
            try_or_exec_error!(
				transfer(
					gas_meter,
					TransferCause::Instantiate,
					&caller,
					&dest,
					endowment,
					nested,
				),
				input_data
			);

            let executable = try_or_exec_error!(
				nested.loader.load_init(&code_hash),
				input_data
			);
            let output = nested.vm
                .execute(
                    &executable,
                    nested.new_call_context(caller.clone(), endowment),
                    input_data,
                    gas_meter,
                )?;

            // Error out if insufficient remaining balance.
            if nested.overlay.get_balance(&dest) < nested.config.existential_deposit {
                return Err(ExecError {
                    reason: "insufficient remaining balance".into(),
                    buffer: output.data,
                });
            }

            // Deposit an instantiation event.
            nested.deferred.push(DeferredAction::DepositEvent {
                event: RawEvent::Instantiated(caller.clone(), dest.clone()),
                topics: Vec::new(),
            });

            Ok(output)
        })?;

        Ok((dest, output))
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
impl<'a, 'b: 'a, T, V, L> Ext for CallContext<'a, 'b, T, V, L>
where
    T: Trait + 'b,
    V: Vm<T, Executable = E>,
    L: Loader<T, Executable = E>,
{
    type T = T;

    /// Instantiate a predicate from the given code.
    ///
    /// The newly created account will be associated with `code`.
    fn instantiate(
        &mut self,
        code: &PredicateHash<Self::T>,
        input_data: Vec<u8>,
    ) -> Result<AccountIdOf<Self::T>, ExecError> {
        self.ctx.instantiate(code_hash, input_data)
    }

    /// Call (possibly other predicate) into the specified account.
    fn call(&mut self, to: &AccountIdOf<Self::T>, input_data: Vec<u8>) -> bool {
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
    fn deposit_event(&mut self, topics: Vec<TopicOf<Self::T>>, data: Vec<u8>) {
        // self.ctx.deferred.push(DeferredAction::DepositEvent {
        //     topics,
        //     event: RawEvent::ContractExecution(self.ctx.self_account.clone(), data),
        // });
    }

    /// Returns the current block number.
    fn block_number(&self) -> BlockNumberOf<Self::T> {
        self.block_number
    }
}

/// A trait that represent an optimistic virtual machine.
///
/// You can view an optimistic virtual machine as something that takes code, an input data buffer,
/// queries it and/or performs actions on the given `Ext` and optionally
/// returns an output data buffer. The type of code depends on the particular virtual machine.
///
/// Execution of code can end by either implicit termination (that is, reached the end of
/// executable), explicit termination via returning a buffer or termination due to a trap.
pub trait Vm<T: Trait> {
    type Executable;

    fn execute<E: Ext<T = T>>(
        &self,
        exec: &Self::Executable,
        ext: E,
        input_data: Vec<u8>,
    ) -> ExecResult;
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
