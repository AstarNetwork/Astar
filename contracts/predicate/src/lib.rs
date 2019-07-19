#![cfg_attr(not(any(test, feature = "std")), no_std)]

use core::option::Option;
use ink_core::{
    env::{ContractEnv, DefaultSrmlTypes, EnvTypes},
    memory::vec::Vec,
};
use ink_model::EnvHandler;
use parity_codec::{Codec, Decode, Encode};
use primitives::traits::{Member, SimpleArithmetic};

pub mod ownership;
pub mod traits;

type AccountId = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::AccountId;
type Balance = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Balance;
type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut contract = Deposit::deploy_mock();
    }
}
