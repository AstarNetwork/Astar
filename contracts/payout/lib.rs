#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract(version = "0.1.0")]
mod payout {
    use ink_core::storage;

    use wbalances::WBalances;

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
    struct Payout{}

    impl Payout {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        fn new(&mut self) {}

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        fn default(&mut self) {
            self.new()
        }

        /// finalizeExit
        /// @dev finalize exit and withdraw asset with ownership state.
        #[ink(message)]
        fn finalize_exit(&mut self, erc20_address: AccountId, state_update: StateUpdate) {
            let owner: AccountId =
                scale::Decode::decode(&mut &state_update.state_object.inputs[0]..);
            let amount = state_update.range.end - state_update.range.start;
            assert!(self.env().caller() == owner);
            self.env().transfer(owner, amount);
            let mut wbalances = WBalances::from_account_id(erc20_address.clone());
                .expect("failed at instantiating the `WBalances` contract");
            wbalances.transer(owner, amount);
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
            let _ = Payout::default();
        }

        /// We test a simple use case of our contract.
        #[test]
        fn it_works() {
            // assert_eq!(payout.get(), false);
            // payout.flip();
            // assert_eq!(payout.get(), true);
        }
    }
}
