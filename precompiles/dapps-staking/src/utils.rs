use super::*;
use evm::ExitError;
use sp_core::{H160, U256};

pub(crate) const SELECTOR_SIZE_BYTES: usize = 4;
pub(crate) const ARG_SIZE_BYTES: usize = 32;
pub(crate) const OFFSET_H160: usize = 12;

/// Smart contract enum. TODO move this to Astar primitives.
/// This is only used to encode SmartContract enum
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum Contract<A> {
    /// EVM smart contract instance.
    Evm(H160),
    /// Wasm smart contract instance. Not used in this precompile
    Wasm(A),
}

#[derive(Clone, Copy, Debug)]
pub struct EvmInArg<'a> {
    pub input: &'a [u8],
}

impl<'a> EvmInArg<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    /// parse selector from the first 4 bytes.
    pub fn selector(&self) -> Result<[u8; SELECTOR_SIZE_BYTES], &'static str> {
        match self.len() {
            l if l < SELECTOR_SIZE_BYTES => Err("Selector too short"),
            _ => {
                let mut selector = [0u8; SELECTOR_SIZE_BYTES];
                selector.copy_from_slice(&self.input[0..SELECTOR_SIZE_BYTES]);
                Ok(selector)
            }
        }
    }

    /// check that the length of input is as expected
    pub fn expecting_arguments(&self, num: usize) -> Result<(), &'static str> {
        let expected_len = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * num;
        match self.len() {
            l if l < expected_len => Err("Too few arguments"),
            l if l > expected_len => Err("Too many arguments"),
            _ => Ok(()),
        }
    }

    /// Parse input and return H160 argument from given position
    pub fn to_h160(&self, position: usize) -> H160 {
        let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
        let end = offset + ARG_SIZE_BYTES;
        // H160 has 20 bytes. The first 12 bytes in u256 have no meaning
        sp_core::H160::from_slice(&self.input[(offset + OFFSET_H160)..end]).into()
    }

    /// Parse input and return U256 argument from given position
    pub fn to_u256(&self, position: usize) -> U256 {
        let offset = SELECTOR_SIZE_BYTES + ARG_SIZE_BYTES * (position - 1);
        let end = offset + ARG_SIZE_BYTES;
        sp_core::U256::from_big_endian(&self.input[offset..end])
    }

    /// The length of the input argument
    pub fn len(&self) -> usize {
        self.input.len()
    }
}

/// Store u32 value in the 32 bytes vector as big endian
pub fn argument_from_u32(value: u32) -> Vec<u8> {
    let mut buffer = [0u8; ARG_SIZE_BYTES];
    buffer[ARG_SIZE_BYTES - core::mem::size_of::<u32>()..].copy_from_slice(&value.to_be_bytes());
    buffer.to_vec()
}

/// Store u128 value in the 32 bytes vector as big endian
pub fn argument_from_u128(value: u128) -> Vec<u8> {
    let mut buffer = [0u8; ARG_SIZE_BYTES];
    buffer[ARG_SIZE_BYTES - core::mem::size_of::<u128>()..].copy_from_slice(&value.to_be_bytes());
    buffer.to_vec()
}

/// Store H160 value in the 32 bytes vector as big endian
pub fn argument_from_h160(value: H160) -> Vec<u8> {
    let mut buffer = [0u8; ARG_SIZE_BYTES];
    buffer[ARG_SIZE_BYTES - core::mem::size_of::<H160>()..]
        .copy_from_slice(&value.to_fixed_bytes());
    buffer.to_vec()
}

/// Store H160 value which is encoded as SmartContract in the 32 bytes vector as big endian
/// This encoded SmartContract has 1 byte more than H160 (=21 bytes)
/// First create buffer of size 32-21 and append encoded input value of size 21
pub fn argument_from_h160_vec(mut value: Vec<u8>) -> Vec<u8> {
    const ENCODED_LEN: usize = core::mem::size_of::<H160>() + 1; // 21
    let mut buffer = Vec::<u8>::from([0; ARG_SIZE_BYTES - ENCODED_LEN]);
    buffer.append(&mut value);
    buffer
}

/// Store u8 array of 20 bytes in the 32 bytes vector as big endian
pub fn argument_from_h160_array(value: [u8; 20]) -> Vec<u8> {
    let mut buffer = Vec::<u8>::from([0; ARG_SIZE_BYTES]);
    buffer[ARG_SIZE_BYTES - core::mem::size_of::<H160>()..].copy_from_slice(&value);
    buffer
}

/// Returns an evm error with provided (static) text.
pub fn exit_error<T: Into<alloc::borrow::Cow<'static, str>>>(text: T) -> ExitError {
    ExitError::Other(text.into())
}
