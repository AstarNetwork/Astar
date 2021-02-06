//! Low-level types used throughout the node code.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature, OpaqueExtrinsic, RuntimeDebug,
};
use sp_std::{
    convert::TryFrom,
    prelude::*,
};

#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// The amount type, should be signed version of balance.
pub type Amount = i128;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Supported tokey symbols.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenSymbol {
    /// Acala native token.
    ACA = 0,
    /// Acala stable coin.
    AUSD = 1,
    /// Polkadot native token.
    DOT = 2,
    /// Wrapped BTC.
    XBTC = 3,
    /// Liquid DOT token.
    LDOT = 4,
    /// BTC wrapped by RenVM.
    RENBTC = 5,
    /// Shiden native token.
    SDN = 6,
    /// Plasm native token.
    PLM = 7,
}

impl TryFrom<u8> for TokenSymbol {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
           0 => Ok(TokenSymbol::ACA),
           1 => Ok(TokenSymbol::AUSD),
           2 => Ok(TokenSymbol::DOT),
           3 => Ok(TokenSymbol::XBTC),
           4 => Ok(TokenSymbol::LDOT),
           5 => Ok(TokenSymbol::RENBTC),
           6 => Ok(TokenSymbol::SDN),
           7 => Ok(TokenSymbol::PLM),
           _ => Err(()),
        }
    }
}

/// Currency identifier.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    Token(TokenSymbol),
}

impl TryFrom<Vec<u8>> for CurrencyId {
    type Error = ();
    fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
        match v.as_slice() {
            b"ACA" => Ok(CurrencyId::Token(TokenSymbol::ACA)),
            b"AUSD" => Ok(CurrencyId::Token(TokenSymbol::AUSD)),
            b"DOT" => Ok(CurrencyId::Token(TokenSymbol::DOT)),
            b"XBTC" => Ok(CurrencyId::Token(TokenSymbol::XBTC)),
            b"LDOT" => Ok(CurrencyId::Token(TokenSymbol::LDOT)),
            b"RENBTC" => Ok(CurrencyId::Token(TokenSymbol::RENBTC)),
            b"SDN" => Ok(CurrencyId::Token(TokenSymbol::SDN)),
            b"PLM" => Ok(CurrencyId::Token(TokenSymbol::PLM)),
            _ => Err(()),
        }
    }
}
