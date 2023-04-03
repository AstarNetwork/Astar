async function run(nodeName, networkInfo, args) {
    networkInfo.nodesByName["astar"].hrmpTo = 2007
    networkInfo.nodesByName["shiden"].hrmpTo = 2006

    const { sendTransaction } = await import("./tx-utils.mjs");
    const { wsUri, userDefinedTypes, hrmpTo } = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);
    const assetId = hrmpTo; // assetId can be any arbitrary number

    await zombie.util.cryptoWaitReady();

    // account to submit tx
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const sender = keyring.addFromUri("//" + args[0]);

    const tx0 = await api.tx.assets.forceCreate(assetId, {id: sender.address}, true, 1);

    const assetLocation = `{"v1":{"parents":1,"interior":{"x1":{"parachain":${hrmpTo}}}}}`;
    const tx1 = await api.tx.xcAssetConfig.registerAssetLocation(JSON.parse(assetLocation), assetId);

    const unitsPerSecond = 7000000000;
    const tx2 = await api.tx.xcAssetConfig.setAssetUnitsPerSecond(JSON.parse(assetLocation), unitsPerSecond);

    const batch = await api.tx.utility.batch([ tx0, tx1, tx2 ]);
    await sendTransaction(api.tx.sudo.sudo(batch), sender);

    const assetIdToLocation = await api.query.xcAssetConfig.assetIdToLocation(assetId);
    const location = JSON.stringify(assetIdToLocation);

    const assetLocationUnitsPerSecond = await api.query.xcAssetConfig.assetLocationUnitsPerSecond(JSON.parse(assetLocation));
    const units = JSON.stringify(assetLocationUnitsPerSecond);

    const result = (assetLocation === location) && (unitsPerSecond === units) ? 1 : 0;
    return result;
}

module.exports = { run };
