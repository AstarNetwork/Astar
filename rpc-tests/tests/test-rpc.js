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
            .signAndSend(alice, (result) => {
                console.log(`Current status is ${result?.status}`);

                if (result?.status?.isInBlock) {
                    if (unsubscribe) {
                        unsubscribe();
                    }

                    expect(result?.status?.isInBlock).to.be.true;
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
            .signAndSend(alice, (result) => {
                console.log(`Current status is ${result?.status}`);

                if (result?.status?.isInBlock) {
                    if (unsubscribe) {
                        unsubscribe();
                    }

                    expect(result?.status?.isInBlock).to.be.true;
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
            .signAndSend(bob, (result) => {
                console.log(`Current status is ${result?.status}`);

                if (result?.status?.isInBlock) {
                    if (unsubscribe) {
                        unsubscribe();
                    }

                    expect(result?.status?.isInBlock).to.be.true;
                    done();
                }
            })
            .then(unsub => {
                unsubscribe = unsub;
            });
    });
});
