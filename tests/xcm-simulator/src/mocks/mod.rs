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

pub(crate) mod msg_queue;
pub(crate) mod parachain;
pub(crate) mod parachain_c;
pub(crate) mod relay_chain;

use frame_support::traits::{Currency, OnInitialize};
use frame_support::weights::Weight;
use pallet_contracts_primitives::Code;
use sp_runtime::traits::Hash;
use xcm::latest::prelude::*;
use xcm_executor::traits::Convert;
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

type ContractBalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub const ALICE: sp_runtime::AccountId32 = sp_runtime::AccountId32::new([0xFAu8; 32]);
pub const INITIAL_BALANCE: u128 = 1_000_000_000_000_000_000_000_000;
pub const DAPP_STAKER_REWARD_PER_BLOCK: parachain::Balance = 1_000;
pub const DAPP_STAKER_DEV_PER_BLOCK: parachain::Balance = 250;

decl_test_parachain! {
    pub struct ParaA {
        Runtime = parachain::Runtime,
        XcmpMessageHandler = parachain::MsgQueue,
        DmpMessageHandler = parachain::MsgQueue,
        new_ext = para_ext(1),
    }
}

decl_test_parachain! {
    pub struct ParaB {
        Runtime = parachain::Runtime,
        XcmpMessageHandler = parachain::MsgQueue,
        DmpMessageHandler = parachain::MsgQueue,
        new_ext = para_ext(2),
    }
}

decl_test_parachain! {
    pub struct ParaC {
        Runtime = parachain_c::Runtime,
        XcmpMessageHandler = parachain_c::MsgQueue,
        DmpMessageHandler = parachain_c::MsgQueue,
        new_ext = para_c_ext(3),
    }
}

decl_test_relay_chain! {
    pub struct Relay {
        Runtime = relay_chain::Runtime,
        XcmConfig = relay_chain::XcmConfig,
        new_ext = relay_ext(),
    }
}

decl_test_network! {
    pub struct MockNet {
        relay_chain = Relay,
        parachains = vec![
            (1, ParaA),
            (2, ParaB),
            (3, ParaC),
        ],
    }
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;

pub type ParachainPalletXcm = pallet_xcm::Pallet<parachain::Runtime>;
// pub type ParachainAssets = pallet_assets::Pallet<parachain::Runtime>;
// pub type ParachainBalances = pallet_balances::Pallet<parachain::Runtime>;
pub type ParachainContracts = pallet_contracts::Pallet<parachain::Runtime>;

pub type NftParachainPalletXcm = pallet_xcm::Pallet<parachain_c::Runtime>;
pub type NftParachainContracts = pallet_contracts::Pallet<parachain_c::Runtime>;
// pub type ParachainXcAssetConfig = pallet_xc_asset_config::Pallet<parachain::Runtime>;

// pub fn parent_account_id() -> parachain::AccountId {
//     let location = (Parent,);
//     parachain::LocationToAccountId::convert(location.into()).unwrap()
// }

/// Derive parachain sovereign account on relay chain, from parachain Id
pub fn child_account_id(para: u32) -> relay_chain::AccountId {
    let location = (Parachain(para),);
    relay_chain::LocationToAccountId::convert(location.into()).unwrap()
}

/// Derive parachain sovereign account on a sibling parachain, from parachain Id
pub fn sibling_account_id(para: u32) -> parachain::AccountId {
    let location = (Parent, X1(Parachain(para)));
    parachain::LocationToAccountId::convert(location.into()).unwrap()
}

/// Prepare parachain test externality
pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
    use parachain::{MsgQueue, Runtime, System};

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (sibling_account_id(1), INITIAL_BALANCE),
            (sibling_account_id(2), INITIAL_BALANCE),
            (sibling_account_id(3), INITIAL_BALANCE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        MsgQueue::set_para_id(para_id.into());

        // // Premint some assets for testing
        // assert_eq!(
        //     Uniques::force_create(RuntimeOrigin::root(), 1, ALICE, true),
        //     Ok(())
        // );
        // assert_eq!(
        //     Uniques::mint(RuntimeOrigin::signed(ALICE), 1, 42, child_account_id(1)),
        //     Ok(())
        // );
        // assert_eq!(Uniques::owner(1, 42), Some(child_account_id(1)));

        parachain::DappsStaking::on_initialize(1);
        let (staker_rewards, dev_rewards) = issue_dapps_staking_rewards();
        parachain::DappsStaking::rewards(staker_rewards, dev_rewards);
    });
    ext
}

