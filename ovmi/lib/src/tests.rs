use crate::executor::*;
use crate::mock::*;
use crate::predicates::*;
use crate::prepare::*;
use alloc::collections::btree_map::BTreeMap;
use codec::Encode;
use sp_core::{crypto::Pair, ecdsa::Pair as ECDSAPair, KeccakHasher};

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

fn make_decide_true_ex(
    inputs: Vec<Vec<u8>>,
    witness: Vec<Vec<u8>>,
) -> PredicateCallInputs<Address> {
    PredicateCallInputs::CompiledPredicate::<Address>(CompiledPredicateCallInputs::DecideTrue {
        inputs,
        witness,
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
        let res =
            MockExternalCall::bytes_to_bool(&ext.call_execute(&EQUAL_ADDRESS, input_data).unwrap())
                .unwrap();
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
        let res = MockExternalCall::bytes_to_bool(
            &ext.call_execute(&IS_LESS_ADDRESS, input_data).unwrap(),
        )
        .unwrap();
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
    let pair: ECDSAPair = ECDSAPair::from_seed(&[1; 32]);
    let miss_pair: ECDSAPair = ECDSAPair::from_seed(&[2; 32]);
    let address: Address = to_account(pair.public().as_ref());
    let miss_address: Address = to_account(miss_pair.public().as_ref());

    let address_bytes = address.encode();
    let key = hex::decode("0000000011112222").unwrap();
    let value = hex::decode("0000000011112222").unwrap();
    let false_value = hex::decode("0000000012345678").unwrap();

    ext.set_stored(&address, &key[..], &value[..]);

    // true case
    {
        let input_data = make_decide_true(vec![address_bytes.clone(), key.clone(), value.clone()]);
        let res = MockExternalCall::bytes_to_bool(
            &ext.call_execute(&IS_STORED_ADDRESS, input_data).unwrap(),
        )
        .unwrap();
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
        let input_data = make_decide_true(vec![miss_address.encode(), key.clone(), value]);
        let res = ext.call_execute(&IS_STORED_ADDRESS, input_data);
        assert_require!(res, "must decide true");
    }
}

#[test]
fn is_valid_signature_decide_true() {
    let ext = MockExternalCall::init();
    let pair: ECDSAPair = ECDSAPair::from_seed(&[1; 32]);
    let miss_pair: ECDSAPair = ECDSAPair::from_seed(&[2; 32]);
    let address: Address = to_account(pair.public().as_ref());
    let miss_address: Address = to_account(miss_pair.public().as_ref());

    let address_bytes = address.encode();
    let message = b"message".to_vec();
    let message_hash = KeccakHasher::hash(&message[..]);
    let signature: Vec<u8> = (pair.sign(&message_hash.0).as_ref() as &[u8]).into();
    let miss_signature: Vec<u8> = (miss_pair.sign(&message_hash.0).as_ref() as &[u8]).into();
    let verifier = b"secp256k1".to_vec();

    // Use check by frontend.
    println!("message_hash: {:?}", message_hash);
    println!("signature   : {:?}", message_hash);

    // true case
    {
        let input_data = make_decide_true(vec![
            message.clone(),
            signature.clone(),
            address_bytes.clone(),
            verifier.clone(),
        ]);
        let res = MockExternalCall::bytes_to_bool(
            &ext.call_execute(&IS_VALID_SIGNATURE_ADDRESS, input_data)
                .unwrap(),
        )
        .unwrap();
        assert!(res);
    }

    // false case (value)
    {
        let input_data = make_decide_true(vec![
            b"no_message".to_vec(),
            signature.clone(),
            address_bytes.clone(),
            verifier.clone(),
        ]);
        let res = ext.call_execute(&IS_VALID_SIGNATURE_ADDRESS, input_data);
        assert_require!(
            res,
            "_inputs[1] must be signature of _inputs[0] by _inputs[2]"
        );
    }

    // false case (address)
    {
        let input_data = make_decide_true(vec![
            message.clone(),
            signature.clone(),
            miss_address.encode(),
            verifier.clone(),
        ]);
        let res = ext.call_execute(&IS_VALID_SIGNATURE_ADDRESS, input_data);
        assert_require!(
            res,
            "_inputs[1] must be signature of _inputs[0] by _inputs[2]"
        );
    }

    // false case (address)
    {
        let input_data = make_decide_true(vec![
            message.clone(),
            miss_signature.clone(),
            address.encode(),
            verifier.clone(),
        ]);
        let res = ext.call_execute(&IS_VALID_SIGNATURE_ADDRESS, input_data);
        assert_require!(
            res,
            "_inputs[1] must be signature of _inputs[0] by _inputs[2]"
        );
    }
}

#[test]
fn ownership_predicate_true() {
    let mut ext = MockExternalCall::init();
    let ownership_predicate_str = load_predicate_json("ownership.json");
    let compiled_predicate = compile_from_json(ownership_predicate_str.as_str()).unwrap();
    let ownership_address = ext.deploy(
        compiled_predicate,
        PAY_OUT_CONTRACT_ADDRESS.clone(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    println!("ownership address: {:?}", ownership_address);

    let pair: ECDSAPair = ECDSAPair::from_seed(&[1; 32]);
    let wallet_address: Address = to_account(pair.public().as_ref());
    let label = b"LOwnershipT".encode();
    let transaction = hex!["00000000000000000000000000000000000000050000000000000000000000001080000000000000000000000000000000000000000100000000000000000000000080000000000000000000000000000000000a000000000000000000000000000000400a0000000000000000000000000000000901000000000000000000000000000000000000000200000000000000000000000004800000000000000000000000000000000000000003000000000000000000000000"].to_vec();
    let tx_hash = KeccakHasher::hash(&transaction[..]);
    let signature: Vec<u8> = (pair.sign(&tx_hash.0).as_ref() as &[u8]).into();
    let signature_dummy =
        hex!["2131311021311102131311021313110213131102131311021313110213131102"].to_vec();

    println!("transaction : {:?}", transaction);
    println!("signature   : {:?}", signature);
    println!("wallet      : {:?}", wallet_address);

    // success case(address)
    {
        let input_data = make_decide_true_ex(
            vec![label.clone(), wallet_address.encode(), transaction.clone()],
            vec![signature.clone()],
        );
        let res = ext.call_execute(&ownership_address, input_data);
        let ret: bool = MockExternalCall::bytes_to_bool(&res.unwrap()).unwrap();
        assert!(ret);
    }

    // false case (address)
    {
        let input_data = make_decide_true_ex(
            vec![label, wallet_address.encode(), transaction],
            vec![signature_dummy],
        );
        let res = ext.call_execute(&ownership_address, input_data);
        assert_require!(
            res,
            "_inputs[1] must be signature of _inputs[0] by _inputs[2]"
        );
    }
}

#[test]
fn verify_inclusion_decide_true() {
    // tested in plasma module.
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
