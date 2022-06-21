import { expect } from 'chai';
import { Keyring } from '@polkadot/api';
import { describeWithAstar, wait } from './util.js';

const CONTRACT = '0x000000000000000000000000000000000000000001'; //0x01

describeWithAstar('Dapp Staking', function(context) {
	it('should be able to Register contract on H160 address 0x01 using Alice account', async function () {
        const api = context.api;

        // Construct the keyring after the API (crypto has an async init)
        const keyring = new Keyring({ type: 'sr25519' });
      
        // Add Alice to our keyring with a hard-derivation path (empty phrase, so uses dev)
        const alice = keyring.addFromUri('//Alice');

        // Create a extrinsic, transferring 100 units to Bob
        const transfer = api.tx.dappsStaking.register(CONTRACT);
      
        // Sign and send the transaction using our account
        const hash = await transfer.signAndSend(alice);

        console.log('Transfer sent with hash', hash.toHex());

        expect(hash).to.exist;
    });
});
