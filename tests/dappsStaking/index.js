import { WsProvider, ApiPromise } from '@polkadot/api';
import plasmTypes from '@plasm/types';

const { plasmDefinitions } = plasmTypes;

async function main() {
    // using the ApiPromise class
    const api = await ApiPromise.create({
        provider: new WsProvider('wss://astar.api.onfinality.io/public-ws'),
        types: {
            ...plasmDefinitions,
        }
    });

    await api.isReady;

    api.rpc.chain.subscribeNewHeads(async (header) => {
        console.log(`Chain is at #${header.number}`);
    });
}

main().catch(console.error);