use crate::traits::*;
use super::*;

/// Reason why a predicate call failed
#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize))]
pub enum PredicateError {
    /// Some error occurred.
    Other(#[codec(skip)] &'static str),
}

pub type PredicateResult = Result<bool, ExecError>;

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
        L: Loader<T, Executable = E>,{
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
    fn call(
        &mut self,
        to: &AccountIdOf<Self::T>,
        input_data: Vec<u8>,
    ) -> bool {
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
    ) -> PredicateResult;
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
