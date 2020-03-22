//! Tests for the plasm-lockdrop module.

#![cfg(test)]

use super::*;
use crate::mock::*;

use hex_literal::hex;
use sp_core::crypto::UncheckedInto;

#[test]
fn session_set_lockdrop_authorities() {
    new_test_ext().execute_with(|| {
        assert_eq!(<Keys<Runtime>>::get(), vec![
            hex!["c83f0a4067f1b166132ed45995eee17ba7aeafeea27fe17550728ee34f998c4e"].unchecked_into(),
            hex!["fa1b7e37aa3e463c81215f63f65a7c2b36ced251dd6f1511d357047672afa422"].unchecked_into(),
            hex!["88da12401449623ab60f20ed4302ab6e5db53de1e7b5271f35c858ab8b5ab37f"].unchecked_into(),
        ]);
    })
}
