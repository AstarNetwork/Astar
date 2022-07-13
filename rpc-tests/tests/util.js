import { ApiPromise, WsProvider } from '@polkadot/api';
import chaiAsPromised from 'chai-as-promised';
import chai from 'chai';
import { run, killAll } from 'polkadot-launch';

import config from '../config.js';

chai.use(chaiAsPromised);

export const SPAWNING_TIME = 120000;
const WS_PORT = 9988;

/**
 * capitalize: capitalize first letter of a string
 *
 * @param {String} str
 * @returns
 */
export const capitalize = str => str.split(' ').map(sub => sub.charAt(0).toUpperCase() + sub.slice(1)).join(' ');

/**
 * wait: async function which wait for milliseconds
 *
 * @param {Number} milliseconds
 * @returns
 */
export async function wait (milliseconds = 0) {
	return new Promise((resolve, reject) => {
		setTimeout(resolve, milliseconds);
	});
}

/**
 * describeWithNetwork: special mocha describe which has global setup and teardown steps for spawning and taking down local network
 *
 * @param {string} network [astar | shiden | shibuya]
 * @param {*} title title of the test suite
 * @param {*} cb callback function which take a context object which will be available in tests
 */
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
