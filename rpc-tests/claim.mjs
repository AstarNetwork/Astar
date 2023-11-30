/// TODO: make this into a rpc test when integrated in Shiden & Astar

import { JsonRpcProvider, Wallet } from "ethers"
import { WsProvider, ApiPromise, Keyring } from '@polkadot/api';

async function waitForTx(tx, signer, api) {
    return new Promise((resolve, reject) => {
        tx.signAndSend(signer, (result) => {
            if (result.status.isInBlock) {
                console.log(`\t Transaction included at blockHash ${result.status.asInBlock}`);
            } else if (result.status.isFinalized) {
                console.log(`\t Transaction finalized at blockHash ${result.status.asFinalized}`);
                result.events.forEach(({ phase, event: { data, method, section } }) => {
                    console.log(`\t' ${phase}: ${section}.${method}:: ${data}`);
                });
                resolve(result.txHash)
            } else if (result.isError) {
                reject();
            }
        })
    })
}

async function buildSignature(signer, substrateAddress, api, chainId) {
    return await signer.signTypedData({
        chainId,
        name: "Astar EVM Claim",
        version: "1",
        salt: await api.query.system.blockHash(0) // genisis hash
    }, {
        Claim: [
            { name: 'substrateAddress', type: 'bytes' }
        ],
    }, {
        substrateAddress
    })
}

async function claimEvmAccount(account, evmAddress, signature, api) {
    return await waitForTx(api.tx.unifiedAccounts.claimEvmAddress(evmAddress, signature), account)
}

async function main() {
    const api = await ApiPromise.create({ provider: new WsProvider('ws://127.0.0.1:9944') });
    await api.isReady;

    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice', { name: 'Alice default' })

    const provider = new JsonRpcProvider("http://127.0.0.1:9944");
    const { chainId } = await provider.getNetwork();
    const ethSigner = new Wallet("0x01ab6e801c06e59ca97a14fc0a1978b27fa366fc87450e0b65459dd3515b7391", provider);

    const sig = await buildSignature(ethSigner, alice.publicKey, api, chainId);
    console.log(`Signature - ${sig}`)
    const hash = await claimEvmAccount(alice, ethSigner.address, sig, api);
    console.log(`Claim Extrisic - ${hash}`);

    console.log(`Claimed Account ${await api.query.unifiedAccounts.evmToNative(alice.address)}, EVM Account: ${ethSigner.address}`);
    console.log(`EVM Balance=${await provider.getBalance(ethSigner.address)}`)

    api.disconnect();
}

await main()
