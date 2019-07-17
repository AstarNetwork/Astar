#![cfg_attr(not(any(test, feature = "std")), no_std)]
use ink_core::env::{ContractEnv, DefaultSrmlTypes, EnvTypes};
use parity_codec::Codec;
use core::option::Option;

pub mod traits;
pub mod cash;

type BlockNumber = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::BlockNumber;
type Hash = <ContractEnv<DefaultSrmlTypes> as EnvTypes>::Hash;
