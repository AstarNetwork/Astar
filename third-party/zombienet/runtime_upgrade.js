const assert = require("assert");

async function run(nodeName, networkInfo, _args) {
    const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    // get blockhash/runtimeVersion at block 1
    const hashAtBlock1 = await api.rpc.chain.getBlockHash(1);
    const versionAtBlock1 = await api.rpc.state.getRuntimeVersion(hashAtBlock1.toHuman());
    const initSpecVersion = versionAtBlock1.specVersion.toHuman();
    console.log("specVersion: ", initSpecVersion);

    // get blockhash/runtimeVersion at current head
    const currentHeader = await api.rpc.chain.getHeader();
    const hashAtCurrent = await api.rpc.chain.getBlockHash(currentHeader.number.toHuman());
    const versionAtCurrent = await api.rpc.state.getRuntimeVersion(hashAtCurrent.toHuman());
    const humanCurrent = versionAtCurrent.specVersion.toHuman();
    console.log("specVersion after upgrade: ", humanCurrent);


    const expectedVersion = parseInt(versionAtBlock1.specVersion.toNumber(), 10) + 1;
    const currentSpecVersion = parseInt(versionAtCurrent.specVersion.toNumber(), 10);
    assert.ok(currentSpecVersion >= expectedVersion, "Current specVersion after upgrade is not increased. Aborting!");
}

module.exports = { run }
