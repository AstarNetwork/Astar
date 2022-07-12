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

	it('should be able to Register contract on H160 address 0x01 using Alice account', async function () {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });
      
        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const alice = keyring.addFromUri('//Alice');

        // Create a extrinsic, transferring 100 units to Bob
        const transfer = api.tx.dappsStaking.register(getAddressEnum(CONTRACT));
      
        // Sign and send the transaction using our account
        const hash = await transfer.signAndSend(alice, { nonce: -1 });

        console.log('Transfer sent with hash', hash.toHex());

        expect(hash).to.exist;
    });

	it('should be able to transfer tokens from alice to bob', async function () {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });
      
        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const alice = keyring.addFromUri('//Alice');

        // Create a extrinsic, transferring 100 units to Bob
        const transfer = api.tx.balances.transfer(BOB, 100);
      
        // Sign and send the transaction using our account
        const hash = await transfer.signAndSend(alice, { nonce: -1 });

        console.log('Transfer sent with hash', hash.toHex());

        expect(hash).to.exist;
    });

    it('should be able to transfer tokens from bob to alice', async function () {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });

        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const bob = keyring.addFromUri('//Bob');

        // Create a extrinsic, transferring 100 units to Bob
        const transfer = api.tx.balances.transfer(ALICE, 200);

        // Sign and send the transaction using our account
        const hash = await transfer.signAndSend(bob, { nonce: -1 });

        console.log('Transfer sent with hash', hash.toHex());

        expect(hash).to.exist;
    });
});
