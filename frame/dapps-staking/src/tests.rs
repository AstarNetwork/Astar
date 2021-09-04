use super::{Event, *};
use frame_support::{assert_err, assert_noop, assert_ok};
use mock::*;

#[test]
fn bonding_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = Origin::signed(1);
        let controller_id = 2u64;

        assert_ok!(DappsStaking::bond(
            stash_id,
            controller_id,
            50,
            crate::RewardDestination::Staked
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(1, 50)));
    })
}

#[test]
fn bonding_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = Origin::signed(1);
        let stash2_id = Origin::signed(2);
        let controller_id = 3u64;

        assert_ok!(DappsStaking::bond(
            stash_id.clone(),
            controller_id.clone(),
            50,
            crate::RewardDestination::Staked
        ));

        // repeat bonding with same stash account, expect error AlreadyBonded
        assert_err!(
            DappsStaking::bond(
                stash_id,
                controller_id.clone(),
                50,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyBonded
        );

        // use already paired controller, expect error AlreadyPaired
        assert_err!(
            DappsStaking::bond(
                stash2_id,
                controller_id,
                50,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyPaired
        );
    })
}
