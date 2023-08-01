import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/api';
import { TypeRegistry } from '@polkadot/types';


const wsProvider = new WsProvider('ws://127.0.0.1:9944');
const registry = new TypeRegistry();
registry.register({ AccountId: 'AccountId20' });
console.log(registry.createType('AccountId').toString());
const api = await ApiPromise.create({
    provider: wsProvider, registry
});


const keyring = new Keyring({ type: 'ethereum' });

const alice = keyring.addFromUri('0x70bd97b3f549bbda0b9420b2210673c5b24e807574c0b7cf2a18fb4f7149f6b9');
const bob = 'ajYMsCKsEAhEvHpeA4XqsfiA9v1CdzZPrCfS6pEfeGHW9j8';

const tx = await api.tx.assets
    .forceClearMetadata(12345n)

const encodedCallData = tx.method.toHex()
console.log(encodedCallData)

api.registry.register({ AccountId: 'AccountId20' });
api.registry.register({ Lookup0: 'AccountId20' });
const txHash = await tx
    .signAndSend(alice);

console.log(`Submitted with hash ${txHash}`);

// Disconnect the API
api.disconnect();
