//! Tests for the plasm-lockdrop module.

#![cfg(test)]

use super::*;
use crate::mock::*;

use hex_literal::hex;
use plasm_primitives::AccountId;
use frame_support::unsigned::ValidateUnsigned;
use frame_support::{assert_ok, assert_noop};
use sp_core::{
    Pair, crypto::UncheckedInto,
    testing::KeyStore,
    traits::KeystoreExt,
    offchain::{
        OffchainExt,
        TransactionPoolExt,
        testing::{
            TestOffchainExt,
            TestTransactionPoolExt,
        },
    },
};

#[test]
fn session_lockdrop_authorities() {
    let alice: <Runtime as Trait>::AuthorityId =
        hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into();
    let bob: <Runtime as Trait>::AuthorityId =
        hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into();
    let charlie: <Runtime as Trait>::AuthorityId =
        hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();
    let dave: <Runtime as Trait>::AuthorityId =
        hex!["1a0f0be3d6596d1dbc302243fe4e975ff20de67559100053024d8f5a7b435b2b"].unchecked_into();

    new_test_ext().execute_with(|| {
        assert_eq!(<Keys<Runtime>>::get(), vec![alice.clone(), bob.clone(), charlie.clone()]);
        assert_eq!(PlasmLockdrop::authority_index_of(&alice), Some(0));
        assert_eq!(PlasmLockdrop::authority_index_of(&bob), Some(1));
        assert_eq!(PlasmLockdrop::authority_index_of(&charlie), Some(2));
        assert_eq!(PlasmLockdrop::authority_index_of(&dave), None);

        VALIDATORS.with(|l| *l.borrow_mut() = Some(vec![
            sp_keyring::sr25519::Keyring::Bob.into(),
            sp_keyring::sr25519::Keyring::Charlie.into(),
        ]));
        advance_session();
        advance_session();

        assert_eq!(<Keys<Runtime>>::get(), vec![bob.clone(), charlie.clone()]);
        assert_eq!(PlasmLockdrop::authority_index_of(&alice), None);
        assert_eq!(PlasmLockdrop::authority_index_of(&bob), Some(0));
        assert_eq!(PlasmLockdrop::authority_index_of(&charlie), Some(1));
        assert_eq!(PlasmLockdrop::authority_index_of(&dave), None);

        VALIDATORS.with(|l| *l.borrow_mut() = Some(vec![
            sp_keyring::sr25519::Keyring::Alice.into(),
            sp_keyring::sr25519::Keyring::Bob.into(),
        ]));
        advance_session();
        advance_session();

        assert_eq!(<Keys<Runtime>>::get(), vec![alice.clone(), bob.clone()]);
        assert_eq!(PlasmLockdrop::authority_index_of(&alice), Some(0));
        assert_eq!(PlasmLockdrop::authority_index_of(&bob), Some(1));
        assert_eq!(PlasmLockdrop::authority_index_of(&charlie), None);
        assert_eq!(PlasmLockdrop::authority_index_of(&dave), None);
    })
}

#[test]
fn oracle_unsinged_transaction() {
    let rate = TickerRate { btc: 10, eth: 12, authority: 0 };
    let vote = ClaimVote { claim_id: Default::default(), approve: false, authority: 0 };

    new_test_ext().execute_with(|| {
        // Invalid signature
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::set_dollar_rate(rate.clone(), Default::default()))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "Transaction has a bad signature");

        // Invalid call
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::claim(Default::default()))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "Transaction call is not expected");

        let bad_account: AccountId = sp_keyring::sr25519::Keyring::Dave.into();
        let bad_pair = sr25519::AuthorityPair::from_string(&format!("//{}", bad_account), None).unwrap(); 

        let bad_rate = TickerRate { btc: 666, eth: 666, authority: 4 };
        let signature = bad_pair.sign(&bad_rate.encode());
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::set_dollar_rate(bad_rate, signature))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "Transaction has a bad signature");

        let lockdrop: Lockdrop = Default::default();
        assert_ok!(PlasmLockdrop::request(Origin::signed(bad_account), lockdrop.clone()));
        let bad_vote = ClaimVote {
            claim_id: BlakeTwo256::hash_of(&lockdrop),
            authority: 4,
            ..vote
        };
        let signature = bad_pair.sign(&bad_vote.encode());
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::vote(bad_vote, signature))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "Transaction has a bad signature");

        let account: AccountId = sp_keyring::sr25519::Keyring::Alice.into();
        let pair = sr25519::AuthorityPair::from_string(&format!("//{}", account), None).unwrap(); 

        // Invalid parameter
        let signature = pair.sign(&vote.encode());
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::vote(vote.clone(), signature))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "Transaction call is not expected");

        // Valid signature & params
        let signature = pair.sign(&rate.encode());
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::set_dollar_rate(rate, signature));
        assert_ok!(dispatch);

        // Valid signature & params
        let valid_vote = ClaimVote { claim_id: BlakeTwo256::hash_of(&lockdrop), ..vote };
        let signature = pair.sign(&valid_vote.encode());
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::vote(valid_vote.clone(), signature.clone()));
        assert_ok!(dispatch);
        assert_ok!(PlasmLockdrop::vote(Origin::NONE, valid_vote.clone(), signature.clone()));

        // Double vote
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::vote(valid_vote, signature))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "Transaction call is not expected");
    });
}

