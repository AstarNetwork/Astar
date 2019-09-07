use wasm_builder_runner::{build_current_project, WasmBuilderSource};

fn main() {
	build_current_project("wasm_binary.rs", WasmBuilderSource::Crates("1.0.5"));
}
