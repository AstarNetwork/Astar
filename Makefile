rebuild:
	./build.sh
	cargo build
	./target/debug/plasm-node purge-chain --dev
	./target/debug/plasm-node --dev

build-push:
	docker run -it -v $(pwd):/opt stakedtechnologies/plasm-builder cargo build --target-out target-debian --release
	docker build . -t stakedtechnologies/plasm-node
	docker run -p 50052:9944 stakedtechnologies/plasm-node
