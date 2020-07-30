#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct StateUpdate<AccountId, Balance, BlockNumber> {
    deposit_contract_address: AccountId,
    range: Range<Balance>,
    block_number: BlockNumber,
    state_object: Property<AccountId>,
}

#[ink::contract(version = "0.1.0")]
mod payout {
    use ink_core::storage;

    #[derive(scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Range {
        start: Balance,
        end: Balance,
    }

    #[derive(scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Property {
        /// Indicates the address of Predicate.
        pub predicate_address: AccountId,
        /// Every input are bytes. Each Atomic Predicate decode inputs to the specific type.
        pub inputs: Vec<Vec<u8>>,
    }

    #[derive(scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct StateUpdate {
        deposit_contract_address: AccountId,
        range: Range,
        block_number: BlockNumber,
        state_object: Property,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    struct Payout {
        /// Stores a single `bool` value on the storage.
        value: storage::Value<bool>,
    }

    impl Payout {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        fn new(&mut self, init_value: bool) {
            self.value.set(init_value);
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        fn default(&mut self) {
            self.new(false)
        }

        /// finalizeExit
        /// @dev finalize exit and withdraw asset with ownership state.
        #[ink(message)]
        fn finalize_exit(&mut self,
                         state_update: StateUpdate
        ) {
            // finalize_exit payout -> owner[state_update.state_objects.inputs[0]] (amount[state_update.range]) at payout.
            *self.value = !self.get();
            // uint256 amount = stateUpdate.range.end - stateUpdate.range.start;
            // require(msg.sender == owner, "msg.sender must be owner");
            // depositContract.erc20().transfer(_owner, amount);
        }

    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[test]
        fn default_works() {
            // Note that even though we defined our `#[ink(constructor)]`
            // above as `&mut self` functions that return nothing we can call
            // them in test code as if they were normal Rust constructors
            // that take no `self` argument but return `Self`.
            let payout = Payout::default();
            assert_eq!(payout.get(), false);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn it_works() {
            // let mut payout = Payout::new(false);
            // assert_eq!(payout.get(), false);
            // payout.flip();
            // assert_eq!(payout.get(), true);
        }
    }
}
