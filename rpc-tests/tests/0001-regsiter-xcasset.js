async function run(nodeName, networkInfo, args) {
    const { sendTransaction } = await import("./tx-utils.mjs");

    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    // account to submit tx
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const sender = keyring.addFromUri("//" + args[0]);

    // https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A43003#/extrinsics/decode/0x6300360001010200591f060430591f
    const assetLocation = '{ XcmVersionedMultiLocation: { V1: { parents: 1, interior: { X2: [ { Parachain: 2006 }, { GeneralKey: 0 } ] } } } }';
    const assetId = '2006';

    const tx = await api.tx.xcAssetConfig.registerAssetLocation(assetLocation, assetId)

    const finalised = await sendTransaction(
        api.tx.sudo.sudo(tx),
        sender
    );

    const assetIdToLocation = await api.query.xcAssetConfig.assetIdToLocation('2006');

    // return config.unwrap().v1;
    const location = assetIdToLocation.unwrap();
    console.log('location', location);

    const result = ('Registered' === 'Registered') ? 1 : 0
    return result;
}

module.exports = { run }
