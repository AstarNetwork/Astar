//! Predicate implementation of Predicate contract which conforms to PGSpec.
//! The details of implementation refer to the description of trait.rs.
//!
//! Ownership means that NFT(StateObject) has an only owner(AccountId) object. In other words, Same as PlasmaCash.

use super::*;
use commitment::traits::Verify;
use core::marker::PhantomData;
use deposit::traits::Deposit;
use ink_core::{
    memory::{format, vec::Vec},
    storage,
};
use primitives::default::*;

ink_model::state! {
	/// Ownership predicate Contract.
	#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
    pub struct Predicate {
        // deposit contract
        DEPOSIT: deposit::default::Deposit,
    }
}

/// Ownership TransactionBody.
#[derive(Debug, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct TransactionBody {
	// the state object that the owner desires to mutate to.
    new_state: StateObject<AccountId>,
	/// the minimum plasma blocknumber of the ownership StateUpdate s from which you are spending.
    origin_block: BlockNumber,
	/// the maximum plasma blocknumber of the ownership StateUpdate s from which you are spending.
    max_block: BlockNumber,
}
impl ink_core::storage::Flush for TransactionBody {
    fn flush(&mut self) {}
}

/// Signature is used the witness for transaction proof.
#[derive(Copy, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "ink-generate-abi", derive(type_metadata::Metadata))]
pub struct Signature(pub [u8; 64]);
pub fn check_signature<T: Codec>(data: &T, pubkey: &AccountId, signature: &Signature) -> bool {
    // TODO check signature, but now can not ink! signature logic.
    // TODO Waiting efficient built-in cryptographic functions. (https://github.com/paritytech/ink/issues/6)
    true
}

