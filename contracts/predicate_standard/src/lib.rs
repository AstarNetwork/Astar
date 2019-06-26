#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{
	memory::{string::String, vec::Vec, format},
	storage,
};
use ink_lang::contract;
use primitives::*;

contract! {
    #![env = ink_core::env::DefaultSrmlTypes]
    /// This simple dummy contract has a `bool` value that can
    /// alter between `true` and `false` using the `flip` message.
    /// Users can retrieve its current state using the `get` message.
    struct PredicateStandard {
        /// The current state of our flag.
        value: storage::Value<bool>,
    }

    impl Deploy for PredicateStandard {
        /// Initializes our state to `false` upon deploying our smart contract.
        fn deploy(&mut self) {
            self.value.set(false)
        }
    }

    impl PredicateStandard {
        /// deprecation method called by depositContract.deprecateExit.
        pub(external) fn deprecation(&self, check_point: Checkpoint) {
			unimplemented!();
        }

		pub (external) fn exit_initiation(&self, exit: Checkpoint) {
			unimplemented!();
		}

		pub (external) fn exit_finalization(&self, exit: Checkpoint) {
			unimplemented!();
		}

		pub (external) fn verify_transaction(&self,
			pre_state: StateUpdate,
			transaction: Transaction,
			witness: Vec<u8>,
			post_state: StateUpdate) {
			unimplemented!();
		}

		pub (external) fn prove_exit_deprecation(&self,
			deprecated_exit: Checkpoint,
			transaction: Transaction,
			witness: Vec<u8>,
			post_state: StateUpdate) {
			unimplemented!();
		}

        /// Returns the current state.
        pub(external) fn get(&self) -> bool {
            env.println(&format!("Storage Value: {:?}", *self.value));
            *self.value
        }
    }
}

#[cfg(all(test, feature = "test-env"))]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
		let mut contract = PredicateStandard::deploy_mock();
		assert_eq!(contract.get(), false);
		contract.flip();
		assert_eq!(contract.get(), true);
	}
}
