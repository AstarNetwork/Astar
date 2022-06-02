import { WsProvider, ApiPromise } from '@polkadot/api';
import { expect } from 'chai';
import { describeWithAstar } from './util.js'

describeWithAstar('Astar RPC', function(context) {
	it('should fetch chain from rpc node', async function () {
		const chain = await context.api.rpc.system.chain();

		expect(chain.toString()).to.equal('Development');
	});

	it('should fetch chain name from rpc node', async function () {
		const name = await context.api.rpc.system.name();

		expect(name.toString()).to.equal('Astar Collator');
	});
});
