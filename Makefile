.PHONY: runtime-upgrade-test
runtime-upgrade-test:
	cargo build -p $(runtime)-runtime --release --locked
	cd tests/e2e && yarn --frozen-lockfile && yarn test:runtime-upgrade-$(runtime)

# use `cargo nextest run` if cargo-nextest is installed, fallback cargo test
cargo_test = $(shell which cargo-nextest >/dev/null && echo "cargo nextest run" || echo "cargo test")

.PHONY: test
test:
	SKIP_WASM_BUILD= ${cargo_test} --all

.PHONY: try-runtime
try-runtime:
	SKIP_WASM_BUILD= ${cargo_test} --all --features try-runtime

.PHONY: test-benchmarks
test-benchmarks:
	SKIP_WASM_BUILD= ${cargo_test} --all --features runtime-benchmarks


.PHONY: test-runtimes
test-runtimes:
	SKIP_WASM_BUILD= ${cargo_test} -p integration-tests --features=shibuya
	SKIP_WASM_BUILD= ${cargo_test} -p integration-tests --features=shiden
	SKIP_WASM_BUILD= ${cargo_test} -p integration-tests --features=astar

.PHONY: test-all
test-all: test test-runtimes try-runtime test-benchmarks
