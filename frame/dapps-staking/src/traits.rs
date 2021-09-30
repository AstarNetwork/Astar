use super::*;

/// Multi-VM pointer to smart contract instance.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum SmartContract<AccountId> {
    /// EVM smart contract instance.
    Evm(sp_core::H160),
    /// Wasm smart contract instance.
    Wasm(AccountId),
}

pub trait IsContract {
    fn is_contract(&self) -> bool;
}
