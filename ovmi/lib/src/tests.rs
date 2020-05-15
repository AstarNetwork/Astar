use crate::executor::*;
use crate::mock::*;
use crate::predicates::*;
use crate::*;
use codec::Encode;
use std::str::FromStr;

macro_rules! assert_require {
    ($res:expr, $msg:expr) => {
        assert_eq!(
            $res.expect_err("Exepcted Error."),
            ExecError::<Address>::Require { msg: $msg }
        )
    };
}

fn make_decide_true(inputs: Vec<Vec<u8>>) -> PredicateCallInputs<Address> {
    PredicateCallInputs::BaseAtomicPredicate::<Address>(BaseAtomicPredicateCallInputs::DecideTrue {
        inputs,
    })
}

#[test]
fn equal_predicate_decide_true() {
    let ext = MockExternalCall::init();
    let input0 = hex::decode("0000000011112222").unwrap();
    let input1 = hex::decode("0000000012345678").unwrap();
    // true case
    {
        let input_data = make_decide_true(vec![input0.clone(), input0.clone()]);
        let res = ext.call_execute(&EQUAL_ADDRESS, input_data).unwrap();
        assert!(res);
    }

    // false case
    {
        let input_data = make_decide_true(vec![input0.clone(), input1.clone()]);
        let res = ext.call_execute(&EQUAL_ADDRESS, input_data);
        assert_require!(res, "2 inputs must be equal");
    }
}

#[test]
fn is_less_than_predicate_decide_true() {
    let ext = MockExternalCall::init();
    let input0: Vec<u8> = (0 as u128).encode();
    let input1: Vec<u8> = (1 as u128).encode();

    // true case
    {
        let input_data = make_decide_true(vec![input0.clone(), input1.clone()]);
        let res = ext.call_execute(&IS_LESS_ADDRESS, input_data).unwrap();
        assert!(res);
    }

    // false case
    {
        let input_data = make_decide_true(vec![input1.clone(), input0.clone()]);
        let res = ext.call_execute(&IS_LESS_ADDRESS, input_data);
        assert_require!(res, "first input is not less than second input");
    }
}

#[test]
fn is_stored_decide_true() {
    let mut ext = MockExternalCall::init();
    let address: Address = 88;
    let address_bytes = address.encode();
    let key = hex::decode("0000000011112222").unwrap();
    let value = hex::decode("0000000011112222").unwrap();
    let false_value = hex::decode("0000000012345678").unwrap();

    ext.set_stored(&address, &key[..], &value[..]);

    // true case
    {
        let input_data = make_decide_true(vec![address_bytes.clone(), key.clone(), value.clone()]);
        let res = ext.call_execute(&IS_STORED_ADDRESS, input_data).unwrap();
        assert!(res);
    }

    // false case (value)
    {
        let input_data = make_decide_true(vec![address_bytes.clone(), key.clone(), false_value]);
        let res = ext.call_execute(&IS_STORED_ADDRESS, input_data);
        assert_require!(res, "must decide true");
    }

    // false case (address)
    {
        let input_data = make_decide_true(vec![(21 as u128).encode(), key.clone(), value]);
        let res = ext.call_execute(&IS_STORED_ADDRESS, input_data);
        assert_require!(res, "must decide true");
    }
}

// TODO: add signature func
#[test]
fn is_valid_signature_decide_true() {
    // let mut ext = MockExternalCall::init();
    // let address: Address = 88;
    // let signature = hex::decode("3050ed8255d5599ebce4be5ef23eceeb61bfae924db5e5b12fc975663854629204a68351940fcea4231e9e4af515e2a10c187fcd7f88f4e5ffed218c76a5553b1b").unwrap();
    // let invalid_signature = hex::decode("00b0ed8255d5599ebce4be5ef23eceeb16bfae924db5e5b12fc975663854629204a68351940fcea4231e9e4af515e2a10c187fcd7f88f4e5ffed218c76a1113bb2").unwrap();
    // let message = b"message".to_vec();
    // let verifier_type = BlakeTwo256::hash(&b"secp256k1".to_vec()[..]);

    // true case
    // {
    //     let input_data = make_decide_true(vec![
    //         message.clone(),
    //         signature.clone(),
    //         address.to_vec(),
    //         verifier_type.clone(),
    //     ]);
    //     let res = ext
    //         .call_execute(&IS_VALID_SIGNATURE_ADDRESS, input_data)
    //         .unwrap();
    //     assert!(res);
    // }

    // // false case (value)
    // {
    //     let input_data = make_decide_true(vec![address_bytes.clone(), key.clone(), false_value]);
    //     let res = ext.call_execute(&IS_VALID_SIGNATURE_ADDRESS, input_data);
    //     assert_require!(res, "must decide true");
    // }
    //
    // // false case (address)
    // {
    //     let input_data = make_decide_true(vec![(21 as u128).encode(), key.clone(), value]);
    //     let res = ext.call_execute(&IS_VALID_SIGNATURE_ADDRESS, input_data);
    //     assert_require!(res, "must decide true");
    // }
}

#[test]
fn verify_inclusion_decide_true() {
    // let mut ext = MockExternalCall::init();
    // let address_1: Address = 88;
    // let address_2: Address = 99;
    //
    // let leaf = b"leaf".to_vec();
    // let token = Address::default().encode();
    // let range = Range {
    //     start: 100,
    //     end: 200,
    // }
    // .encode();
    // let inclusion_proof: Vec<(
    //     (Address, u128, Vec<(Hash, Address)>),
    //     (u128, u128, Vec<(Hash, u128)>),
    // )> = vec![(
    //     (
    //         address_1,
    //         0,
    //         vec![(
    //             Hash::from_str("dd779be20b84ced84b7cbbdc8dc98d901ecd198642313d35d32775d75d916d3a")
    //                 .unwrap(),
    //             address_2,
    //         )],
    //     ),
    //     (
    //         0,
    //         0,
    //         vec![
    //             (
    //                 Hash::from_str(
    //                     "036491cc10808eeb0ff717314df6f19ba2e232d04d5f039f6fa382cae41641da",
    //                 )
    //                 .unwrap(),
    //                 7,
    //             ),
    //             (
    //                 Hash::from_str(
    //                     "ef583c07cae62e3a002a9ad558064ae80db17162801132f9327e8bb6da16ea8a",
    //                 )
    //                 .unwrap(),
    //                 5000,
    //             ),
    //         ],
    //     ),
    // )];
    // let inclusion_proof_bytes = inclusion_proof.encode();
    // let root = Hash::from_str("ef583c07cae62e3a002a9ad558064ae80db0000000000000000000b6da16ea8a")
    //     .unwrap()
    //     .encode();
    //
    // // true case
    // {
    //     let input_data = make_decide_true(vec![
    //         leaf.clone(),
    //         token.clone(),
    //         range.clone(),
    //         inclusion_proof_bytes.clone(),
    //         root.clone(),
    //     ]);
    //     let res = ext
    //         .call_execute(&VERIFY_INCLUAION_ADDRESS, input_data)
    //         .unwrap();
    //     assert!(res);
    // }
}
