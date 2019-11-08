use wasm_builder_runner::{build_current_project_with_rustflags, WasmBuilderSource};

fn main() {
	build_current_project_with_rustflags(
		"wasm_binary.rs",
		WasmBuilderSource::Git {
			repo: "https://github.com/satellitex/substrate.git",
			rev: "97643d4639868119550bd7ef6d824729cd6e587a",
		},
		// This instructs LLD to export __heap_base as a global variable, which is used by the
		// external memory allocator.
		"-Clink-arg=--export=__heap_base",
	);
}
