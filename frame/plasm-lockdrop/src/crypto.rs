//! Lockdrop authorities keys.

use sp_core::ecdsa;
use sp_io::{crypto::secp256k1_ecdsa_recover_compressed, hashing::keccak_256};
use sp_runtime::app_crypto::KeyTypeId;
use sp_std::vec::Vec;

/// Plasm Lockdrop Authority local KeyType.
///
/// For security reasons the offchain worker doesn't have direct access to the keys
/// but only to app-specific subkeys, which are defined and grouped by their `KeyTypeId`.
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"plaa");

/// SR25519 keys support
pub mod sr25519 {
    mod app_sr25519 {
        use crate::KEY_TYPE;
        use sp_runtime::app_crypto::{app_crypto, sr25519};
        app_crypto!(sr25519, KEY_TYPE);
    }

    /// An authority keypair using sr25519 as its crypto.
    #[cfg(feature = "std")]
    pub type AuthorityPair = app_sr25519::Pair;

    /// An authority signature using sr25519 as its crypto.
    pub type AuthoritySignature = app_sr25519::Signature;

    /// An authority identifier using sr25519 as its crypto.
    pub type AuthorityId = app_sr25519::Public;
}

/// ED25519 keys support
pub mod ed25519 {
    mod app_ed25519 {
        use crate::KEY_TYPE;
        use sp_runtime::app_crypto::{app_crypto, ed25519};
        app_crypto!(ed25519, KEY_TYPE);
    }

    /// An authority keypair using ed25519 as its crypto.
    #[cfg(feature = "std")]
    pub type AuthorityPair = app_ed25519::Pair;

    /// An authority signature using ed25519 as its crypto.
    pub type AuthoritySignature = app_ed25519::Signature;

    /// An authority identifier using ed25519 as its crypto.
    pub type AuthorityId = app_ed25519::Public;
}

// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
fn ethereum_signable_message(what: &[u8]) -> Vec<u8> {
    let mut l = what.len();
    let mut rev = Vec::new();
    while l > 0 {
        rev.push(b'0' + (l % 10) as u8);
        l /= 10;
    }
    let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
    v.extend(rev.into_iter().rev());
    v.extend_from_slice(what);
    v
}

// Attempts to recover the Ethereum public key from a message signature signed by using
// the Ethereum RPC's `personal_sign` and `eth_sign`.
pub fn eth_recover(s: &ecdsa::Signature, what: &[u8]) -> Option<ecdsa::Public> {
    let msg = keccak_256(&ethereum_signable_message(what));
    let public = secp256k1_ecdsa_recover_compressed(s.as_ref(), &msg).ok()?;
    Some(ecdsa::Public::from_raw(public))
}

/*
// Constructs the message that Bitcoin RPC's would sign.
fn bitcoin_signable_message(what: &[u8]) -> Vec<u8> {
    let mut l = what.len();
    let mut rev = Vec::new();
    while l > 0 {
        rev.push(b'0' + (l % 10) as u8);
        l /= 10;
    }
    let mut v = b"\x18Bitcoin Signed Message:\n".to_vec();
    v.extend(rev.into_iter().rev());
    v.extend_from_slice(what);
    v
}

// Attempts to recover the Bitcoin public key from a message signature signed by using
// the Bitcoin RPC's.
pub fn btc_recover(s: &ecdsa::Signature, what: &[u8]) -> Option<ecdsa::Public> {
    let msg = sha2_256(&bitcoin_signable_message(what));
    match secp256k1_ecdsa_recover_compressed(s.as_ref(), &msg) {
        Ok(public) => Some(ecdsa::Public::from_raw(public)),
        Err(e) => {
            panic!("recover error: {:?}", e.encode());
        }
    }
}
*/
