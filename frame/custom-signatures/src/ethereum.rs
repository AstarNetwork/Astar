//! Ethereum signature compatibility instances.

use sp_io::{crypto::secp256k1_ecdsa_recover_compressed, hashing::keccak_256};
use sp_runtime::traits::{Verify, Lazy, IdentifyAccount};
use codec::{Encode, Decode};
use sp_core::ecdsa;

/// Ethereum compatible signature type.
#[derive(Encode, Decode, Clone)]
pub struct EthereumSignature(pub [u8; 65]);

impl PartialEq for EthereumSignature {
    fn eq(&self, other: &Self) -> bool {
        &self.0[..] == &other.0[..]
    }
}

impl sp_std::fmt::Debug for EthereumSignature {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "EcdsaSignature({:?})", &self.0[..])
    }
}

/// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
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

/// Attempts to recover the Ethereum public key from a message signature signed by using
/// the Ethereum RPC's `personal_sign` and `eth_sign`.
impl Verify for EthereumSignature {
    type Signer = ecdsa::Public;

    fn verify<L: Lazy<[u8]>>(&self, mut msg: L, signer: &<Self::Signer as IdentifyAccount>::AccountId) -> bool {
        let msg = msg.get();
        let msg = keccak_256(&ethereum_signable_message(&msg));
        if let Some(public) = secp256k1_ecdsa_recover_compressed(&self.0, &msg).ok() {
            ecdsa::Public::from_raw(public).into_account() == *signer
        } else {
            false
        }
    }
}

#[test]
fn verify_works() {
    use sp_core::{ecdsa, Pair};
    use hex_literal::hex;

    let msg = "test eth signed message";
    let pair = ecdsa::Pair::from_seed(&hex![
        "7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
    ]);
    let signature = EthereumSignature(hex!["dd0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"]);
    assert_eq!(signature.verify(msg.as_ref(), &pair.public().into_account()), true);
}
