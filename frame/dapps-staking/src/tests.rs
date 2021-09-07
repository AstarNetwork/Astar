use super::{Event, *};
use frame_support::{assert_err, assert_noop, assert_ok};
use mock::{Balances, *};

#[test]
fn bonding_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // test that bonding works with amount that is less than available on in stash
        let stash1_id = 1;
        let stash1_signed_id = Origin::signed(stash1_id);
        let controller1_id = 2u64;
        let staking1_amount = Balances::free_balance(&stash1_id) - 1;
        assert_ok!(DappsStaking::bond(
            stash1_signed_id,
            controller1_id,
            staking1_amount,
            crate::RewardDestination::Staked
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash1_id,
            staking1_amount,
        )));

        // test that bonding works with amount that is equal to existential deposit
        let stash2_id = 2;
        let stash2_signed_id = Origin::signed(stash2_id);
        let controller2_id = 4u64;
        let staking2_amount = EXISTENTIAL_DEPOSIT;
        assert_ok!(DappsStaking::bond(
            stash2_signed_id,
            controller2_id,
            staking2_amount,
            crate::RewardDestination::Stash
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash2_id,
            staking2_amount,
        )));

        // test that bonding works with the amount that equals the entire stash
        let stash3_id = 540;
        let stash3_signed_id = Origin::signed(stash3_id);
        let controller3_id = 6u64;
        let stash3_free_amount = Balances::free_balance(&stash3_id);
        assert_ok!(DappsStaking::bond(
            stash3_signed_id,
            controller3_id,
            stash3_free_amount,
            crate::RewardDestination::Stash
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash3_id,
            stash3_free_amount,
        )));

        // TODO: Confirm that this is correct behavior.
        // test that bonding works when more is staked than available in stash
        let stash4_id = 3;
        let stash4_signed_id = Origin::signed(stash4_id);
        let controller4_id = 8u64;
        let stash4_free_amount = Balances::free_balance(&stash4_id);
        assert_ok!(DappsStaking::bond(
            stash4_signed_id,
            controller4_id,
            stash4_free_amount + 1,
            crate::RewardDestination::Stash
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash4_id,
            stash4_free_amount,
        )));

        // TODO: should we also check the storage content?
    })
}

#[test]
fn bonding_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = Origin::signed(1);
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

        // use already paired controller with a new stash, expect error AlreadyPaired
        let stash2_id = Origin::signed(2);
        assert_err!(
            DappsStaking::bond(
                stash2_id.clone(),
                controller_id,
                50,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyPaired
        );

        // try to stake less than minimum amount, expect error InsufficientValue
        let controller2_id = 20u64;
        assert_err!(
            DappsStaking::bond(
                stash2_id,
                controller2_id,
                EXISTENTIAL_DEPOSIT - 1,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::InsufficientValue
        );
    })
}

#[test]
fn bonding_extra_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id: u64 = 1;
        let stash1_id_signed = Origin::signed(stash1_id);
        let controller1_id = 20u64;
        let stash1_amount = Balances::free_balance(&stash1_id);

        // TODO: do I need to re-assert this? Can I make this UT dependent on previous? Is it worth the complication since these tests are pretty simple?
        assert_ok!(DappsStaking::bond(
            stash1_id_signed.clone(),
            controller1_id,
            stash1_amount - 1000,
            crate::RewardDestination::Staked
        ));

        // stake extra funds and expect a pass
        let first_extra_amount: mock::Balance = 900;
        assert_ok!(DappsStaking::bond_extra(
            stash1_id_signed.clone(),
            first_extra_amount
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash1_id,
            first_extra_amount,
        )));

        // stake remaining funds and expect a pass
        let second_extra_amount: mock::Balance = 100;
        assert_ok!(DappsStaking::bond_extra(
            stash1_id_signed.clone(),
            second_extra_amount
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash1_id,
            second_extra_amount,
        )));

        // TODO: if we stake additional funds, it will 'pass' but nothing will happen, no events will be deposited.
        // Is that correct??? Do we need a new error for this?
        // let third_extra_amount: mock::Balance = 10;
        // assert_noop!(
        //     DappsStaking::bond_extra(stash1_id_signed,
        //     third_extra_amount),
        //     <some error???>
        // );
    })
}

#[test]
fn bonding_extra_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = Origin::signed(1);
        let controller1_id = 20u64;

        assert_err!(
            DappsStaking::bond_extra(stash1_id, 10),
            crate::pallet::pallet::Error::<TestRuntime>::NotStash
        );
    })
}

#[test]
fn set_controller_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = Origin::signed(1);
        let controller1_id = 20u64;

        // add stash and controller by bonding
        assert_ok!(DappsStaking::bond(
            stash1_id.clone(),
            controller1_id,
            50,
            crate::RewardDestination::Staked
        ));

        // set a new controller, different from the old one
        let new_controller1_id = 30u64;
        assert_ok!(DappsStaking::set_controller(
            stash1_id.clone(),
            new_controller1_id
        ));
        // TODO: should we have some event here?
    })
}

#[test]
fn set_controller_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = Origin::signed(1);
        let controller1_id = 20u64;

        // stash doesn't exist yet, expect error NotStash
        assert_noop!(
            DappsStaking::set_controller(stash1_id.clone(), controller1_id),
            crate::pallet::pallet::Error::<TestRuntime>::NotStash
        );

        // add stash and controller by bonding
        assert_ok!(DappsStaking::bond(
            stash1_id.clone(),
            controller1_id,
            50,
            crate::RewardDestination::Staked
        ));

        // try to set the old controller, expect error AlreadyPaired
        assert_noop!(
            DappsStaking::set_controller(stash1_id.clone(), controller1_id),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyPaired
        );
    })
}
