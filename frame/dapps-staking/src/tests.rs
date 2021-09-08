use super::{Event, *};
use frame_support::{assert_err, assert_noop, assert_ok, assert_storage_noop};
use mock::{Balances, *};

// TODO: Split chunky unit tests into more simple ones. It should be clear from the TC name what is being tested.

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

#[test]
fn unbond_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // prepare stash-controller pair with some bonded funds
        let stash_id = 1;
        let controller_id = 100;
        let bond_amount = 50 + EXISTENTIAL_DEPOSIT;
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            bond_amount,
            crate::RewardDestination::Staked
        ));

        // unbond a valid amout
        assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 50));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id, 50,
        )));

        // unbond 1 value and expect to unbond everything remaining since we come under the existintial limit
        assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 1));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id,
            EXISTENTIAL_DEPOSIT,
        )));

        // at this point there's nothing more to unbond but we can still call unbond
        // TODO: should this raise an error if nothing is bonded?
    })
}

#[test]
fn unbond_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let controller_id = 10;

        // try to unbond using non-existing controller, expect error NotController
        assert_noop!(
            DappsStaking::unbond(Origin::signed(controller_id), 100),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );

        // bond using controller id as stash id in order to verify that it's still not possible to unbond using controller id
        let another_controller_id = 100u64;
        assert_ok!(DappsStaking::bond(
            Origin::signed(controller_id),
            another_controller_id,
            100,
            crate::RewardDestination::Staked
        ));

        // try to unbond using stash id, expect error NotController
        assert_noop!(
            DappsStaking::unbond(Origin::signed(controller_id), 100),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );

        // TODO: should this be a configurable constant instead? Or is it practice to hardcode constants like this sometimes?
        // remove values up to MAX_UNLOCKING_CHUNKS and expect everything to work
        for chunk in 1..=MAX_UNLOCKING_CHUNKS {
            assert_ok!(DappsStaking::unbond(
                Origin::signed(another_controller_id),
                1
            ));
        }
        assert_noop!(
            DappsStaking::unbond(Origin::signed(another_controller_id), 1),
            crate::pallet::pallet::Error::<TestRuntime>::NoMoreChunks
        );
    })
}

#[test]
fn withdraw_unbonded_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 1;
        let controller_id = 10;
        let bond_amount: Balance = 100;

        // create a bond
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            bond_amount,
            crate::RewardDestination::Staked
        ));

        // unbond some amount, the remainder bond should remain above existential deposit
        let first_unbond_amount = (bond_amount - 2 * EXISTENTIAL_DEPOSIT) / 2;
        assert_ok!(DappsStaking::unbond(
            Origin::signed(controller_id),
            first_unbond_amount
        ));
        // repeat the unbond twice with the same amount so we get two chunks
        assert_ok!(DappsStaking::unbond(
            Origin::signed(controller_id),
            first_unbond_amount
        ));

        // verify that withdraw works even if no chunks are available (era has not advanced enough)
        let current_era = <CurrentEra<TestRuntime>>::get().unwrap_or(Zero::zero());
        <CurrentEra<TestRuntime>>::put(current_era + BONDING_DURATION - 1);
        assert_storage_noop!(DappsStaking::withdraw_unbonded(Origin::signed(
            controller_id
        )));
        // no withdraw event should have happened, the old unbond event should still be the last
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id,
            first_unbond_amount,
        )));

        // advance the era by 1 so we satisfy the bonding duration for chunks
        let current_era = <CurrentEra<TestRuntime>>::get().unwrap();
        <CurrentEra<TestRuntime>>::put(current_era + 1);

        // verify that we withdraw both chunks that were unbonded
        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(
            controller_id
        )));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Withdrawn(
            stash_id,
            2 * first_unbond_amount,
        )));

        // At this point, we have bonded 2 * EXISTENTIAL_DEPOSIT
        // Unbond just enough to go below existential deposit and verify that entire bond is released
        // assert_ok!(
        //     DappsStaking::unbond(
        //         Origin::signed(controller_id),
        //         EXISTENTIAL_DEPOSIT + 1
        //     )
        // );
        // assert_ok!(
        //     DappsStaking::withdraw_unbonded(
        //         Origin::signed(controller_id)
        //     )
        // );
        // System::assert_last_event(mock::Event::DappsStaking(crate::Event::Withdrawn(
        //     stash_id,
        //     2 * EXISTENTIAL_DEPOSIT,
        // )));

        // TODO: What about this scenario? It will fail since BondingDuration needs to pass AFTER unbond was called.
        // So if BONDING_DURATION is e.g. 5, we bonded our funds at era 0 and we unbond them at era 100, we won't be able to withdraw them by era 105.
        // Is this by design? Or is it a flaw that needs to be fixed?
    })
}

#[test]
fn withdraw_unbonded_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let controller_id = 10;

        assert_noop!(
            DappsStaking::withdraw_unbonded(Origin::signed(controller_id)),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );

        // TODO: should we raise some error in case noop was performed due to not having anything to withdraw?
    })
}
