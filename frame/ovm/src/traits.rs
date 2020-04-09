use super::*;

/// A function that generates an `AccountId` for a predicate upon instantiation.
pub trait PredicateAddressFor<PredicateHash, AccountId> {
    fn predicate_address_for(code_hash: &PredicateHash, data: &[u8], origin: &AccountId) -> AccountId;
}

pub trait AtomicPredicate {
    fn decide_true(inputs: Vec<u8>);
    fn decide(inputs: Vec<u8>) -> Decision;
}

pub trait DecidablePredicate {
    fn decide_with_witness(
        inputs: Vec<u8>,
        witness: Vec<u8>,
    ) -> Decision;
}

pub trait LogicalConnective<AccountId> {
    fn is_valid_challenge(
        inputs: Vec<u8>,
        challenge_inputs: Vec<u8>,
        challenge: Property<AccountId>,
    ) -> Decision;
}
