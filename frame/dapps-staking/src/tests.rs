use super::{Event, *};
use frame_support::{assert_err, assert_noop, assert_ok, assert_storage_noop};
use mock::{Balances, *};

// TODO: Add checks that verify content of the storage!

#[test]
fn bonding_less_than_stash_amount_is_ok() {
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
    })
}

#[test]
fn bonding_existential_deposit_amount_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
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
    })
}

#[test]
fn bonding_entire_stash_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
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
    })
}

#[test]
fn bonding_more_than_in_stash_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // test that bonding works when more is staked than available in stash
        let stash4_id = 3;
        let controller4_id = 8u64;
        let stash4_free_amount = Balances::free_balance(&stash4_id);
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash4_id),
            controller4_id,
            stash4_free_amount + 1,
            crate::RewardDestination::Stash
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash4_id,
            stash4_free_amount,
        )));
    })
}

#[test]
fn bonding_less_than_exist_deposit_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // try to stake less than minimum amount, expect error InsufficientValue
        let stash2_id = 2;
        let controller2_id = 20u64;
        assert_noop!(
            DappsStaking::bond(
                Origin::signed(2),
                controller2_id,
                EXISTENTIAL_DEPOSIT - 1,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::InsufficientValue
        );
    })
}

#[test]
fn bonding_with_same_stash_or_controller_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 1;
        let controller_id = 3u64;

        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            50,
            crate::RewardDestination::Staked
        ));

        // repeat bonding with same stash account, expect error AlreadyBonded
        assert_noop!(
            DappsStaking::bond(
                Origin::signed(stash_id),
                controller_id,
                50,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyBonded
        );

        // use already paired controller with a new stash, expect error AlreadyPaired
        let stash2_id = 2;
        assert_noop!(
            DappsStaking::bond(
                Origin::signed(stash2_id),
                controller_id,
                50,
                crate::RewardDestination::Staked
            ),
            crate::pallet::pallet::Error::<TestRuntime>::AlreadyPaired
        );
    })
}

#[test]
fn bonding_extra_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id: u64 = 1;
        let controller1_id = 20u64;
        let stash1_amount = Balances::free_balance(&stash1_id);

        assert_ok!(DappsStaking::bond(
            Origin::signed(stash1_id),
            controller1_id,
            stash1_amount - 1000,
            crate::RewardDestination::Staked
        ));

        // bond extra funds and expect a pass
        let first_extra_amount: mock::Balance = 900;
        assert_ok!(DappsStaking::bond_extra(
            Origin::signed(stash1_id),
            first_extra_amount
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash1_id,
            first_extra_amount,
        )));

        // bond remaining funds and expect a pass
        let second_extra_amount: mock::Balance = 100;
        assert_ok!(DappsStaking::bond_extra(
            Origin::signed(stash1_id),
            second_extra_amount
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Bonded(
            stash1_id,
            second_extra_amount,
        )));

        // TODO: if we bond additional funds, it will 'pass' but nothing will happen, no events will be deposited.
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
fn bonding_extra_with_controller_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        assert_noop!(
            DappsStaking::bond_extra(Origin::signed(stash1_id), 10),
            crate::pallet::pallet::Error::<TestRuntime>::NotStash
        );
    })
}

#[test]
fn set_controller_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        // add stash and controller by bonding
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash1_id),
            controller1_id,
            50,
            crate::RewardDestination::Staked
        ));
        // set a new controller, different from the old one
        let new_controller1_id = 30u64;
        assert_ok!(DappsStaking::set_controller(
            Origin::signed(stash1_id),
            new_controller1_id
        ));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::ControllerChanged(
            stash1_id,
            new_controller1_id,
        )));
    })
}

#[test]
fn set_controller_for_non_existing_stash_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        // stash doesn't exist yet, expect error NotStash
        assert_noop!(
            DappsStaking::set_controller(Origin::signed(stash1_id), controller1_id),
            crate::pallet::pallet::Error::<TestRuntime>::NotStash
        );
    })
}

#[test]
fn set_controller_twice_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash1_id = 1;
        let controller1_id = 20u64;

        // add stash and controller by bonding
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash1_id),
            controller1_id,
            50,
            crate::RewardDestination::Staked
        ));

        // try to set the old controller, expect error AlreadyPaired
        assert_noop!(
            DappsStaking::set_controller(Origin::signed(stash1_id), controller1_id),
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
        assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 1));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Unbonded(
            stash_id,
            Zero::zero(),
        )));
    })
}

#[test]
fn unbond_with_non_existing_controller_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // try to unbond using non-existing controller, expect error NotController
        let controller_id = 10;
        assert_noop!(
            DappsStaking::unbond(Origin::signed(controller_id), 100),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );
    })
}

#[test]
fn unbond_with_stash_id_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 10;
        let controller_id = 100u64;
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            100,
            crate::RewardDestination::Staked
        ));

        // try to unbond using stash id, expect error NotController
        assert_noop!(
            DappsStaking::unbond(Origin::signed(stash_id), 100),
            crate::pallet::pallet::Error::<TestRuntime>::NotController
        );
    })
}

#[test]
fn unbond_too_many_chunks_is_not_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let stash_id = 10;
        let controller_id = 100u64;
        assert_ok!(DappsStaking::bond(
            Origin::signed(stash_id),
            controller_id,
            100,
            crate::RewardDestination::Staked
        ));

        // remove values up to MAX_UNLOCKING_CHUNKS and expect everything to work
        for chunk in 1..=MAX_UNLOCKING_CHUNKS {
            assert_ok!(DappsStaking::unbond(Origin::signed(controller_id), 1));
        }
        assert_noop!(
            DappsStaking::unbond(Origin::signed(controller_id), 1),
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

        // unbond some amount, the remainder bond should remain above existential deposit. Repeat twice to get two chunks.
        let first_unbond_amount = (bond_amount - 2 * EXISTENTIAL_DEPOSIT) / 2;
        for _ in 1..=2 {
            assert_ok!(DappsStaking::unbond(
                Origin::signed(controller_id),
                first_unbond_amount
            ));
        }

        // verify that withdraw works even if no chunks are available (era has not advanced enough)
        let current_era = <CurrentEra<TestRuntime>>::get().unwrap_or(Zero::zero());
        <CurrentEra<TestRuntime>>::put(current_era + UNBONDING_DURATION - 1);
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
        assert_ok!(DappsStaking::unbond(
            Origin::signed(controller_id),
            EXISTENTIAL_DEPOSIT + 1
        ));
        let current_era = <CurrentEra<TestRuntime>>::get().unwrap_or(Zero::zero());
        <CurrentEra<TestRuntime>>::put(current_era + UNBONDING_DURATION + 1);
        assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(
            controller_id
        )));
        System::assert_last_event(mock::Event::DappsStaking(crate::Event::Withdrawn(
            stash_id,
            2 * EXISTENTIAL_DEPOSIT,
        )));
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
    })
}
