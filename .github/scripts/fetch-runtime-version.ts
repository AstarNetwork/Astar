import { ApiPromise, WsProvider } from '@polkadot/api';

async function main() {
    const endpoint = process.env.ENDPOINT;

    if (!endpoint) {
        console.error('ENDPOINT environment variable is required');
        process.exit(1);
    }

    const provider = new WsProvider(endpoint);
    const api = await ApiPromise.create({ provider });

    try {
        const version = await api.rpc.state.getRuntimeVersion();
        const specVersion = version.specVersion.toString().replace(/,/g, '');
        console.log(specVersion);
    } finally {
        await api.disconnect();
    }
}

main().catch((error) => {
    console.error('Error:', error);
    process.exit(1);
});