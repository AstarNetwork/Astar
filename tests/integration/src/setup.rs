// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

//! Runtime integration tests setup & imports.

pub use frame_support::{
    assert_noop, assert_ok,
    traits::{OnFinalize, OnIdle, OnInitialize},
    weights::Weight,
};
pub use pallet_evm::AddressMapping;
pub use sp_core::{H160, H256, U256};
pub use sp_io::hashing::keccak_256;
pub use sp_runtime::{AccountId32, MultiAddress};

pub use astar_primitives::{ethereum_checked::AccountMapping, evm::UnifiedAddressMapper};

#[cfg(feature = "shibuya")]
pub use shibuya::*;
#[cfg(feature = "shibuya")]
mod shibuya {
    use super::*;
    pub use shibuya_runtime::*;

    /// 1 SBY.
    pub const UNIT: Balance = SBY;

    pub fn alith_secret_key() -> libsecp256k1::SecretKey {
        libsecp256k1::SecretKey::parse(&keccak_256(b"Alith")).unwrap()
    }

    /// H160 address mapped to `ALICE`.
    pub fn alith() -> H160 {
        UnifiedAccounts::eth_address(&alith_secret_key())
    }

    /// Convert `H160` to `AccountId32`.
    pub fn account_id_from(address: H160) -> AccountId32 {
        <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(address)
    }

    /// Deploy an EVM contract with code.
    pub fn deploy_evm_contract(code: &str) -> H160 {
        assert_ok!(EVM::create2(
            RuntimeOrigin::root(),
            alith(),
            hex::decode(code).expect("invalid code hex"),
            H256::zero(),
            U256::zero(),
            1_000_000,
            U256::from(DefaultBaseFeePerGas::get()),
            None,
            None,
            vec![],
        ));
        match System::events()
            .iter()
            .last()
            .expect("no event found")
            .event
        {
            RuntimeEvent::EVM(pallet_evm::Event::Created { address }) => address,
            _ => panic!("Deploy failed."),
        }
    }

    /// Deploy a WASM contract with its name. (The code is in `resource/`.)
    pub fn deploy_wasm_contract(name: &str) -> AccountId32 {
        let path = format!("ink-contracts/{}.wasm", name);
        let code = std::fs::read(path).expect("invalid path");
        let instantiate_result = Contracts::bare_instantiate(
            ALICE,
            0,
            Weight::from_parts(10_000_000_000, 1024 * 1024),
            None,
            pallet_contracts_primitives::Code::Upload(code),
            // `new` constructor
            hex::decode("9bae9d5e").expect("invalid data hex"),
            vec![],
            pallet_contracts::DebugInfo::Skip,
            pallet_contracts::CollectEvents::Skip,
        );

        let address = instantiate_result
            .result
            .expect("instantiation failed")
            .account_id;
        // On instantiation, the contract got existential deposit.
        assert_eq!(Balances::free_balance(&address), ExistentialDeposit::get(),);
        address
    }

    /// Build the signature payload for given native account and eth private key
    fn get_evm_signature(who: &AccountId32, secret: &libsecp256k1::SecretKey) -> [u8; 65] {
        // sign the payload
        UnifiedAccounts::eth_sign_prehash(&UnifiedAccounts::build_signing_payload(who), secret)
    }

    /// Create the mappings for the accounts
    pub fn connect_accounts(who: &AccountId32, secret: &libsecp256k1::SecretKey) {
        assert_ok!(UnifiedAccounts::claim_evm_address(
            RuntimeOrigin::signed(who.clone()),
            UnifiedAccounts::eth_address(secret),
            get_evm_signature(who, secret)
        ));
    }

    pub fn claim_default_accounts(account: AccountId) {
        let default_h160 = UnifiedAccounts::to_default_h160(&account);
        assert_ok!(UnifiedAccounts::claim_default_evm_address(
            RuntimeOrigin::signed(account.clone())
        ));
        assert_eq!(UnifiedAccounts::to_h160(&account).unwrap(), default_h160);
    }
}

#[cfg(feature = "shiden")]
pub use shiden::*;
#[cfg(feature = "shiden")]
mod shiden {
    pub use shiden_runtime::*;

    /// 1 SDN.
    pub const UNIT: Balance = SDN;
}

#[cfg(feature = "astar")]
pub use astar::*;
#[cfg(feature = "astar")]
mod astar {
    pub use astar_runtime::*;

    /// 1 ASTR.
    pub const UNIT: Balance = ASTR;
}

pub const ALICE: AccountId32 = AccountId32::new([1_u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2_u8; 32]);
pub const CAT: AccountId32 = AccountId32::new([3_u8; 32]);

pub const INITIAL_AMOUNT: u128 = 100_000 * UNIT;

pub type SystemError = frame_system::Error<Runtime>;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_dapps_staking as DappStakingCall;
pub use pallet_proxy::Event as ProxyEvent;
pub use pallet_utility::{Call as UtilityCall, Event as UtilityEvent};

pub struct ExtBuilder {
    balances: Vec<(AccountId32, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self { balances: vec![] }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(AccountId32, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime> {
            balances: self.balances,
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    ExtBuilder::default()
        .balances(vec![
            (ALICE, INITIAL_AMOUNT),
            (BOB, INITIAL_AMOUNT),
            (CAT, INITIAL_AMOUNT),
        ])
        .build()
}

// Block time: 12 seconds.
pub const BLOCK_TIME: u64 = 12_000;

pub fn run_to_block(n: u32) {
    while System::block_number() < n {
        let block_number = System::block_number();
        Timestamp::set_timestamp(block_number as u64 * BLOCK_TIME);
        DappsStaking::on_finalize(block_number);
        Authorship::on_finalize(block_number);
        Session::on_finalize(block_number);
        AuraExt::on_finalize(block_number);
        PolkadotXcm::on_finalize(block_number);
        Ethereum::on_finalize(block_number);
        BaseFee::on_finalize(block_number);

        System::set_block_number(block_number + 1);

        TransactionPayment::on_initialize(block_number);
        DappsStaking::on_initialize(block_number);
        Authorship::on_initialize(block_number);
        Aura::on_initialize(block_number);
        AuraExt::on_initialize(block_number);
        Ethereum::on_initialize(block_number);
        BaseFee::on_initialize(block_number);
        #[cfg(any(feature = "shibuya", feature = "shiden", features = "astar"))]
        RandomnessCollectiveFlip::on_initialize(block_number);

        XcmpQueue::on_idle(block_number, Weight::MAX);
        DmpQueue::on_idle(block_number, Weight::MAX);
        Contracts::on_idle(block_number, Weight::MAX);
    }
}

fn last_events(n: usize) -> Vec<RuntimeEvent> {
    frame_system::Pallet::<Runtime>::events()
        .into_iter()
        .rev()
        .take(n)
        .rev()
        .map(|e| e.event)
        .collect()
}

pub fn expect_events(e: Vec<RuntimeEvent>) {
    assert_eq!(last_events(e.len()), e);
}

/// Initialize `env_logger` for tests. It will enable logging like `DEBUG`
/// and `TRACE` in runtime.
#[allow(dead_code)]
pub fn init_env_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}
