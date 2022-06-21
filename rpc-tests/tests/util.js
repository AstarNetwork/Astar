import { ApiPromise, WsProvider } from '@polkadot/api';
import { spawn } from 'child_process';
import chaiAsPromised from 'chai-as-promised';
import chai from 'chai';
import { typesBundleForPolkadot } from '@acala-network/type-definitions';

export default { typesBundle: typesBundleForPolkadot };

chai.use(chaiAsPromised);

export const BINARY_PATH = `../target/release/astar-collator`;
export const SPAWNING_TIME = 120000;
const WS_PORT = 9944;

export async function wait (milliseconds = 0) {
	return new Promise((resolve, reject) => {
		setTimeout(resolve, milliseconds);
	});
}

export async function startAstarNode() {
	const cmd = BINARY_PATH;
	const args = [
		`--dev`,
		`--execution=native`, // Faster execution using native
		`--no-telemetry`,
		`--no-prometheus`,
		`--port=30333`,
		`--rpc-port=9933`,
		`--rpc-external`,
		`--ws-port=${WS_PORT}`,
		`--ws-external`,
		`--rpc-cors=all`,
		`--rpc-methods=unsafe`,
		`--tmp`,
	];
	const binary = spawn(cmd, args);

	binary.on('error', (err) => {
		if ((err).errno == 'ENOENT') {
			console.error(
				`\x1b[31mMissing Astar collator binary (${BINARY_PATH}).\nPlease compile the Astar project: cargo build --release`
			);
		} else {
			console.error(err);
		}
		process.exit(1);
	});

	let api;
	await new Promise((resolve, reject) => {
		const timer = setTimeout(() => {
			console.error(`Failed to start Astar Collator.`);
			console.error(`Command: ${cmd} ${args.join(' ')}`);
			process.exit(1);
		}, SPAWNING_TIME - 2000);

		const onData = async (chunk) => {
			console.log(chunk.toString());
			if (chunk.toString().match(/Imported #1/)) {
				try {
					api = await ApiPromise.create({
						provider: new WsProvider(`ws://localhost:${WS_PORT}`),
						typesBundle: typesBundleForPolkadot
					});
					await api.isReady;

					clearTimeout(timer);
					resolve();
				} catch(e) {
					binary.kill();
					reject(e);
				}
			}
		};
		binary.stderr.on('data', onData);
		binary.stdout.on('data', onData);
	});

	return { api, binary };
}

export function describeWithAstar(title, cb) {
	describe(title, () => {
		let context = { api: null };
		let binary;
		// Making sure the Astar node has started
		before('Starting Astar Test Node', async function () {
			this.timeout(SPAWNING_TIME);
			const init = await startAstarNode();
			context.api = init.api;
			binary = init.binary;
		});

		after(async function () {
			context.api.disconnect()
			binary.kill();
		});

		cb(context);
	});
}
