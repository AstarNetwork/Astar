import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/api';
import chaiAsPromised from 'chai-as-promised';
import chai from 'chai';

chai.use(chaiAsPromised);

export const SPAWNING_TIME = 500000;
export const WS_PORT = process.env.WS_PORT;

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
 * sendTransaction: sign and send transaction from sender accounts.
 *
 * @param {*} transaction polkadot js api transaction
 * @param {*} sender account from which transaction needs to be sent
 *
 * @returns true when transaction is finalised
 */
export async function sendTransaction(transaction, sender) {
	return new Promise((resolve, reject) => {
		let unsubscribe;
		let timeout;

		transaction.signAndSend(sender, async (result) => {
			console.log(`Current status is ${result?.status}`);

			if (result.isFinalized) {
				if (unsubscribe) {
					unsubscribe();
				}

				clearTimeout(timeout);
				resolve(true);
			}
		}).then(unsub => {
			unsubscribe = unsub;
		}).catch(error => {
			console.error(error);
			reject(error);
		});

		timeout = setTimeout(() => {
			reject(new Error('Transaction timeout'));
		}, SPAWNING_TIME);
	});
}

/**
 * describeWithNetwork: special mocha describe which has global setup and teardown steps for spawning and taking down local network
 *
 * @param {string} network [astar | shiden | shibuya]
 * @param {number} paraId [2006 | 2007 | 1000]
 * @param {*} title title of the test suite
 * @param {*} cb callback function which take a context object which will be available in tests
 */
export function describeWithNetwork(network, paraId, title, cb) {
	describe(title, () => {
		let context = {
			api: null,
			keyring: null,
			alice: null,
			bob: null,
			charlie: null,
			dave: null
		};
		let timeout;

		// Making sure the Astar node has started
		before('Starting Astar Test Node', async function () {
			this.timeout(SPAWNING_TIME);

			const api = await ApiPromise.create({
				provider: new WsProvider(`ws://localhost:${WS_PORT}`)
			});
			await api.isReady;

			await new Promise((resolve, reject) => {
				let unsubHeads;

				api.rpc.chain.subscribeNewHeads((lastHead) => {
					console.log('Parachain blocks:', lastHead.number.toNumber());
					if (lastHead.number.toNumber() > 1) {
						if (unsubHeads) {
							console.log('unsubscribing');
							unsubHeads();
						}

						clearTimeout(timeout);
						resolve();
					}
				}).then(unsub => {
					unsubHeads = unsub;
				});

				timeout = setTimeout(() => {
					reject(new Error('Block production failed'));
				}, SPAWNING_TIME);
			});

			const keyring = new Keyring({ type: 'sr25519' });

			const alice = keyring.addFromUri('//Alice');
			const bob = keyring.addFromUri('//Bob');
			const charlie = keyring.addFromUri('//Charlie');
			const dave = keyring.addFromUri('//Dave');

			context.api = api;
			context.keyring = keyring;
			context.alice = alice;
			context.bob = bob;
			context.charlie = charlie;
			context.dave = dave;
		});

		after(async function () {
			await context.api.disconnect()
			process.exit(0);
		});

		cb(context);
	});
}
