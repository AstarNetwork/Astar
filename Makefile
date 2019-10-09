SHELL := /bin/bash
CHILD_NODE_DIR := $(shell git rev-parse --show-toplevel)/child

.PHONY: build-debian
build-debian:
	docker run -it -v $(shell pwd):/opt stakedtechnologies/plasm-builder cargo build --target-dir target-debian --release
	docker build . -t stakedtechnologies/plasm-node

.PHONY: push-parent
push-parent:
	docker push stakedtechnologies/plasm-node

.PHONY: build-doc
build-doc:
	cargo doc --all --all-features --no-deps --open
