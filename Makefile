.PHONY: runtime-upgrade-test
runtime-upgrade-test:
	cargo build -p $(runtime)-runtime --release --locked
	cd tests/e2e && yarn --frozen-lockfile && yarn test:runtime-upgrade-$(runtime)
