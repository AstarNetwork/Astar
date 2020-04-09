use super::*;

/// A function that generates an `AccountId` for a predicate upon instantiation.
pub trait PredicateAddressFor<PredicateHash, AccountId> {
    fn predicate_address_for(code_hash: &PredicateHash, data: &[u8], origin: &AccountId) -> AccountId;
}
