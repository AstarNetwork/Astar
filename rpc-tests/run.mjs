import { WsProvider, ApiPromise, Keyring } from '@polkadot/api';
import { evmToAddress } from "@polkadot/util-crypto";
import { JsonRpcProvider, Wallet, ethers, keccak256 } from "ethers";

const MAGIC_NUMBER = 0xff51;

function signMessage(bytes, signer) {
    const hashed = ethers.getBytes(keccak256(bytes))
    return ethSigner.signMessage(hashed);
}

async function waitForTx(tx, signer) {
    return new Promise((resolve, reject) => {
        tx.signAndSend(signer, (result) => {
            if (result.status.isInBlock) {
                console.log(`\t Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`\t Transaction finalized at blockHash ${result.status.asFinalized}`);
                resolve(result.txHash)
            } else if (result.isError) {
                reject();
            }
        })
    })
}

const wsProvider = new WsProvider('ws://127.0.0.1:9944');
const api = await ApiPromise.create({ provider: wsProvider });
const keyring = new Keyring({ type: 'sr25519' });
// only for transfering some balance to eth account
const alice = keyring.addFromUri('//Alice', { name: 'Alice default' });

const ethProvider = new JsonRpcProvider('http://127.0.0.1:9944');
const ethSigner = Wallet.createRandom(ethProvider);
const ss58Address = evmToAddress(ethSigner.address);


// transfer some balance to eth account
console.log(`Sending some balance to evm account - ${await waitForTx(api.tx.balances.transfer(ss58Address, 100_000_000_000_000_000_000n), alice)}`);
console.log(`ETH Account Balance - ${await ethProvider.getBalance(ethSigner.address)}`);


const sampleTx = api.tx.system.remarkWithEvent("hello")
const value = api.createType("(u16, u32, RuntimeCall)", [MAGIC_NUMBER, 0, sampleTx])
const signed = await signMessage(value.toU8a(), ethSigner);

console.log(`Unsigned value - ${value.toU8a().toString()}`)
console.log(`Signed value - ${signed.toString()}`)

const unsignedTx = api.tx.ethCall.call(sampleTx, ethSigner.address, signed, 0);
// send it without calling sign, pass callback with status/events
unsignedTx.send(({ status }) => {
    if (status.isInBlock) {
        console.log(`Call included in ${status.asInBlock}`);
        api.disconnect();
    }
});

