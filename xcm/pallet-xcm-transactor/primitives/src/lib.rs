#![cfg_attr(not(feature = "std"), no_std)]

use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};
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

#[repr(u32)]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug, IntoPrimitive, FromPrimitive)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub enum Error {
    Success = 0,
    NoResponse = 1,
    #[num_enum(default)]
    RuntimeError = 2,
}

#[cfg(feature = "ink-as-dependency")]
impl ink_env::chain_extension::FromStatusCode for Error {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            code => Err(code.into()),
        }
    }
}
