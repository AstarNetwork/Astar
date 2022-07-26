import { expect } from 'chai';
import { describeWithNetwork, capitalize } from './util.js';
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

	it('should fetch chain name from rpc node', async function () {
		const name = await context.api.rpc.system.name();

		expect(name.toString()).to.equal('Astar Collator');
	});

	it('should be able to Register contract on H160 address 0x01 using Alice account', function (done) {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });

        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const alice = keyring.addFromUri('//Alice');

        let unsubscribe;

        // Create a extrinsic, transferring 200 units to alice
        api.tx.dappsStaking
            .register(getAddressEnum(CONTRACT))
            .signAndSend(alice, async (result) => {
                console.log(`Current status is ${result?.status}`);

                if (result?.status?.isInBlock) {
                    if (unsubscribe) {
                        unsubscribe();
                    }

                    const blockHash = result?.status?.asInBlock;

                    const apiAt = await api.at(blockHash);

                    const events = await apiAt.query.system.events();

                    events.forEach(f => console.log(f.toHuman()));

                    const filtered = events.filter((eventObj) => {
                        const { event: { method, section, data } } = eventObj;

                        return (
                            section === 'dappsStaking' &&
                            method === 'NewContract'
                        );
                    });

                    expect(result?.status?.isInBlock).to.be.true;

                    // Check event is in block for transfer
                    expect(filtered.length).to.equal(1);
                    expect(filtered[0].event.section).to.equal('dappsStaking');
                    expect(filtered[0].event.method).to.equal('NewContract');
                    done();
                }
            })
            .then(unsub => {
                unsubscribe = unsub;
            });
    });

	it('should be able to transfer tokens from alice to bob', function (done) {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });

        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const alice = keyring.addFromUri('//Alice');

        let unsubscribe;

        // Create a extrinsic, transferring 100 units to Bob
        api.tx.balances
            .transfer(BOB, 100)
            .signAndSend(alice, async (result) => {
                console.log(`Current status is ${result?.status}`);

                if (result?.status?.isInBlock) {
                    if (unsubscribe) {
                        unsubscribe();
                    }

                    const blockHash = result?.status?.asInBlock;

                    const apiAt = await api.at(blockHash);

                    const events = await apiAt.query.system.events();

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

                    expect(result?.status?.isInBlock).to.be.true;

                    // Check event is in block for transfer
                    expect(filtered.length).to.equal(1);
                    expect(filtered[0].event.section).to.equal('balances');
                    expect(filtered[0].event.method).to.equal('Transfer');
                    done();
                }
            })
            .then(unsub => {
                unsubscribe = unsub;
            });
    });

    it('should be able to transfer tokens from bob to alice', function (done) {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });

        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const bob = keyring.addFromUri('//Bob');

        let unsubscribe;

        // Create a extrinsic, transferring 200 units to alice
        api.tx.balances
            .transfer(ALICE, 200)
            .signAndSend(bob, async (result) => {
                console.log(`Current status is ${result?.status}`);

                if (result?.status?.isInBlock) {
                    if (unsubscribe) {
                        unsubscribe();
                    }

                    const blockHash = result?.status?.asInBlock;

                    const apiAt = await api.at(blockHash);

                    const events = await apiAt.query.system.events();

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

                    expect(result?.status?.isInBlock).to.be.true;

                    // Check event is in block for transfer
                    expect(filtered.length).to.equal(1);
                    expect(filtered[0].event.section).to.equal('balances');
                    expect(filtered[0].event.method).to.equal('Transfer');
                    done();
                }
            })
            .then(unsub => {
                unsubscribe = unsub;
            });
    });
});
