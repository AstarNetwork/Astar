//! Tests for the plasm-lockdrop module.

#![cfg(test)]

use super::*;
use crate::mock::*;

use frame_support::unsigned::ValidateUnsigned;
use frame_support::{assert_noop, assert_ok};
use hex_literal::hex;
use plasm_primitives::AccountId;
use sp_core::{
    crypto::UncheckedInto,
    offchain::{
        testing::{TestOffchainExt, TestTransactionPoolExt},
        OffchainExt, TransactionPoolExt,
    },
    testing::KeyStore,
    traits::KeystoreExt,
    Pair,
};
use sp_runtime::DispatchError;

#[test]
fn lockdrop_authorities() {
    let alice: <Runtime as Trait>::AuthorityId =
        hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into();
    let bob: <Runtime as Trait>::AuthorityId =
        hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into();
    let charlie: <Runtime as Trait>::AuthorityId =
        hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into();
    let dave: <Runtime as Trait>::AuthorityId =
        hex!["1a0f0be3d6596d1dbc302243fe4e975ff20de67559100053024d8f5a7b435b2b"].unchecked_into();

    new_test_ext().execute_with(|| {
        assert_noop!(
            PlasmLockdrop::set_authorities(Origin::none(), vec![]),
            DispatchError::BadOrigin,
        );

        assert_eq!(PlasmLockdrop::authority_index_of(&alice), Some(0));
        assert_eq!(PlasmLockdrop::authority_index_of(&bob), Some(1));
        assert_eq!(PlasmLockdrop::authority_index_of(&charlie), Some(2));
        assert_eq!(PlasmLockdrop::authority_index_of(&dave), None);

        assert_ok!(PlasmLockdrop::set_authorities(
            Origin::root(),
            vec![bob.clone(), charlie.clone()],
        ));
        assert_eq!(<Keys<Runtime>>::get(), vec![bob.clone(), charlie.clone()]);
        assert_eq!(PlasmLockdrop::authority_index_of(&alice), None);
        assert_eq!(PlasmLockdrop::authority_index_of(&bob), Some(0));
        assert_eq!(PlasmLockdrop::authority_index_of(&charlie), Some(1));
        assert_eq!(PlasmLockdrop::authority_index_of(&dave), None);

        assert_ok!(PlasmLockdrop::set_authorities(
            Origin::root(),
            vec![alice.clone(), bob.clone()],
        ));
        assert_eq!(<Keys<Runtime>>::get(), vec![alice.clone(), bob.clone()]);
        assert_eq!(PlasmLockdrop::authority_index_of(&alice), Some(0));
        assert_eq!(PlasmLockdrop::authority_index_of(&bob), Some(1));
        assert_eq!(PlasmLockdrop::authority_index_of(&charlie), None);
        assert_eq!(PlasmLockdrop::authority_index_of(&dave), None);
    })
}