/// Prepare parachain test externality
pub fn para_c_ext(para_id: u32) -> sp_io::TestExternalities {
    use parachain_c::{MsgQueue, Runtime, System};

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (sibling_account_id(1), INITIAL_BALANCE),
            (sibling_account_id(2), INITIAL_BALANCE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        MsgQueue::set_para_id(para_id.into());
    });
    ext
}

/// Prepare relay chain test externality
pub fn relay_ext() -> sp_io::TestExternalities {
    use relay_chain::{Runtime, System};

    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap();

    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, INITIAL_BALANCE),
            (child_account_id(1), INITIAL_BALANCE),
            (child_account_id(2), INITIAL_BALANCE),
            (child_account_id(3), INITIAL_BALANCE),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// Advance parachain blocks until `block_number`.
// No effect if parachain is already at that number or exceeds it.
// pub fn advance_parachain_block_to(block_number: u64) {
//     while parachain::System::block_number() < block_number {
//         // On Finalize
//         let current_block_number = parachain::System::block_number();
//         parachain::PolkadotXcm::on_finalize(current_block_number);
//         parachain::Balances::on_finalize(current_block_number);
//         parachain::DappsStaking::on_finalize(current_block_number);
//         parachain::System::on_finalize(current_block_number);

//         // Forward 1 block
//         let current_block_number = current_block_number + 1;
//         parachain::System::set_block_number(current_block_number);
//         parachain::System::reset_events();

//         // On Initialize
//         parachain::System::on_initialize(current_block_number);
//         {
//             parachain::DappsStaking::on_initialize(current_block_number);
//             let (staker_rewards, dev_rewards) = issue_dapps_staking_rewards();
//             parachain::DappsStaking::rewards(staker_rewards, dev_rewards);
//         }
//         parachain::Balances::on_initialize(current_block_number);
//         parachain::PolkadotXcm::on_initialize(current_block_number);
//     }
// }

/// Issues and returns negative imbalances of (staker rewards, developer rewards)
fn issue_dapps_staking_rewards() -> (parachain::NegativeImbalance, parachain::NegativeImbalance) {
    (
        parachain::Balances::issue(DAPP_STAKER_REWARD_PER_BLOCK),
        parachain::Balances::issue(DAPP_STAKER_DEV_PER_BLOCK),
    )
}

/// Load a given wasm module from wasm binary contents along
/// with it's hash.
///
/// The fixture files are located under the `fixtures/` directory.
pub fn load_module<T>(
    fixture_name: &str,
) -> std::io::Result<(Vec<u8>, <T::Hashing as Hash>::Output)>
where
    T: frame_system::Config,
{
    let fixture_path = ["fixtures/", fixture_name, ".wasm"].concat();
    let wasm_binary = std::fs::read(fixture_path)?;
    let code_hash = T::Hashing::hash(&wasm_binary);
    Ok((wasm_binary, code_hash))
}

/// Load and deploy the contract from wasm binary
/// and check for successful deploy
pub fn deploy_contract<T: pallet_contracts::Config>(
    contract_name: &str,
    origin: T::AccountId,
    value: ContractBalanceOf<T>,
    gas_limit: Weight,
    storage_deposit_limit: Option<ContractBalanceOf<T>>,
    data: Vec<u8>,
) -> (T::AccountId, <T::Hashing as Hash>::Output) {
    let (code, hash) = load_module::<T>(contract_name).unwrap();
    let outcome = pallet_contracts::Pallet::<T>::bare_instantiate(
        origin,
        value,
        gas_limit,
        storage_deposit_limit,
        Code::Upload(code),
        data,
        // vec![],
        vec![],
        true,
    );

    // make sure it does not revert
    let result = outcome.result.unwrap();
    assert!(
        result.result.did_revert() == false,
        "deploy_contract: reverted - {:?}",
        result
    );
    (result.account_id, hash)
}
