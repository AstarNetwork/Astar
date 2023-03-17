async function run(nodeName, networkInfo, args) {
    networkInfo.nodesByName["astar"].hrmpTo = 2007
    networkInfo.nodesByName["shiden"].hrmpTo = 2006

    const { sendTransaction } = await import("./tx-utils.mjs");
    const { wsUri, userDefinedTypes, hrmpTo } = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    // account to submit tx
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const sender = keyring.addFromUri("//" + args[0]);

    const assetLocation = `{"v1":{"parents":1,"interior":{"x2":[{"parachain":${hrmpTo}},{"generalKey":"0x"}]}}}`;

    const tx = await api.tx.xcAssetConfig.registerAssetLocation(JSON.parse(assetLocation), hrmpTo);
    const finalised = await sendTransaction(api.tx.sudo.sudo(tx), sender);

    const assetIdToLocation = await api.query.xcAssetConfig.assetIdToLocation(hrmpTo);
    const location = JSON.stringify(assetIdToLocation);
    console.log("location", location);

    const result = assetLocation === location ? 1 : 0;
    return result;
}

module.exports = { run };
