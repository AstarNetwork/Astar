#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{
	memory::{string::String, vec::Vec, format},
	storage,

use ink_lang::contract;
use primitives::*;
use ink_model::gen_input_data::gen_input_data;

contract! {
    #![env = ink_core::env::DefaultSrmlTypes]

    event CheckpointStarted{
        checkpoint : Checkpoint,
        challengeable_until: BlockNumber,
    }

    event CheckpointChallenged{
        challenge : Challenge,
    }

    event CheckpointFinalized{
        checkpoint: Hash,
    }

    event ExitStarted{
        exit : Hash,
        redeemable_after : BlockNumber,
    }

    event ExitFinalized{
        exit : Checkpoint,
    }

    struct Deposit {
        //constant values
        COMMITMENT_ADDRESS : storage::Value<AccountId>,
        //MUST be an adress of ERC20 token
        TOKEN_ADDRES : storage::Value<AccountId>,
        CHALLENGE_PERIOD : storage::Value<BlockNumber>,
        EXIT_PERIOD : storage::Value<BlockNumber>,

        //changable values
        total_deposited : storage::Value<Range>,
        checkpoints : storage::HashMap<Hash,CheckpointStatus>,
        deposited_ranges : storage::HashMap<RangeNumber, Range>,
        exit_redeemable_after : storage::HashMap<Hash,BlockNumber>,
        challenges : storage::HashMap<Hash,bool>,
    }

    impl Deploy for Deposit {
        fn deploy(&mut self , init_ac : AccountId) {
            self.TOKEN_ADDRES.set(init_ac);
        }
    }

    impl Deposit {
        pub(external) fn deposit(&mut self, depositer : AccountId, amount : Balance, initial_state : StateObject){
            //MUST keep track of the total deposited assets, totalDeposited.
            //MUST transfer the deposited amount from the depositer to the deposit contract’s address.
			let params = vec!([Box::new(depositer),Box::new(env.address()),Box::new(amount)]);
			let sent:bool = env.call(TOKEN_ADDRES,0,0,gen_input_data("transfer_from",params));

            //MUST create a state update with a state object equal to the provided initialState.
			let state_update = StateUpdate{
				range : Range,
				state_object : initial_state,
				plasma_contract,
				plasma_block_number,
			};
			//MUST compute the range of the created state update as totalDeposited to totalDeposited + amount.


            //MUST update the total amount deposited after the deposit is handled.
            *self.total_deposited = *self.total_deposited + amount;

            //MUST insert the created state update into the checkpoints mapping with challengeableUntil being the current block number - 1.



            //MUST emit a CheckpointFinalized event for the inserted checkpoint.
            env.emit(
                CheckpointFinalized{
                    checkpoint,
                }
            );
        }

    }
}

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut contract = Deposit::deploy_mock();
    }
}
