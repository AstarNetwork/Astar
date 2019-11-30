SHELL := /bin/sh
CHILD_NODE_DIR := $(shell git rev-parse --show-toplevel)/child

.PHONY: docker-build
docker-build:
	docker run -it -v $(shell pwd):/opt stakedtechnologies/plasm-builder cargo build -p plasm-cli --target-dir target-debian --release
	docker build . -t stakedtechnologies/plasm-node

.PHONY: docker-push
docker-push:
	docker push stakedtechnologies/plasm-node

.PHONY: build-doc
build-doc:
	cd .. && cargo doc --all --all-features --no-deps --open
