async function run(nodeName, networkInfo, args) {
    networkInfo.nodesByName["astar"].hrmpTo = 2007;
    networkInfo.nodesByName["shiden"].hrmpTo = 2006;

    const { wsUri, userDefinedTypes, hrmpTo } =
        networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const BN = require("bn.js");
    const ONE = new BN(10).pow(new BN(18));

    await zombie.util.cryptoWaitReady();

    const assetsAsset = await api.query.assets.asset(hrmpTo);
    const supply = parseInt(JSON.parse(assetsAsset).supply, 16);
    console.log('assetsAsset.supply', supply);

    const twentyone = parseInt(ONE.muln(21000).toString(), 10);
    console.log('ONE.muln(21000)', twentyone);

    const result = twentyone === supply ? 1 : 0;
    return result;
}

module.exports = { run };
