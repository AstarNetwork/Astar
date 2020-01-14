SHELL := /bin/sh
CHILD_NODE_DIR := $(shell git rev-parse --show-toplevel)/child

.PHONY: docker-build
docker-build:
	docker run -it -v $(shell pwd)/../..:/opt staketechnologies/plasm-builder cargo build --target-dir third-party/docker/target-debian --release
	docker build . -t staketechnologies/plasm-node

.PHONY: docker-push
docker-push:
	docker push staketechnologies/plasm-node

.PHONY: build-doc
build-doc:
	cd ../.. && cargo doc --all --all-features --no-deps --open
