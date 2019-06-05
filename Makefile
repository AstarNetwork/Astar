SHELL := /bin/bash
CHILD_NODE_DIR := $(shell git rev-parse --show-toplevel)/child

.PHONY: rebuild
rebuild:
	./build.sh
	cargo build
	./target/debug/plasm-node purge-chain --dev
	./target/debug/plasm-node --dev

.PHONY: build-child-wasm
build-child-wasm:
	cd child && ./build.sh

.PHONY: build-parent-debian
build-parent-debian:
	docker run -it -v $(shell pwd):/opt stakedtechnologies/plasm-builder cargo build --target-dir target-debian --release
	docker build . -t stakedtechnologies/plasm-node

.PHONY: push-parent
push-parent:
	docker push stakedtechnologies/plasm-node

.PHONY: build-child-debian
build-child-debian:
	 docker run -it -v $(shell pwd):/opt  -w="/opt/child" stakedtechnologies/plasm-builder cargo build --target-dir $(CHILD_NODE_DIR)/target-debian --release
	 docker build $(CHILD_NODE_DIR) -t stakedtechnologies/plasm-child-node

.PHONY: push-child
push-child:
	docker push stakedtechnologies/plasm-child-node
