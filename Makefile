.PHONY: help
help: ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\n Usage: \033[36mmake \x1b[33m[target]\033[0m\n" } /^[a-zA-Z_-]+:.*?##/ { printf "  \x1b[33m%-20s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

.DEFAULT_GOAL := help

# use `cargo nextest run` if cargo-nextest is installed, fallback cargo test
cargo_test = $(shell which cargo-nextest >/dev/null && echo "cargo nextest run" || echo "cargo test")

.PHONY: test
test: ## Run unit tests
	${cargo_test} --workspace

.PHONY: test-features
test-features: ## Run features tests
	${cargo_test} --workspace --features try-runtime,runtime-benchmarks,evm-tracing

.PHONY: test-runtimes
test-runtimes: ## Run integration tests
	SKIP_WASM_BUILD= ${cargo_test} -p integration-tests --features=shibuya
	SKIP_WASM_BUILD= ${cargo_test} -p integration-tests --features=shiden
	SKIP_WASM_BUILD= ${cargo_test} -p integration-tests --features=astar

.PHONY: test-all
test-all: ## Run all tests
	$(MAKE) test
	$(MAKE) test-runtimes
	$(MAKE) test-features

.PHONY: runtime-upgrade-test
runtime-upgrade-test: ## Runtime upgrade test. e.g. make runtime-upgrade-test runtime=astar
	cargo build -p $(runtime)-runtime --release --locked
	cd tests/e2e && yarn --frozen-lockfile && yarn test:runtime-upgrade-$(runtime)

.PHONY: build
build: ## Build release profile
	cargo build --release --locked