#[test]
fn dollar_rate_median_filter() {
    let alice: <Runtime as Trait>::AuthorityId =
        hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into();
    let bob: <Runtime as Trait>::AuthorityId =
        hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into();
    let charlie: <Runtime as Trait>::AuthorityId =
        hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();

    new_test_ext().execute_with(|| {
        let rate = TickerRate { btc: 10, eth: 12, authority: 0 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 12));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (0, 10, 12));

        let rate = TickerRate { btc: 9, eth: 11, authority: 1 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (9, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob.clone()), (0, 9, 11));

        let rate = TickerRate { btc: 15, eth: 3, authority: 2 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie.clone()), (0, 15, 3));

        Timestamp::set_timestamp(1);

        let rate = TickerRate { btc: 11, eth: 11, authority: 0 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (11, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(alice), (1, 11, 11));

        let rate = TickerRate { btc: 9, eth: 11, authority: 1 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (11, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob), (1, 9, 11));

        let rate = TickerRate { btc: 50, eth: 25, authority: 2 };
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
        let rate = TickerRate { btc: 10, eth: 12, authority: 0 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 12));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (0, 10, 12));

        let rate = TickerRate { btc: 9, eth: 11, authority: 1 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (9, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob.clone()), (0, 9, 11));

        let rate = TickerRate { btc: 15, eth: 3, authority: 2 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie.clone()), (0, 15, 3));

        Timestamp::set_timestamp(1);

        let rate = TickerRate { btc: 50, eth: 50, authority: 0 };
        assert_ok!(PlasmLockdrop::set_dollar_rate(Origin::NONE, rate, Default::default()));
        assert_eq!(<DollarRate<Runtime>>::get(), (15, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (1, 50, 50));

        Timestamp::set_timestamp(2);

        let rate = TickerRate { btc: 55, eth: 55, authority: 0 };
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
        assert_eq!(<Alpha>::get(), Perbill::from_parts(200_000_000));

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
        assert_eq!(<Alpha>::get(), Perbill::from_parts(200_000_000));

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

#[test]
fn dollar_rate_ticker_works() {
    let mut ext = new_test_ext();
    let (offchain, state) = TestOffchainExt::new();
    ext.register_extension(OffchainExt::new(offchain));

    ext.execute_with(|| {
        state.write().expect_request(
            0,
            sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://api.coingecko.com/api/v3/coins/bitcoin".into(),
                sent: true,
                response: Some(COINGECKO_BTC_TICKER.into()),
                ..Default::default()
            },
        );
        assert_eq!(<Runtime as Trait>::BitcoinTicker::fetch(), Ok(6766));
        state.write().expect_request(
            0,
            sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://api.coingecko.com/api/v3/coins/ethereum".into(),
                sent: true,
                response: Some(COINGECKO_ETH_TICKER.into()),
                ..Default::default()
            },
        );
        assert_eq!(<Runtime as Trait>::EthereumTicker::fetch(), Ok(139));
    })
}

