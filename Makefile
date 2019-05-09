rebuild:
	./build.sh
	cargo build
	./target/debug/plasm-node purge-chain --dev
	./target/debug/plasm-node --dev
