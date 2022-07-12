import { ApiPromise, WsProvider } from '@polkadot/api';
import chaiAsPromised from 'chai-as-promised';
import chai from 'chai';
import { run, killAll } from 'polkadot-launch';

import config from '../config.js';

chai.use(chaiAsPromised);

export const BINARY_PATH = `../target/release/astar-collator`;
export const SPAWNING_TIME = 120000;
const WS_PORT = 9988;

export const capitalize = str => str.split(' ').map(sub => sub.charAt(0).toUpperCase() + sub.slice(1)).join(' ');

export async function wait (milliseconds = 0) {
	return new Promise((resolve, reject) => {
		setTimeout(resolve, milliseconds);
	});
}

export function describeWithNetwork(network, title, cb) {
	describe(title, () => {
		let context = { api: null };
		// Making sure the Astar node has started
		before('Starting Astar Test Node', async function () {
			this.timeout(SPAWNING_TIME);
			
			await run(process.cwd(), config(network));

			const api = await ApiPromise.create({
				provider: new WsProvider(`ws://localhost:${WS_PORT}`)
			});
			await api.isReady;
			context.api = api;
		});

		after(async function () {
			context.api.disconnect()
			killAll();
		});

		cb(context);
	});
}