#[test]
fn oracle_unsinged_transaction() {
    let rate = TickerRate {
        btc: 10,
        eth: 12,
        authority: 0,
    };
    let vote = ClaimVote {
        claim_id: Default::default(),
        approve: false,
        authority: 0,
    };

    new_test_ext().execute_with(|| {
        // Invalid signature
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::set_dollar_rate(
            rate.clone(),
            Default::default(),
        ))
        .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "Transaction has a bad signature");

        // Invalid call
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::claim(Default::default()))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "InvalidTransaction custom error");

        let bad_account: AccountId = sp_keyring::sr25519::Keyring::Dave.into();
        let bad_pair =
            sr25519::AuthorityPair::from_string(&format!("//{}", bad_account), None).unwrap();

        let bad_rate = TickerRate {
            btc: 666,
            eth: 666,
            authority: 4,
        };
        let signature = bad_pair.sign(&bad_rate.encode());
        let dispatch =
            PlasmLockdrop::pre_dispatch(&crate::Call::set_dollar_rate(bad_rate, signature))
                .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "InvalidTransaction custom error");

        let lockdrop: Lockdrop = Default::default();
        assert_ok!(PlasmLockdrop::request(
            Origin::none(),
            lockdrop.clone(),
            Default::default(),
        ));
        let bad_vote = ClaimVote {
            claim_id: BlakeTwo256::hash_of(&lockdrop),
            authority: 4,
            ..vote
        };
        let signature = bad_pair.sign(&bad_vote.encode());
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::vote(bad_vote, signature))
            .map_err(|e| <&'static str>::from(e));
        assert_noop!(dispatch, "InvalidTransaction custom error");

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
        let valid_vote = ClaimVote {
            claim_id: BlakeTwo256::hash_of(&lockdrop),
            ..vote
        };
        let signature = pair.sign(&valid_vote.encode());
        let dispatch =
            PlasmLockdrop::pre_dispatch(&crate::Call::vote(valid_vote.clone(), signature.clone()));
        assert_ok!(dispatch);
        assert_ok!(PlasmLockdrop::vote(
            Origin::none(),
            valid_vote.clone(),
            signature.clone()
        ));

        // Double vote is not a problem, decision could be changed because of transaction
        // confirmation or other reasons.
        let dispatch = PlasmLockdrop::pre_dispatch(&crate::Call::vote(valid_vote, signature));
        assert_ok!(dispatch);
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
        let rate = TickerRate {
            btc: 10,
            eth: 12,
            authority: 0,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 12));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (0, 10, 12));

        let rate = TickerRate {
            btc: 9,
            eth: 11,
            authority: 1,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (9, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob.clone()), (0, 9, 11));

        let rate = TickerRate {
            btc: 15,
            eth: 3,
            authority: 2,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie.clone()), (0, 15, 3));

        Timestamp::set_timestamp(1);

        let rate = TickerRate {
            btc: 11,
            eth: 11,
            authority: 0,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (11, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(alice), (1, 11, 11));

        let rate = TickerRate {
            btc: 9,
            eth: 11,
            authority: 1,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (11, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob), (1, 9, 11));

        let rate = TickerRate {
            btc: 50,
            eth: 25,
            authority: 2,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
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
        let rate = TickerRate {
            btc: 10,
            eth: 12,
            authority: 0,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 12));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (0, 10, 12));

        let rate = TickerRate {
            btc: 9,
            eth: 11,
            authority: 1,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (9, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(bob.clone()), (0, 9, 11));

        let rate = TickerRate {
            btc: 15,
            eth: 3,
            authority: 2,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (10, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie.clone()), (0, 15, 3));

        Timestamp::set_timestamp(1);

        let rate = TickerRate {
            btc: 50,
            eth: 50,
            authority: 0,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (15, 11));
        assert_eq!(<DollarRateF<Runtime>>::get(alice.clone()), (1, 50, 50));

        Timestamp::set_timestamp(2);

        let rate = TickerRate {
            btc: 55,
            eth: 55,
            authority: 0,
        };
        assert_ok!(PlasmLockdrop::set_dollar_rate(
            Origin::none(),
            rate,
            Default::default()
        ));
        assert_eq!(<DollarRate<Runtime>>::get(), (55, 55));
        assert_eq!(<DollarRateF<Runtime>>::get(alice), (2, 55, 55));
        // Followed items should be ereased as expired
        assert_eq!(<DollarRateF<Runtime>>::get(bob), (0, 0, 0));
        assert_eq!(<DollarRateF<Runtime>>::get(charlie), (0, 0, 0));
    })
}

/*
#[test]
fn check_btc_issue_amount() {
    new_test_ext().execute_with(|| {
        assert_eq!(<DollarRate<Runtime>>::get(), (9_000, 200));
        assert_eq!(<Alpha>::get(), Perbill::from_parts(446_981_087));

        let day = 24 * 60 * 60;
        for i in 1..2000 {
            if i < 3 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 0);
                assert_eq!(PlasmLockdrop::btc_issue_amount(i as u128, i * day), 0);
            } else if i < 10 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 96552);
                assert_eq!(
                    PlasmLockdrop::btc_issue_amount(i as u128, i * day),
                    96552 * i as u128
                );
            } else if i < 30 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 402300);
                assert_eq!(
                    PlasmLockdrop::btc_issue_amount(i as u128, i * day),
                    402300 * i as u128
                );
            } else if i < 100 {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 1448280);
                assert_eq!(
                    PlasmLockdrop::btc_issue_amount(i as u128, i * day),
                    1448280 * i as u128
                );
            } else {
                assert_eq!(PlasmLockdrop::btc_issue_amount(1, i * day), 6436800);
                assert_eq!(
                    PlasmLockdrop::btc_issue_amount(i as u128, i * day),
                    6436800 * i as u128
                );
            }
        }
    })
}
*/