impl
    traits::Predicate<
        AccountId,
        TransactionBody,
        Signature,
        RangeNumber,
        commitment::default::Commitment,
        deposit::default::Deposit,
    > for Predicate
{
    /// Deplpy predicate contract.
    fn deploy(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        token_address: AccountId,
        chalenge_period: BlockNumber,
        exit_period: BlockNumber,
    ) {
        self.deposit()
            .deploy(env, token_address, chalenge_period, exit_period);
    }

    /// The main thing that must be defined for a state transition model is this verifyTransaction function
    /// which accepts a preState state update, and verifies against a transaction and witness that a given postState is correct.
    fn verify_transaction(
        &self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        pre_state: &StateUpdate<AccountId>,
        transaction: &Transaction<TransactionBody>,
        witness: &Signature,
        post_state: &StateUpdate<AccountId>,
    ) -> bool {
        // Define a custom witness struct for their particular type of state.
        let owner = &pre_state.state_object.data;
        if !check_signature(transaction, owner, witness) {
            env.println("Owner must have signed the transaction.");
            return false;
        }

        // Disallow state transitions which pass verification without some interested party’s consent, e.g. the owner’s signature
        // check the prestate came after or at the originating block
        if pre_state.plasma_block_number > transaction.body.origin_block {
            env.println(
                "Transaction preState must come before or on the transaction body origin block.",
            );
            return false;
        }
        // check the poststate came before or at the max block
        if post_state.plasma_block_number > transaction.body.max_block {
            env.println(
                "Transaction postState must come before or on the transaction body max block.",
            );
            return false;
        }
        // check the state objects are the same
        if post_state.state_object != transaction.body.new_state {
            env.println("postState must be the transaction.body.newState.");
            return false;
        }
        true
    }

    /// Allows the predicate contract to start an exit from a checkpoint. Checkpoint may be pending or finalized.
    fn start_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: Checkpoint<AccountId>,
    ) -> primitives::Result<ExitStarted> {
        // Extract the owner from the state object data field
        let owner = &checkpoint.state_update.state_object.data;

        // Require that this is called by the owner
        if &env.caller() != owner {
            return Err("Only owner may initiate the exit.");
        }

        // Forward the authenticated startExit to the deposit contract
        self.deposit().start_exit(env, checkpoint)
    }

    /// Finalizes an exit that has passed its exit period and has not been successfully challenged.
    fn finalize_exit(
        &mut self,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        exit: Checkpoint<AccountId>,
        deposited_range_id: RangeNumber,
    ) -> primitives::Result<ExitFinalized<AccountId>> {
        // Extract the owner from the state object data field
        let owner = &exit.state_update.state_object.data;
        // Require that this is called by the owner
        if &env.caller() != owner {
            return Err("Only owner may finalize the exit.");
        }
        // handle the finalization from the parent class now thaat we've verified it's authenticated
        self.deposit().finalize_exit(env, exit, deposited_range_id)
    }

    fn commitment(&mut self) -> &mut commitment::default::Commitment {
        self.deposit().commitment()
    }
    fn deposit(&mut self) -> &mut deposit::default::Deposit {
        &mut self.DEPOSIT
    }

    fn commitment_ref(&self) -> &commitment::default::Commitment {
        self.deposit_ref().commitment_ref()
    }
    fn deposit_ref(&self) -> &deposit::default::Deposit {
        &self.DEPOSIT
    }
}

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;
    use crate::traits::Predicate as _;
    use ink_core::storage::{
        alloc::{AllocateUsing, BumpAlloc, Initialize as _},
        Key,
    };
    use ink_model::EnvHandler;
    use std::convert::TryInto;

    impl Predicate {
        /// Deploys the testable contract by initializing it with the given values.
        pub fn deploy_mock(
            token_address: AccountId,
            challenge_period: BlockNumber,
            exit_period: BlockNumber,
        ) -> (
            Self,
            EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        ) {
            // initialize Environment.
            ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_address(get_contract_address());
            ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(1);
            ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_caller(get_sender_address());

            let (mut predicate, mut env) = unsafe {
                let mut alloc = BumpAlloc::from_raw_parts(Key([0x0; 32]));
                (
                    Self::allocate_using(&mut alloc),
                    AllocateUsing::allocate_using(&mut alloc),
                )
            };
            predicate.initialize(());
            predicate.deploy(&mut env, token_address, challenge_period, exit_period);
            (predicate, env)
        }
    }

    fn get_contract_address() -> AccountId {
        AccountId::decode(&mut &[1u8; 32].to_vec()[..]).expect("account id decoded.")
    }
    fn get_token_address() -> AccountId {
        AccountId::decode(&mut &[2u8; 32].to_vec()[..]).expect("account id decoded.")
    }
    fn get_sender_address() -> AccountId {
        AccountId::decode(&mut &[3u8; 32].to_vec()[..]).expect("account id decoded.")
    }
    fn get_receiver_address() -> AccountId {
        AccountId::decode(&mut &[4u8; 32].to_vec()[..]).expect("account id decoded.")
    }

    #[test]
    fn verify_transaction_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Predicate::deploy_mock(erc20_address, 5, 5);
        let this = env.address();
        let sender = get_sender_address();
        let receiver = get_receiver_address();

        let range = Range {
            start: 0,
            end: 1000,
        };
        let state_object = StateObject {
            predicate: this.clone(),
            data: sender.clone(),
        };
        let new_state = StateObject {
            predicate: this.clone(),
            data: receiver.clone(),
        };

        let pre_state = StateUpdate {
            range: range.clone(),
            state_object: state_object.clone(),
            plasma_block_number: 0,
        };
        let post_state = StateUpdate {
            range: range.clone(),
            state_object: new_state.clone(),
            plasma_block_number: 3,
        };
        let transaction = Transaction {
            predicate: this.clone(),
            range: range.clone(),
            body: TransactionBody {
                new_state: post_state.state_object.clone(),
                origin_block: 1,
                max_block: 5,
            },
        };
        let witness = Signature([1u8; 64]);

        assert_eq!(
            true,
            contract.verify_transaction(&mut env, &pre_state, &transaction, &witness, &post_state)
        )
    }

    fn deposit_unwrap(
        contract: &mut Predicate,
        env: &mut EnvHandler<ink_core::env::ContractEnv<DefaultSrmlTypes>>,
        checkpoint: &Checkpoint<AccountId>,
    ) -> Hash {
        let checkpoint_id = checkpoint.id();
        let range = checkpoint.state_update.range.clone();
        // previous deposit.
        assert_eq!(
            Ok(CheckpointFinalized {
                checkpoint: checkpoint_id.clone()
            }),
            contract.deposit().deposit(
                env,
                checkpoint.state_update.state_object.data.clone(),
                ((range.end - range.start) as u64).try_into().unwrap(),
                checkpoint.state_update.state_object.clone(),
            )
        );
        checkpoint_id
    }

    #[test]
    fn start_exit_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Predicate::deploy_mock(erc20_address, 5, 5);
        let this = env.address();
        let sender = get_sender_address();
        let receiver = get_receiver_address();

        let range = Range {
            start: 0,
            end: 1000,
        };
        let state_object = StateObject {
            predicate: this.clone(),
            data: sender.clone(),
        };

        let checkpoint = Checkpoint {
            state_update: StateUpdate {
                range: range.clone(),
                state_object: state_object.clone(),
                plasma_block_number: 0,
            },
            sub_range: Range {
                start: range.start,
                end: range.end,
            },
        };
        let checkpoint_id = deposit_unwrap(&mut contract, &mut env, &checkpoint);

        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_caller(get_receiver_address());

        // Invalid case.
        assert_eq!(
            Err("Only owner may initiate the exit."),
            contract.start_exit(&mut env, checkpoint.clone())
        );

        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_caller(get_sender_address());

        // Check the emit value.
        assert_eq!(
            Ok(ExitStarted {
                exit: checkpoint_id,
                redeemable_after: env.block_number() + 5,
            }),
            contract.start_exit(&mut env, checkpoint.clone())
        );
    }

    #[test]
    fn finalize_exit_normal() {
        let erc20_address = get_token_address();
        let (mut contract, mut env) = Predicate::deploy_mock(erc20_address, 5, 5);
        let this = env.address();
        let sender = get_sender_address();
        let receiver = get_receiver_address();

        let range = Range {
            start: 0,
            end: 1000,
        };
        let state_object = StateObject {
            predicate: this.clone(),
            data: sender.clone(),
        };

        let checkpoint = Checkpoint {
            state_update: StateUpdate {
                range: range.clone(),
                state_object: state_object.clone(),
                plasma_block_number: 0,
            },
            sub_range: Range {
                start: range.start,
                end: range.end,
            },
        };
        let checkpoint_id = deposit_unwrap(&mut contract, &mut env, &checkpoint);

        // exit_start.
        assert_eq!(
            Ok(ExitStarted {
                exit: checkpoint_id,
                redeemable_after: env.block_number() + 5,
            }),
            contract.start_exit(&mut env, checkpoint.clone())
        );
        let deposited_range_id = 1000;

        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_block_number(10);
        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_caller(get_receiver_address());

        // Invalid case.
        assert_eq!(
            Err("Only owner may finalize the exit."),
            contract.finalize_exit(&mut env, checkpoint.clone(), deposited_range_id)
        );

        ink_core::env::ContractEnv::<DefaultSrmlTypes>::set_caller(get_sender_address());
        // check return value.
        assert_eq!(
            Ok(ExitFinalized {
                exit: checkpoint.clone()
            }),
            contract.finalize_exit(&mut env, checkpoint.clone(), deposited_range_id)
        );
    }
}
