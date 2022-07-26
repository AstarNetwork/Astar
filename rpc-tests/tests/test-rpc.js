import { expect } from 'chai';
import {
    capitalize,
    describeWithNetwork,
    sendTransaction
} from './util.js';
import { Keyring } from '@polkadot/api';

const CONTRACT = '0x0000000000000000000000000000000000000001'; //0x01
const ALICE = 'ajYMsCKsEAhEvHpeA4XqsfiA9v1CdzZPrCfS6pEfeGHW9j8';
const BOB = 'ZAP5o2BjWAo5uoKDE6b6Xkk4Ju7k6bDu24LNjgZbfM3iyiR';

export const getAddressEnum = (address) => ({ Evm: address });

const network = process.env.NETWORK;

if (['astar', 'shiden', 'shibuya'].includes(network) === false) {
    throw new Error('Please set valid network in NETWORK env variable');
}

describeWithNetwork(network, `${network} RPC`, function(context) {
	it('should fetch chain from rpc node', async function () {
		const chain = await context.api.rpc.system.chain();

		expect(chain.toString()).to.equal(`${capitalize(network)} Testnet`);
	});

	it('should fetch chain name from rpc node', async () => {
		const name = await context.api.rpc.system.name();

		expect(name.toString()).to.equal('Astar Collator');
	});

	it('should be able to Register contract on H160 address 0x01 using Alice account', async () => {
        const events = await sendTransaction(
            context.api,
            context.api.tx.dappsStaking.register(getAddressEnum(CONTRACT)),
            context.alice,
            'dappsStaking',
            'NewContract'
        );

        expect(events.length).to.equal(1);
        expect(events[0].event.section).to.equal('dappsStaking');
        expect(events[0].event.method).to.equal('NewContract');
        expect(events[0].event.data[0].toString()).to.equal(ALICE);
    });

	it('should be able to transfer tokens from alice to bob', async () => {
        const events = await sendTransaction(
            context.api,
            context.api.tx.balances.transfer(BOB, 100),
            context.alice,
            'balances',
            'Transfer'
        );


        const filtered = events.filter((eventObj) => {
            const { event: { method, section, data } } = eventObj;

            return (
                section === 'balances' &&
                method === 'Transfer' &&
                data[0].toString() === ALICE &&
                data[1].toString() === BOB &&
                data[2].toString() === '100'
            );
        });

        // Check filtered event
        expect(filtered.length).to.equal(1);
        expect(filtered[0].event.section).to.equal('balances');
        expect(filtered[0].event.method).to.equal('Transfer');
    });

    it('should be able to transfer tokens from bob to alice', async () => {
        const events = await sendTransaction(
            context.api,
            context.api.tx.balances.transfer(ALICE, 200),
            context.bob,
            'balances',
            'Transfer'
        );


        const filtered = events.filter((eventObj) => {
            const { event: { method, section, data } } = eventObj;

            return (
                section === 'balances' &&
                method === 'Transfer' &&
                data[0].toString() === BOB &&
                data[1].toString() === ALICE &&
                data[2].toString() === '200'
            );
        });

        // Check filtered event
        expect(filtered.length).to.equal(1);
        expect(filtered[0].event.section).to.equal('balances');
        expect(filtered[0].event.method).to.equal('Transfer');
    });
});
