use codec::Encode;
use sp_core::{ecdsa, keccak_256, U256};
use web3::types::Address;

/// Get Ethereum address for given ECDSA public key.
pub fn to_address(public: &ecdsa::Public) -> Address {
    let pubkey = secp256k1::PublicKey::parse_slice(public.as_ref(), None)
        .expect("ecdsa contains correct public key");
    let mut output = [0u8; 20];
    output.copy_from_slice(&keccak_256(&pubkey.serialize()[..]));
    output.into()
}

/// Compile smart contract input for lock value on given duration.
pub fn lock_method(duration: u64) -> Vec<u8> {
    let mut method = vec![0xdd, 0x46, 0x70, 0x64]; // lock(uint256) signature
    method.extend(&U256::from(duration).encode()[..]);
    method
}
