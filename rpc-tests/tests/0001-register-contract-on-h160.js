async function run(nodeName, networkInfo, args) {
    const { sendTransaction } = await import("./tx-utils.mjs");

    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    // account to submit tx
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const sender = keyring.addFromUri("//" + args[0]);

    const CONTRACT = '0x0000000000000000000000000000000000000001'; //0x01
    const getAddressEnum = (address) => ({ Evm: address });

    const tx = await api.tx.dappsStaking.register(sender.address, getAddressEnum(CONTRACT))

    const finalised = await sendTransaction(
        api.tx.sudo.sudo(tx),
        sender
    );

    const dappInfoOpt = await api.query.dappsStaking.registeredDapps(getAddressEnum(CONTRACT));
    const dappInfo = dappInfoOpt.unwrap();

    const result = (dappInfo.state.toString() === 'Registered') ? 1 : 0
    return result;
}

module.exports = { run }
