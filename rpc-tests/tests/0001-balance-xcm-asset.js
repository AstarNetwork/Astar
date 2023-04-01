async function run(nodeName, networkInfo, args) {
    networkInfo.nodesByName["astar"].assetId = 2007;
    networkInfo.nodesByName["shiden"].assetId = 2006;

    const { wsUri, userDefinedTypes, assetId } =
        networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const BN = require("bn.js");
    const ONE = new BN(10).pow(new BN(18));

    await zombie.util.cryptoWaitReady();

    const assetsAsset = await api.query.assets.asset(assetId);
    const supply = parseInt(JSON.parse(assetsAsset).supply, 16);
    const twentyone = parseInt(ONE.muln(21000).toString(), 10);

    const result = twentyone === supply ? 1 : 0;
    return result;
}

module.exports = { run };
