use codec::{Decode, Encode};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
useplasm_executor::Executor;
useplasm_primitives::{BlockNumber, Hash};
useplasm_runtime::{
	Block, BuildStorage, Call, CheckedExtrinsic, GenesisConfig, Header, UncheckedExtrinsic,
};
useplasm_runtime::constants::currency::*;
useplasm_testing::keyring::*;
use primitives::{Blake2Hasher, NativeOrEncoded, NeverNativeValue};
use primitives::storage::well_known_keys;
use primitives::traits::CodeExecutor;
use runtime_support::Hashable;
use state_machine::TestExternalities as CoreTestExternalities;
use substrate_executor::{NativeExecutor, RuntimeInfo, WasmExecutionMethod, Externalities};

criterion_group!(benches, bench_execute_block);
criterion_main!(benches);

/// The wasm runtime code.
const COMPACT_CODE: &[u8] =plasm_runtime::WASM_BINARY;

const GENESIS_HASH: [u8; 32] = [69u8; 32];

const VERSION: u32 =plasm_runtime::VERSION.spec_version;

const HEAP_PAGES: u64 = 20;

type TestExternalities<H> = CoreTestExternalities<H, u64>;

#[derive(Debug)]
enum ExecutionMethod {
	Native,
	Wasm(WasmExecutionMethod),
}

fn sign(xt: CheckedExtrinsic) -> UncheckedExtrinsic {
	node_testing::keyring::sign(xt, VERSION, GENESIS_HASH)
}

fn new_test_ext(genesis_config: &GenesisConfig) -> TestExternalities<Blake2Hasher> {
	let mut test_ext = TestExternalities::new_with_code(
		COMPACT_CODE,
		genesis_config.build_storage().unwrap(),
	);
	test_ext.ext().place_storage(well_known_keys::HEAP_PAGES.to_vec(), Some(HEAP_PAGES.encode()));
	test_ext
}

fn construct_block<E: Externalities>(
	executor: &NativeExecutor<Executor>,
	ext: &mut E,
	number: BlockNumber,
	parent_hash: Hash,
	extrinsics: Vec<CheckedExtrinsic>,
) -> (Vec<u8>, Hash) {
	use trie::{TrieConfiguration, trie_types::Layout};

	// sign extrinsics.
	let extrinsics = extrinsics.into_iter().map(sign).collect::<Vec<_>>();

	// calculate the header fields that we can.
	let extrinsics_root = Layout::<Blake2Hasher>::ordered_trie_root(
		extrinsics.iter().map(Encode::encode)
	).to_fixed_bytes()
		.into();

	let header = Header {
		parent_hash,
		number,
		extrinsics_root,
		state_root: Default::default(),
		digest: Default::default(),
	};

	// execute the block to get the real header.
	executor.call::<_, NeverNativeValue, fn() -> _>(
		ext,
		"Core_initialize_block",
		&header.encode(),
		true,
		None,
	).0.unwrap();

	for i in extrinsics.iter() {
		executor.call::<_, NeverNativeValue, fn() -> _>(
			ext,
			"BlockBuilder_apply_extrinsic",
			&i.encode(),
			true,
			None,
		).0.unwrap();
	}

	let header = match executor.call::<_, NeverNativeValue, fn() -> _>(
		ext,
		"BlockBuilder_finalize_block",
		&[0u8;0],
		true,
		None,
	).0.unwrap() {
		NativeOrEncoded::Native(_) => unreachable!(),
		NativeOrEncoded::Encoded(h) => Header::decode(&mut &h[..]).unwrap(),
	};

	let hash = header.blake2_256();
	(Block { header, extrinsics }.encode(), hash.into())
}


fn test_blocks(genesis_config: &GenesisConfig, executor: &NativeExecutor<Executor>)
			   -> Vec<(Vec<u8>, Hash)>
{
	let mut test_ext = new_test_ext(genesis_config);
	let mut block1_extrinsics = vec![
		CheckedExtrinsic {
			signed: None,
			function: Call::Timestamp(timestamp::Call::set(42 * 1000)),
		},
	];
	block1_extrinsics.extend((0..20).map(|i| {
		CheckedExtrinsic {
			signed: Some((alice(), signed_extra(i, 0))),
			function: Call::Balances(balances::Call::transfer(bob().into(), 1 * DOLLARS)),
		}
	}));
	let block1 = construct_block(
		executor,
		&mut test_ext.ext(),
		1,
		GENESIS_HASH.into(),
		block1_extrinsics,
	);

	vec![block1]
}

fn bench_execute_block(c: &mut Criterion) {
	c.bench_function_over_inputs(
		"execute blocks",
		|b, strategy| {
			let genesis_config =plasm_testing::genesis::config(false, Some(COMPACT_CODE));
			let (use_native, wasm_method) = match strategy {
				ExecutionMethod::Native => (true, WasmExecutionMethod::Interpreted),
				ExecutionMethod::Wasm(wasm_method) => (false, *wasm_method),
			};
			let executor = NativeExecutor::new(wasm_method, None);

			// Get the runtime version to initialize the runtimes cache.
			{
				let mut test_ext = new_test_ext(&genesis_config);
				executor.runtime_version(&mut test_ext.ext());
			}

			let blocks = test_blocks(&genesis_config, &executor);

			b.iter_batched_ref(
				|| new_test_ext(&genesis_config),
				|test_ext| {
					for block in blocks.iter() {
						executor.call::<_, NeverNativeValue, fn() -> _>(
							&mut test_ext.ext(),
							"Core_execute_block",
							&block.0,
							use_native,
							None,
						).0.unwrap();
					}
				},
				BatchSize::LargeInput,
			);
		},
		vec![
			ExecutionMethod::Native,
			ExecutionMethod::Wasm(WasmExecutionMethod::Interpreted),
			#[cfg(feature = "wasmtime")]
				ExecutionMethod::Wasm(WasmExecutionMethod::Compiled),
		],
	);
}
