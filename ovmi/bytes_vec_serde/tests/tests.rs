use bytes_vec_serde_derive::BytesVecSerde;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(BytesVecSerde, Debug)]
struct TestStruct {
    pub name: Vec<Vec<u8>>,
    pub value: u128,
}

#[test]
fn struct_true() {
    let test_struct = TestStruct {
        name: vec![vec![1, 2, 3]],
        value: 100,
    };
    let test_struct_serde = TestStructSerializable {
        name: vec!["aa".to_string()],
        value: 100,
    };
    println!("{:?}", test_struct);
    println!("{:?}", test_struct_serde);
    assert!(false);
}
