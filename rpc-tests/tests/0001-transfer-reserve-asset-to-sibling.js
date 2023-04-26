async function run(nodeName, networkInfo, args) {
    networkInfo.nodesByName["astar"].hrmpTo = 2007;
    networkInfo.nodesByName["shiden"].hrmpTo = 2006;

    const { sendTransaction } = await import("./tx-utils.mjs");
    const { wsUri, userDefinedTypes, hrmpTo } =
        networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const { decodeAddress } = require("@polkadot/util-crypto");
    const BN = require("bn.js");
    const ONE = new BN(10).pow(new BN(18));

    await zombie.util.cryptoWaitReady();

    // account to submit tx
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const sender = keyring.addFromUri("//" + args[0]);

    const tx = await api.tx.polkadotXcm.reserveTransferAssets(
        {
            V3: {
                parents: 1,
                interior: { X1: { Parachain: hrmpTo } },
            },
        },
        {
            V3: {
                parents: 0,
                interior: {
                    X1: {
                        AccountId32: {
                            id: decodeAddress(sender.address),
                        },
                    },
                },
            },
        },
        {
            V3: [
                {
                    fun: { Fungible: ONE.muln(21000) },
                    id: { Concrete: { parents: 0, interior: "Here" } },
                },
            ],
        },
        0
    );

    const finalised = await sendTransaction(tx, sender);

    const result = 1 === 1 ? 1 : 0;
    return result;
}

module.exports = { run };
