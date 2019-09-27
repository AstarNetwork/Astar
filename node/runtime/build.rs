use wasm_builder_runner::{build_current_project_with_rustflags, WasmBuilderSource};

fn main() {
	build_current_project_with_rustflags(
		"wasm_binary.rs",
		WasmBuilderSource::Git {
			repo: "https://github.com/satellitex/substrate.git",
			rev: "d2e7d660f8dbbb8f9753dfd231cef1c9b502c41c",
		},
		// This instructs LLD to export __heap_base as a global variable, which is used by the
		// external memory allocator.
		"-Clink-arg=--export=__heap_base",
	);
}
