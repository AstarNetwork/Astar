.PHONY: runtime-upgrade-test
runtime-upgrade-test:
	cargo build -p $(runtime)-runtime --release --locked
	cd tests/e2e && yarn --frozen-lockfile && yarn test:runtime-upgrade-$(runtime)

.PHONY: test-runtimes
test-runtimes:
	SKIP_WASM_BUILD= cargo test -p integration-tests --features=shibuya
	SKIP_WASM_BUILD= cargo test -p integration-tests --features=shiden
	SKIP_WASM_BUILD= cargo test -p integration-tests --features=astar
