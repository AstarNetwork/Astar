#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{
    memory::format,
    memory::string::String,
    storage::{self,Vec,Flush}
};
use ink_lang::contract;

use parity_codec::{Encode,Decode};

type RangeNumber = u32;
type BlockNumber = u32;
type ChallengeNumber = u32;

#[derive(Clone,Encode,Decode,Default,PartialEq,Eq)]
#[cfg_attr(feature="std",derive(Debug))]
pub struct Range{
    start : RangeNumber,
    end : RangeNumber,
}

#[derive(Encode,Decode)]
#[cfg_attr(feature="std",derive(Debug))]
pub struct StateObject{
    id : String,
    predicate : AccountId,
    data: Vec<u8>,
}

#[derive(Encode,Decode)]
#[cfg_attr(feature="std",derive(Debug))]
pub struct StateUpdate {
    range : Range,
    state_object : StateObject,
    plasma_contract : AccountId,
    plasma_block_number : BlockNumber,
}

#[derive(Encode,Decode)]
#[cfg_attr(feature="std",derive(Debug))]
pub struct Checkpoint {
    state_update : StateUpdate,
    sub_range : Range,
}

#[derive(Encode,Decode)]
#[cfg_attr(feature="std",derive(Debug))]
pub struct CheckpointStatus {
    challengeable_until : BlockNumber,
    outstanding_challenges : ChallengeNumber,
}

#[derive(Encode,Decode)]
#[cfg_attr(feature="std",derive(Debug))]
pub struct Challenge {
    challenged_checkpoint : Checkpoint,
    challenging_checkpoint : Checkpoint,
}

contract! {
    #![env = ink_core::env::DefaultSrmlTypes]

    struct Deposit {
        //constant values
        TOKEN_ADDRES : storage::Value<AccountId>,
        CHALLENGE_PERIOD : storage::Value<BlockNumber>,
        EXIT_PERIOD : storage::Value<BlockNumber>,

        //changable values
        total_deposited : storage::Value<Range>,
        checkpoints : storage::HashMap<Hash,Checkpoint>,
        deposited_ranges : storage::HashMap<RangeNumber, Range>,
        exit_redeemable_after : storage::HashMap<Hash,BlockNumber>,
        challenges : storage::HashMap<Hash,bool>,

        //delete later
        value: storage::Value<bool>,
    }

    impl Deploy for Deposit {
        fn deploy(&mut self , init_ac : AccountId) {
            self.TOKEN_ADDRES.set(init_ac);
        }
    }

    impl Deposit {
        pub(external) fn flip(&mut self) {
            *self.value = !*self.value;
        }

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
        let mut contract = Deposit::deploy_mock();
    }
}
