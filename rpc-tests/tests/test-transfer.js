import { expect } from 'chai';
import { Keyring } from '@polkadot/api';
import { describeWithAstar, wait } from './util.js';

const ALICE = 'ajYMsCKsEAhEvHpeA4XqsfiA9v1CdzZPrCfS6pEfeGHW9j8';
const BOB = 'ZAP5o2BjWAo5uoKDE6b6Xkk4Ju7k6bDu24LNjgZbfM3iyiR';

describeWithAstar('Token transfer', function(context) {
	it('should be able to transfer tokens from alice to bob', async function () {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });
      
        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const alice = keyring.addFromUri('//Alice');

        let { data: { free: previousFree }, nonce: previousNonce } = await api.query.system.account(BOB);

        // Create a extrinsic, transferring 100 units to Bob
        const transfer = api.tx.balances.transfer(BOB, 100);
      
        // Sign and send the transaction using our account
        const hash = await transfer.signAndSend(alice);

        console.log('Transfer sent with hash', hash.toHex());

        await wait(2000);

        let { data: { free: newFree } } = await api.query.system.account(BOB);

        expect(newFree.sub(previousFree).toString()).to.equals('100');
    });

    it('should be able to transfer tokens from bob to alice', async function () {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });

        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const bob = keyring.addFromUri('//Bob');

        let { data: { free: previousFree }, nonce: previousNonce } = await api.query.system.account(ALICE);

        // Create a extrinsic, transferring 100 units to Bob
        const transfer = api.tx.balances.transfer(ALICE, 200);

        // Sign and send the transaction using our account
        const hash = await transfer.signAndSend(bob);

        console.log('Transfer sent with hash', hash.toHex());

        await wait(2000);

        let { data: { free: newFree } } = await api.query.system.account(ALICE);

        expect(newFree.sub(previousFree).toString()).to.equals('200');
    });
});
