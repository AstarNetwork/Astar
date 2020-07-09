use ovmi;

fn main() {
    let compiled_predicate = match ovmi::compile_from_json("{}") {
        Ok(res) => res,
        Err(err) => panic!(err),
    };
    println!("{:?}", compiled_predicate);
}
