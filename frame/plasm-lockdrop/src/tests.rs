//! Tests for the plasm-lockdrop module.

#![cfg(test)]

use super::*;
use crate::mock::*;

use hex_literal::hex;
use frame_support::assert_ok;
use sp_core::crypto::UncheckedInto;

#[test]
fn session_lockdrop_authorities() {
    let alice: <Runtime as Trait>::AuthorityId =
        hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into();
    let bob: <Runtime as Trait>::AuthorityId =
        hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into();
    let charlie: <Runtime as Trait>::AuthorityId =
        hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();
    let freddy: <Runtime as Trait>::AuthorityId =
        hex!["18da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();

    new_test_ext().execute_with(|| {
        assert_eq!(<Keys<Runtime>>::get(), vec![alice.clone(), bob.clone(), charlie.clone()]);
        assert_eq!(PlasmLockdrop::is_authority(&alice), true);
        assert_eq!(PlasmLockdrop::is_authority(&bob), true);
        assert_eq!(PlasmLockdrop::is_authority(&charlie), true);
        assert_eq!(PlasmLockdrop::is_authority(&freddy), false);
    })
}

#[test]
fn set_dollar_rate_should_work() {
    let alice: <Runtime as Trait>::AuthorityId =
        hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into();
    let bob: <Runtime as Trait>::AuthorityId =
        hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into();
    let charlie: <Runtime as Trait>::AuthorityId =
        hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();

    new_test_ext().execute_with(|| {
        let rate = TickerRate { btc: 10, eth: 12, sender: alice.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 12));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (0, 10, 12));

        let rate = TickerRate { btc: 9, eth: 11, sender: bob.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (9, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob.clone()), (0, 9, 11));

        let rate = TickerRate { btc: 15, eth: 3, sender: charlie.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie.clone()), (0, 15, 3));

        Timestamp::set_timestamp(1);

        let rate = TickerRate { btc: 11, eth: 11, sender: alice.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (11, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(alice), (1, 11, 11));

        let rate = TickerRate { btc: 9, eth: 11, sender: bob.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (11, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob), (1, 9, 11));

        let rate = TickerRate { btc: 50, eth: 25, sender: charlie.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (11, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie), (1, 50, 25));
    })
}

#[test]
fn dollar_rate_should_expire() {
    let alice: <Runtime as Trait>::AuthorityId =
        hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into();
    let bob: <Runtime as Trait>::AuthorityId =
        hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into();
    let charlie: <Runtime as Trait>::AuthorityId =
        hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();

    new_test_ext().execute_with(|| {
        let rate = TickerRate { btc: 10, eth: 12, sender: alice.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 12));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (0, 10, 12));

        let rate = TickerRate { btc: 9, eth: 11, sender: bob.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (9, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob.clone()), (0, 9, 11));

        let rate = TickerRate { btc: 15, eth: 3, sender: charlie.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie.clone()), (0, 15, 3));

        Timestamp::set_timestamp(1);

        let rate = TickerRate { btc: 50, eth: 50, sender: alice.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (15, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (1, 50, 50));

        Timestamp::set_timestamp(2);

        let rate = TickerRate { btc: 55, eth: 55, sender: alice.clone() };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (55, 55));
        assert_eq!(<DollarRateF<Runtime>>::get(alice), (2, 55, 55));
        // Followed items should be ereased as expired
        assert_eq!(<DollarRateF<Runtime>>::get(bob), (0, 0, 0));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie), (0, 0, 0));
    })
}

#[test]
fn check_btc_issue_amount() {
    new_test_ext().execute_with(|| {
        assert_eq!(<DollarRate<Runtime>>::get(), (5000, 120));
        assert_eq!(<Alpha>::get(), 2000);

        let day = 24 * 60 * 60;
        for i in 1..2000 {
            if i < 30 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 0);
                assert_eq!(PlasmLockdrop::btc_issue_amount(i as u128, i * day), 0);
            } else if i < 100 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 240000);
                assert_eq!(PlasmLockdrop::btc_issue_amount(i as u128, i * day), 240000 * i as u128);
            } else if i < 300 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 1000000);
                assert_eq!(PlasmLockdrop::btc_issue_amount(i as u128, i * day), 1000000 * i as u128);
            } else if i < 1000 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 3600000);
                assert_eq!(PlasmLockdrop::btc_issue_amount(i as u128, i * day), 3600000 * i as u128);
            } else {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 16000000);
                assert_eq!(PlasmLockdrop::btc_issue_amount(i as u128, i * day), 16000000 * i as u128);
            }
        }
    })
}

#[test]
fn check_eth_issue_amount() {
    new_test_ext().execute_with(|| {
        assert_eq!(<DollarRate<Runtime>>::get(), (5000, 120));
        assert_eq!(<Alpha>::get(), 2000);

        let day = 24 * 60 * 60;
        for i in 1..2000 {
            if i < 30 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 0);
                assert_eq!(PlasmLockdrop::eth_issue_amount(i as u128, i * day), 0);
            } else if i < 100 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 5760);
                assert_eq!(PlasmLockdrop::eth_issue_amount(i as u128, i * day), 5760 * i as u128);
            } else if i < 300 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 24000);
                assert_eq!(PlasmLockdrop::eth_issue_amount(i as u128, i * day), 24000 * i as u128);
            } else if i < 1000 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 86400);
                assert_eq!(PlasmLockdrop::eth_issue_amount(i as u128, i * day), 86400 * i as u128);
            } else {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 384000);
                assert_eq!(PlasmLockdrop::eth_issue_amount(i as u128, i * day), 384000 * i as u128);
            }
        }
    })
}