#[test]
fn check_eth_issue_amount() {
    new_test_ext().execute_with(|| {
        assert_eq!(<DollarRate<Runtime>>::get(), (9_000, 200));
        assert_eq!(<Alpha>::get(), Perbill::from_parts(446_981_087));

        let day = 24 * 60 * 60;
        for i in 1..2000 {
            if i < 3 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 0);
                assert_eq!(PlasmLockdrop::eth_issue_amount(i as u128, i * day), 0);
            } else if i < 10 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 2136);
                assert_eq!(
                    PlasmLockdrop::eth_issue_amount(i as u128, i * day),
                    2136 * i as u128
                );
            } else if i < 30 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 8900);
                assert_eq!(
                    PlasmLockdrop::eth_issue_amount(i as u128, i * day),
                    8900 * i as u128
                );
            } else if i < 100 {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 32040);
                assert_eq!(
                    PlasmLockdrop::eth_issue_amount(i as u128, i * day),
                    32040 * i as u128
                );
            } else {
                assert_eq!(PlasmLockdrop::eth_issue_amount(1, i * day), 142400);
                assert_eq!(
                    PlasmLockdrop::eth_issue_amount(i as u128, i * day),
                    142400 * i as u128
                );
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
        state
            .write()
            .expect_request(sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://127.0.0.1:34347/btc/ticker".into(),
                sent: true,
                response: Some("6766".into()),
                ..Default::default()
            });
        assert_eq!(BitcoinPrice::fetch(), Ok(6766));
        state
            .write()
            .expect_request(sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://127.0.0.1:34347/eth/ticker".into(),
                sent: true,
                response: Some("139".into()),
                ..Default::default()
            });
        assert_eq!(EthereumPrice::fetch(), Ok(139));
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

        state
            .write()
            .expect_request(sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://127.0.0.1:34347/btc/ticker".into(),
                sent: true,
                response: Some("6766".into()),
                ..Default::default()
            });
        state
            .write()
            .expect_request(sp_core::offchain::testing::PendingRequest {
                method: "GET".into(),
                uri: "http://127.0.0.1:34347/eth/ticker".into(),
                sent: true,
                response: Some("139".into()),
                ..Default::default()
            });

        let key = <Runtime as Trait>::AuthorityId::all()[0].clone();
        assert_ok!(PlasmLockdrop::send_dollar_rate(key));

        let transaction = pool_state.write().transactions.pop().unwrap();
        let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
        // Simple parameter checks
        match ex.call {
            crate::mock::Call::PlasmLockdrop(call) => {
                if let crate::Call::set_dollar_rate(rate, signature) = call.clone() {
                    assert_eq!(
                        rate,
                        TickerRate {
                            authority: 0,
                            btc: 6766,
                            eth: 139
                        }
                    );
                    assert!(Keys::<Runtime>::get()[rate.authority as usize]
                        .verify(&rate.encode(), &signature));
                }

                let dispatch =
                    PlasmLockdrop::pre_dispatch(&call).map_err(|e| <&'static str>::from(e));
                assert_ok!(dispatch);
            }
            e => panic!("Unexpected call: {:?}", e),
        }
    })
}

#[test]
fn simple_success_lockdrop_request() {
    new_test_ext().execute_with(|| {
        let lockdrop: Lockdrop = Default::default();
        let claim_id = BlakeTwo256::hash_of(&lockdrop);
        assert_ok!(PlasmLockdrop::request(
            Origin::none(),
            lockdrop,
            Default::default(),
        ));
        let vote = ClaimVote {
            claim_id,
            approve: true,
            authority: 0,
        };
        let vote2 = ClaimVote {
            claim_id,
            approve: true,
            authority: 1,
        };
        let vote3 = ClaimVote {
            claim_id,
            approve: true,
            authority: 2,
        };
        assert_ok!(PlasmLockdrop::vote(
            Origin::none(),
            vote,
            Default::default(),
        ));
        assert_noop!(
            PlasmLockdrop::claim(Origin::none(), claim_id),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("NotEnoughVotes")
            },
        );
        assert_ok!(PlasmLockdrop::vote(
            Origin::none(),
            vote2,
            Default::default(),
        ));
        assert_noop!(
            PlasmLockdrop::claim(Origin::none(), claim_id),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("NotEnoughVotes")
            },
        );
        assert_ok!(PlasmLockdrop::vote(
            Origin::none(),
            vote3,
            Default::default()
        ));
        assert_ok!(PlasmLockdrop::claim(Origin::none(), claim_id));
    })
}

