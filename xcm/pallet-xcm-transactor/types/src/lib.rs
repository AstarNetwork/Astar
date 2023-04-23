#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use xcm::{latest::Weight, prelude::*};

#[derive(Encode, Decode)]
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
#[derive(
    PartialEq,
    Eq,
    Copy,
    Clone,
    Encode,
    Decode,
    Debug,
    num_enum::IntoPrimitive,
    num_enum::FromPrimitive,
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
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
