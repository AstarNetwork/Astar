//! Ethereum prefixed signatures compatibility instances.

use codec::{Decode, Encode};
use sp_core::ecdsa;
use sp_io::{crypto::secp256k1_ecdsa_recover_compressed, hashing::keccak_256};
use sp_runtime::traits::{IdentifyAccount, Lazy, Verify};
use sp_runtime::MultiSignature;
use sp_std::prelude::*;

/// Ethereum-compatible signature type.
#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub struct EthereumSignature(pub [u8; 65]);

impl sp_std::fmt::Debug for EthereumSignature {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "EthereumSignature({:?})", &self.0[..])
    }
}

impl From<ecdsa::Signature> for EthereumSignature {
    fn from(signature: ecdsa::Signature) -> Self {
        Self(signature.into())
    }
}

impl sp_std::convert::TryFrom<Vec<u8>> for EthereumSignature {
    type Error = ();

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        if data.len() == 65 {
            let mut inner = [0u8; 65];
            inner.copy_from_slice(&data[..]);
            Ok(EthereumSignature(inner))
        } else {
            Err(())
        }
    }
}

/// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
pub fn signable_message(what: &[u8]) -> Vec<u8> {
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
    type Signer = <MultiSignature as Verify>::Signer;

    fn verify<L: Lazy<[u8]>>(
        &self,
        mut msg: L,
        account: &<Self::Signer as IdentifyAccount>::AccountId,
    ) -> bool {
        let msg = keccak_256(&signable_message(&msg.get()));
        match secp256k1_ecdsa_recover_compressed(&self.0, &msg).ok() {
            Some(public) => {
                let signer = Self::Signer::from(ecdsa::Public::from_raw(public));
                *account == signer.into_account()
            }
            None => false,
        }
    }
}

#[test]
fn verify_should_works() {
    use hex_literal::hex;
    use sp_core::{ecdsa, Pair};

    let msg = "test eth signed message";
    let pair = ecdsa::Pair::from_seed(&hex![
        "7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
    ]);
    let account = <MultiSignature as Verify>::Signer::from(pair.public()).into_account();
    let signature = EthereumSignature(hex!["dd0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"]);
    assert_eq!(signature.verify(msg.as_ref(), &account), true);
}
