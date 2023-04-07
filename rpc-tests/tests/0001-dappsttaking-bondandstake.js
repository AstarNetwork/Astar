// https://github.com/AstarNetwork/chain-extension-contracts/blob/main/tests/dapps-staking.spec.ts
async function run(nodeName, networkInfo, args) {
    const BN = require('bn.js');
    const ONE = new BN(10).pow(new BN(18));

    const { sendTransaction } = await import("./tx-utils.mjs");

    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    // account to submit tx
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const sender = keyring.addFromUri("//" + args[0]);

    const CONTRACT = '0x0000000000000000000000000000000000000001'; //0x01
    const getAddressEnum = (address) => ({ Evm: address });

    await sendTransaction(
        api.tx.dappsStaking.bondAndStake(getAddressEnum(CONTRACT), ONE.muln(10000)),
        sender
    );

    const result = 1
    return result;
}

module.exports = { run }
