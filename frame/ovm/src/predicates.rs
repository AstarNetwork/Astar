use super::traits::*;
use super::*;

// predicate must be derive to:
// address()

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct BaseAtomicPredicate;

impl BaseAtomicPredicate {
    pub fn new() -> BaseAtomicPredicate {
        Self
    }
}

impl AtomicPredicate for BaseAtomicPredicate {
    fn decide_true(inputs: Vec<u8>) -> Result {
        if Self::decide(_inputs) != Decision::True {
            // error: "must decide true"
        }
        property = Property {
            predicate_address: Self::address(),
            inputs: inputs,
        };

        T::set_predicate_decision(utils.getPropertyId(property), true)
    }
    fn decide(_inputs: Vec<u8>) -> Decision {
        Decision::False
    }
}

impl DecidablePredicate for BaseAtomicPredicate {
    fn decide_with_witness(inputs: Vec<u8>, _witness: Vec<u8>) -> Decision {
        Self::decide(inputs)
    }
}
