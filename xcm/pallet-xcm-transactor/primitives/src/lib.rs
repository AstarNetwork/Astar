#![cfg_attr(not(feature = "std"), no_std)]

use num_enum::{IntoPrimitive, TryFromPrimitive};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{RuntimeDebug, H160};
use xcm::{latest::Weight, prelude::*};

pub const XCM_EXTENSION_ID: u16 = 04;

#[repr(u16)]
#[derive(TryFromPrimitive, IntoPrimitive)]
pub enum Command {
    PrepareExecute = 0,
    Execute = 1,
    ValidateSend = 2,
    Send = 3,
    NewQuery = 4,
    TakeResponse = 5,
    PalletAccountId = 6,
}

/// Type of XCM Response Query
#[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
// #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum QueryType<AccountId> {
    // No callback, store the response for manual polling
    NoCallback,
    // Call Wasm contract's method on recieving response
    // It expects the contract method to have following signature
    //     -  (query_id: QueryId, responder: Multilocation, response: Response)
    WASMContractCallback {
        contract_id: AccountId,
        selector: [u8; 4],
    },
    // Call Evm contract's method on recieving response
    // It expects the contract method to have following signature
    //     -  (query_id: QueryId, responder: Multilocation, response: Response)
    EVMContractCallback {
        contract_id: H160,
        selector: [u8; 4],
    },
}

/// Query config
#[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
// #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct QueryConfig<AccountId, BlockNumber> {
    // query type
    pub query_type: QueryType<AccountId>,
    // blocknumber after which query will be expire
    pub timeout: BlockNumber,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ValidateSendInput {
    pub dest: VersionedMultiLocation,
    pub xcm: VersionedXcm<()>,
}

pub struct PreparedExecution<Call> {
    pub xcm: Xcm<Call>,
    pub weight: Weight,
}

pub struct ValidatedSend {
    pub dest: MultiLocation,
    pub xcm: Xcm<()>,
}

#[macro_export]
macro_rules! create_error_enum {
    ($vis:vis $type_name:ident) => {
        #[repr(u32)]
        #[derive(
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::marker::Copy,
            ::core::clone::Clone,
            // crate name mismatch, 'parity-scale-codec' is crate name but in ink! contract
            // it is usually renamed to `scale`
            Encode,
            Decode,
            ::core::fmt::Debug,
            ::num_enum::IntoPrimitive,
            ::num_enum::FromPrimitive,
        )]
        #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
        $vis enum $type_name {
            Success = 0,
            NoResponse = 1,
            #[num_enum(default)]
            RuntimeError = 2,
        }
    };
}

create_error_enum!(pub Error);
