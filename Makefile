SHELL := /bin/bash

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
	cd child &&\
	 docker run -it -v $(shell pwd):/opt stakedtechnologies/plasm-builder cargo build --target-dir target-debian --release &&\
	 docker build child -t stakedtechnologies/plasm-child-node

.PHONY: push-child
push-child:
	cd child && docker push stakedtechnologies/plasm-child-node
