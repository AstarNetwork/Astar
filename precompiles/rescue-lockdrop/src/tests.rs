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

use ethers::prelude::H256;
use sp_core::crypto::AccountId32;

use crate::mock::*;

use astar_primitives::evm::UnifiedAddressMapper;
use precompile_utils::testing::*;
use sp_core::ecdsa;

fn precompiles() -> TestPrecompileSet<TestRuntime> {
    PrecompilesValue::get()
}

#[test]
fn unify_lockdrop_account_works() {
    ExtBuilder::default().build().execute_with(|| {
        // Get Alice EVM address based on the Public Key
        let alice_eth = UnifiedAccounts::eth_address(&alice_secret());
        // Get derived AccountId from the Blake2b hash of the compressed ECDSA Public key
        let account_id = account_id(&alice_secret());
        // Sign the EIP712 payload to Claim EVM address
        let sig = get_evm_signature(&account_id, &alice_secret());

        precompiles()
            .prepare_test(
                alice_eth,
                PRECOMPILE_ADDRESS,
                PrecompileCall::claim_lock_drop_account {
                    account_id: H256::from_slice(account_id.as_ref()),
                    signature: sig.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(true);

        // Ensure that the AccountId mapped with Alice EVM account is the AccountId from the Blake2b hash of the compressed ECDSA Public key
        let alice_account_id = UnifiedAccounts::into_account_id(alice_eth);
        assert_eq!(alice_account_id, account_id);
    });
}

#[test]
fn wrong_caller_cannot_unify_derived_address() {
    ExtBuilder::default().build().execute_with(|| {
        // Get Alice EVM address based on the Public Key
        let alice_eth = UnifiedAccounts::eth_address(&alice_secret());
        // Get derived AccountId from the Blake2b hash of the compressed ECDSA Public key
        let account_id = account_id(&alice_secret());
        // Sign the EIP712 payload to Claim EVM address
        let sig = get_evm_signature(&account_id, &alice_secret());

        precompiles()
            .prepare_test(
                Bob,
                PRECOMPILE_ADDRESS,
                PrecompileCall::claim_lock_drop_account {
                    account_id: H256::from_slice(account_id.as_ref()),
                    signature: sig.into(),
                },
            )
            .expect_no_logs()
            .execute_returns(false);

        // Ensure that the AccountId mapped with Alice EVM account is the default AccountId (not the AccountId from the Blake2b hash of the compressed ECDSA Public key)
        // This means account has not been mapped
        let alice_account_id = UnifiedAccounts::into_account_id(alice_eth);
        let alice_default_account_id =
            <TestRuntime as pallet_unified_accounts::Config>::DefaultMappings::to_default_account_id(
                &alice_eth,
            );

        assert_eq!(alice_account_id, alice_default_account_id);
    });
}

fn account_id(secret: &libsecp256k1::SecretKey) -> AccountId32 {
    sp_io::hashing::blake2_256(
        ecdsa::Public::from_full(
            &libsecp256k1::PublicKey::from_secret_key(secret).serialize()[1..65],
        )
        .unwrap()
        .as_ref(),
    )
    .into()
}
