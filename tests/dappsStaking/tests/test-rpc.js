import { WsProvider, ApiPromise } from '@polkadot/api';
import plasmTypes from '@plasm/types';
import { expect } from "chai";

const { plasmDefinitions } = plasmTypes;

const RPC_ENDPOINT = 'wss://astar.api.onfinality.io/public-ws';

async function startAstarNode () {
	// TODO: start a node with child_process
	console.log("Starting Astar node...");
}

describe('Astar RPC', function() {
	let api;
	// Making sure the Astar node has started
	before("Connecting to Astar Node", async function () {
		this.timeout(5000);
		const init = await startAstarNode();
		// using the ApiPromise class
		api = await ApiPromise.create({
			provider: new WsProvider(RPC_ENDPOINT),
			types: {
				...plasmDefinitions,
			}
		});
	
		await api.isReady;
	});

	it("should fetch chain from rpc node", async function () {
		const chain = await api.rpc.system.chain();

		expect(chain.toString()).to.equal('Astar');
	});

	it("should fetch chain name from rpc node", async function () {
		const name = await api.rpc.system.name();

		expect(name.toString()).to.equal('Astar Collator');
	});

	after(async function () {
		console.log(`\x1b[31m Disconnecting RPC\x1b[0m`);
		api.disconnect()
	});
});
