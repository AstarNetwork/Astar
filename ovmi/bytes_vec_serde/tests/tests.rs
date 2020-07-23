#[macro_use]
extern crate bytes_vec_serde_derive;
use bytes_vec_serde_derive::BytesVecSerde;

#[derive(BytesVecSerde)]
struct TestStruct {
    name: Vec<u8>,
    value: u128,
}

#[test]
fn struct_true() {
    // let test_struct = TestStruct {
    //     name: vec![1, 2, 3],
    //     value: 100,
    // };
    assert_eq!(
        TestStruct::hello_macro(),
        "Hello, Macro! My name is TestStruct"
    );
}