#[test]
fn dollar_rate_offchain_worker() {
    let mut ext = new_test_ext();
    let (offchain, state) = TestOffchainExt::new();
    let (pool, pool_state) = TestTransactionPoolExt::new();
    ext.register_extension(KeystoreExt(KeyStore::new()));
    ext.register_extension(OffchainExt::new(offchain));
    ext.register_extension(TransactionPoolExt::new(pool));

    let account: AccountId = sp_keyring::sr25519::Keyring::Alice.into();
    ext.execute_with(|| {
        let seed = format!("//{}", account).as_bytes().to_vec();
        <Runtime as Trait>::AuthorityId::generate_pair(Some(seed)); 

        state.write().expect_request(
            0,
            sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://api.coingecko.com/api/v3/coins/bitcoin".into(),
                sent: true,
                response: Some(COINGECKO_BTC_TICKER.into()),
                ..Default::default()
            },
        );
        let btc = <Runtime as Trait>::BitcoinTicker::fetch().unwrap();

        state.write().expect_request(
            0,
            sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://api.coingecko.com/api/v3/coins/ethereum".into(),
                sent: true,
                response: Some(COINGECKO_ETH_TICKER.into()),
                ..Default::default()
            },
        );
        let eth = <Runtime as Trait>::EthereumTicker::fetch().unwrap();

        assert_ok!(PlasmLockdrop::send_dollar_rate(btc, eth));

        let transaction = pool_state.write().transactions.pop().unwrap();
        let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
        // Simple parameter checks
        match ex.call {
            crate::mock::Call::PlasmLockdrop(call) => {
                if let crate::Call::set_dollar_rate(rate, signature) = call.clone() {
                    assert_eq!(rate, TickerRate { authority: 0, btc: 6766, eth: 139 });
                    assert!(Keys::<Runtime>::get()[rate.authority as usize].verify(&rate.encode(), &signature));
                }

                let dispatch = PlasmLockdrop::pre_dispatch(&call).map_err(|e| <&'static str>::from(e));
                assert_ok!(dispatch);
            },
            e => panic!("Unexpected call: {:?}", e),
        }
    })
}

#[test]
fn simple_success_lockdrop_request() {
    new_test_ext().execute_with(|| {
        let lockdrop: Lockdrop = Default::default();
        let claim_id = BlakeTwo256::hash_of(&lockdrop);
        assert_ok!(PlasmLockdrop::request(Origin::signed(Default::default()), lockdrop));
        let vote = ClaimVote { claim_id, approve: true, authority: 0 };
        assert_ok!(PlasmLockdrop::vote(Origin::NONE, vote.clone(), Default::default()));
        assert_noop!(
            PlasmLockdrop::claim(Origin::signed(Default::default()), claim_id),
            "this request don't get enough authority votes"
        );
        assert_ok!(PlasmLockdrop::vote(Origin::NONE, vote.clone(), Default::default()));
        assert_noop!(
            PlasmLockdrop::claim(Origin::signed(Default::default()), claim_id),
            "this request don't get enough authority votes"
        );
        assert_ok!(PlasmLockdrop::vote(Origin::NONE, vote, Default::default()));
        assert_ok!(PlasmLockdrop::claim(Origin::signed(Default::default()), claim_id));
    })
}

#[test]
fn simple_fail_lockdrop_request() {
    new_test_ext().execute_with(|| {
        let lockdrop: Lockdrop = Default::default();
        let claim_id = BlakeTwo256::hash_of(&lockdrop);
        assert_ok!(PlasmLockdrop::request(Origin::signed(Default::default()), lockdrop));
        let vote = ClaimVote { claim_id, approve: false, authority: 0 };
        assert_ok!(PlasmLockdrop::vote(Origin::NONE, vote.clone(), Default::default()));
        assert_noop!(
            PlasmLockdrop::claim(Origin::signed(Default::default()), claim_id),
            "this request don't get enough authority votes"
        );
        assert_ok!(PlasmLockdrop::vote(Origin::NONE, vote.clone(), Default::default()));
        assert_noop!(
            PlasmLockdrop::claim(Origin::signed(Default::default()), claim_id),
            "this request don't get enough authority votes"
        );
        assert_ok!(PlasmLockdrop::vote(Origin::NONE, vote, Default::default()));
        assert_noop!(
            PlasmLockdrop::claim(Origin::signed(Default::default()), claim_id),
            "this request don't approved by authorities"
        );
    })
}
