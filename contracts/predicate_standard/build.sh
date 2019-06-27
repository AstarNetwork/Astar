#!/bin/bash
set -e

PROJNAME=predicate_standard

# cargo clean
# rm Cargo.lock

CARGO_INCREMENTAL=0 &&
# Without --features generate-api-description, because passed compile my struct using argments of contract method.
cargo build --release --target=wasm32-unknown-unknown --verbose
wasm2wat -o target/$PROJNAME.wat target/wasm32-unknown-unknown/release/$PROJNAME.wasm
cat target/$PROJNAME.wat | sed "s/(import \"env\" \"memory\" (memory (;0;) 2))/(import \"env\" \"memory\" (memory (;0;) 2 16))/" > target/$PROJNAME-fixed.wat
wat2wasm -o target/$PROJNAME.wasm target/$PROJNAME-fixed.wat
wasm-opt -Oz target/$PROJNAME.wasm -o target/$PROJNAME-opt.wasm
wasm-prune --exports call,deploy target/$PROJNAME-opt.wasm target/$PROJNAME-pruned.wasm