#[test]
fn simple_fail_lockdrop_request() {
    new_test_ext().execute_with(|| {
        let lockdrop: Lockdrop = Default::default();
        let claim_id = BlakeTwo256::hash_of(&lockdrop);
        assert_ok!(PlasmLockdrop::request(
            Origin::none(),
            lockdrop,
            Default::default(),
        ));
        let vote = ClaimVote {
            claim_id,
            approve: false,
            authority: 0,
        };
        let vote2 = ClaimVote {
            claim_id,
            approve: false,
            authority: 1,
        };
        let vote3 = ClaimVote {
            claim_id,
            approve: false,
            authority: 2,
        };
        assert_ok!(PlasmLockdrop::vote(
            Origin::none(),
            vote,
            Default::default(),
        ));
        assert_noop!(
            PlasmLockdrop::claim(Origin::none(), claim_id),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("NotEnoughVotes")
            },
        );
        assert_ok!(PlasmLockdrop::vote(
            Origin::none(),
            vote2,
            Default::default(),
        ));
        assert_noop!(
            PlasmLockdrop::claim(Origin::none(), claim_id),
            DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("NotEnoughVotes")
            },
        );
        assert_ok!(PlasmLockdrop::vote(
            Origin::none(),
            vote3,
            Default::default()
        ));
        assert_noop!(
            PlasmLockdrop::claim(Origin::none(), claim_id),
            DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotApproved")
            },
        );
    })
}

#[test]
fn lockdrop_request_hash() {
    let transaction_hash =
        hex!["6c4364b2f5a847ffc69f787a0894191b75aa278a95020f02e4753c76119324e0"].into();
    let public_key = ecdsa::Public::from_raw(hex![
        "039360c9cbbede9ee771a55581d4a53cbcc4640953169549993a3b0e6ec7984061"
    ]);
    let params = Lockdrop::Ethereum {
        transaction_hash,
        public_key,
        duration: 2592000,
        value: 100000000000000000,
    };
    let claim_id = hex!["a94710e9db798a7d1e977b9f748ae802031eee2400a77600c526158892cd93d8"].into();
    assert_eq!(BlakeTwo256::hash_of(&params), claim_id);

    let transaction_hash =
        hex!["896d1cbe07c0207b714d87bcde04a535fec049a62c4e279dc2a6b71108afa523"].into();
    let public_key = ecdsa::Public::from_raw(hex![
        "039360c9cbbede9ee771a55581d4a53cbcc4640953169549993a3b0e6ec7984061"
    ]);
    let params = Lockdrop::Ethereum {
        transaction_hash,
        public_key,
        duration: 2592000,
        value: 100000000000000000,
    };
    let claim_id = hex!["0c4fb4f0dd7bdff67e5f8f7df5822d8cd8738f7e60a0a8d6151ef9abf51f8f4d"].into();
    assert_eq!(BlakeTwo256::hash_of(&params), claim_id);
}

#[test]
fn lockdrop_request_pow() {
    let nonce = hex!["30df083c7f59ea11a39bb341d37bd26d126d8522d408ebc2133bf7c7dc9d0c38"];
    let claim_id = hex!["a94710e9db798a7d1e977b9f748ae802031eee2400a77600c526158892cd93d8"];
    let pow_byte = BlakeTwo256::hash_of(&(claim_id, nonce)).as_bytes()[0];
    assert_eq!(pow_byte, 0);

    let nonce = hex!["6e64a5e25d145dd43b20c3ba4af5f4673ba88389f17afe2307ee4cb1a513978e"];
    let claim_id = hex!["0c4fb4f0dd7bdff67e5f8f7df5822d8cd8738f7e60a0a8d6151ef9abf51f8f4d"];
    let pow_byte = BlakeTwo256::hash_of(&(claim_id, nonce)).as_bytes()[0];
    assert_eq!(pow_byte, 0);
}

#[test]
fn eth_recover_works() {
    use sp_core::ecdsa;
    let msg = "test eth signed message";
    let pair = ecdsa::Pair::from_seed(&hex![
        "7e9c7ad85df5cdc88659f53e06fb2eb9bab3ebc59083a3190eaf2c730332529c"
    ]);
    let signature = hex!["dd0992d40e5cdf99db76bed162808508ac65acd7ae2fdc8573594f03ed9c939773e813181788fc02c3c68f3fdc592759b35f6354484343e18cb5317d34dab6c61b"];
    assert_eq!(eth_recover(&signature, msg.as_ref()), Some(pair.public()));

    let claim_id = H256::from(hex![
        "952e4e0f1f15f5b3828632320c0bcb4941c164974e08bea2e3996ae74c64cf58"
    ]);
    let account_id = AccountId::from(hex![
        "16eb796bee0c857db3d646ee7070252707aec0c7d82b2eda856632f6a2306a58"
    ]);
    let msg2 = PlasmLockdrop::claim_message(&claim_id, &account_id);
    let signature2 = hex!["80137f73ee7c3dc73f1219916dc58deb7ea2b9b00f39dbb01773038ec352efab5c4caf412b40a176a591db858cfa8717fe9f8b81cfd6d40905877c3e301320791b"];
    assert_eq!(eth_recover(&signature2, msg2.as_ref()), Some(pair.public()));
}
