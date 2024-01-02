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

use crate::mock::*;
use crate::*;

use frame_support::{assert_ok, assert_storage_noop};
use sp_runtime::traits::Zero;

#[test]
fn sanity_check() {
    ExtBuilder::build().execute_with(|| {
        assert!(DappStakingMigration::max_call_weight()
            .all_gte(DappStakingMigration::min_call_weight()));
    });
}

#[test]
fn migrate_dapps_check() {
    ExtBuilder::build().execute_with(|| {
        init();

        // Cleanup single entry, check pre and post states
        let init_old_count = pallet_dapps_staking::RegisteredDapps::<Test>::iter().count();
        assert!(init_old_count > 0, "Sanity check.");

        let init_new_count = pallet_dapp_staking_v3::IntegratedDApps::<Test>::iter().count();
        assert!(init_new_count.is_zero(), "Sanity check.");

        assert_eq!(
            DappStakingMigration::migrate_dapps(),
            Ok(<Test as Config>::WeightInfo::migrate_dapps_success())
        );
        assert_eq!(
            init_old_count,
            pallet_dapps_staking::RegisteredDapps::<Test>::iter().count() + 1,
            "One entry should have been cleaned up."
        );
        assert_eq!(
            pallet_dapp_staking_v3::IntegratedDApps::<Test>::iter().count(),
            1,
            "Single new entry should have been added."
        );

        // Cleanup the remaining entries.
        for _ in 1..init_old_count {
            assert_eq!(
                DappStakingMigration::migrate_dapps(),
                Ok(<Test as Config>::WeightInfo::migrate_dapps_success())
            );
        }

        // Further calls should result in Err
        assert_eq!(
            DappStakingMigration::migrate_dapps(),
            Err(<Test as Config>::WeightInfo::migrate_dapps_noop())
        );
    });
}

#[test]
fn migrate_ledgers_check() {
    ExtBuilder::build().execute_with(|| {
        init();

        // Cleanup all enries, check pre and post states.
        let init_old_count = pallet_dapps_staking::Ledger::<Test>::iter().count();
        assert!(init_old_count > 0, "Sanity check.");

        let init_new_count = pallet_dapp_staking_v3::Ledger::<Test>::iter().count();
        assert!(init_new_count.is_zero(), "Sanity check.");

        assert!(pallet_dapp_staking_v3::CurrentEraInfo::<Test>::get()
            .total_locked
            .is_zero());

        for x in 0..init_old_count {
            assert_eq!(
                DappStakingMigration::migrate_ledger(),
                Ok(<Test as Config>::WeightInfo::migrate_ledger_success())
            );

            assert_eq!(
                init_old_count - x - 1,
                pallet_dapps_staking::Ledger::<Test>::iter().count(),
                "One entry should have been cleaned up."
            );
            assert_eq!(
                x + 1,
                pallet_dapp_staking_v3::Ledger::<Test>::iter().count(),
                "Single new entry should have been added."
            );
            assert!(pallet_dapp_staking_v3::CurrentEraInfo::<Test>::get().total_locked > 0);
        }

        // Further calls should result in Err
        assert_eq!(
            DappStakingMigration::migrate_ledger(),
            Err(<Test as Config>::WeightInfo::migrate_ledger_noop())
        );
    });
}

// TODO: this doesn't work since clear_prefix doesn't work in tests for some reason.
#[ignore]
#[test]
fn storage_cleanup_check() {
    let mut ext = ExtBuilder::build();
    assert_ok!(ext.commit_all());

    ext.execute_with(|| {
        init();

        let init_count = (pallet_dapps_staking::RegisteredDapps::<Test>::iter().count()
            + pallet_dapps_staking::Ledger::<Test>::iter().count()) as u32;

        for _ in 0..init_count {
            assert_ok!(DappStakingMigration::cleanup_old_storage(init_count));
        }
    });
}

#[test]
fn migrate_call_works() {
    ExtBuilder::build().execute_with(|| {
        init();
        let account = 1;

        // Call enough times to clean everything up.
        while MigrationStateStorage::<Test>::get() != MigrationState::Finished {
            assert_ok!(DappStakingMigration::migrate(
                frame_system::RawOrigin::Signed(account).into(),
                Some(Weight::from_parts(1, 1))
            ));

            match MigrationStateStorage::<Test>::get() {
                MigrationState::RegisteredDApps | MigrationState::Ledgers => {
                    assert!(
                        pallet_dapp_staking_v3::ActiveProtocolState::<Test>::get().maintenance,
                        "Pallet must be in the maintenance mode during old storage migration."
                    );
                }
                _ => {
                    assert!(
                        !pallet_dapp_staking_v3::ActiveProtocolState::<Test>::get().maintenance,
                        "Maintenance mode is disabled during old storage cleanup."
                    );
                }
            }
        }

        // Check post-state
        assert!(pallet_dapps_staking::RegisteredDapps::<Test>::iter()
            .count()
            .is_zero());
        assert!(pallet_dapps_staking::Ledger::<Test>::iter()
            .count()
            .is_zero());
        assert!(pallet_dapps_staking::RegisteredDevelopers::<Test>::iter()
            .count()
            .is_zero());
        assert!(pallet_dapps_staking::GeneralEraInfo::<Test>::iter()
            .count()
            .is_zero());

        // Migrate call can still be called, but it shouldn't have any effect.
        assert_storage_noop!(assert_ok!(DappStakingMigration::migrate(
            frame_system::RawOrigin::Signed(account).into(),
            None
        )));
    });
}
