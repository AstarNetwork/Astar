async function run(nodeName, networkInfo, args) {
    const { sendTransaction } = await import("./tx-utils.mjs");

    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    // account to submit tx
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const FROM = keyring.addFromUri("//" + args[0]);
    const TO = keyring.addFromUri("//" + args[1]);
    const AMOUNT = 1000000000000000;

    const originalBalance = await api.query.system.account(TO.address);
    console.log('originalBalance', originalBalance.toString());

    await sendTransaction(
        api.tx.balances.transfer({ Id: TO.address }, AMOUNT),
        FROM
    );

    const newBalance = await api.query.system.account(TO.address);
    console.log('newBalance', newBalance.toString());

    const difference = newBalance.data.free - originalBalance.data.free
    const result =  difference === AMOUNT ? 1 : 0

    return result;
}

module.exports = { run }
