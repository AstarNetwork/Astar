async function run(nodeName, networkInfo, args) {
    const polkadotCryptoUtils = require("@polkadot/util-crypto");

    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const SPAWNING_TIME = 500000;

    async function sendTransaction(transaction, sender) {
      return new Promise((resolve, reject) => {
        let unsubscribe;
        let timeout;

        transaction.signAndSend(sender, async (result) => {
          console.log(`Current status is ${result?.status}`);

          if (result.isFinalized) {
            if (unsubscribe) {
              unsubscribe();
            }

            clearTimeout(timeout);
            resolve(true);
          }
        }).then(unsub => {
          unsubscribe = unsub;
        }).catch(error => {
          console.error(error);
          reject(error);
        });

        timeout = setTimeout(() => {
          reject(new Error('Transaction timeout'));
        }, SPAWNING_TIME);
      });
    }

    await zombie.util.cryptoWaitReady();

    const keyring = new zombie.Keyring({ type: "sr25519" });
    const FROM = keyring.addFromUri("//" + args[0]);

    const evmAccount = {
      privateKey: '0x01ab6e801c06e59ca97a14fc0a1978b27fa366fc87450e0b65459dd3515b7391',
      address: '0xaaafB3972B05630fCceE866eC69CdADd9baC2771',
    };
    const addressPrefix = 5;
    const TO = polkadotCryptoUtils.evmToAddress(evmAccount.address, addressPrefix);
    const AMOUNT = 1000000000000000;

    const originalBalance = await api.query.system.account(TO);
    console.log('originalBalance', originalBalance.toString());

    await sendTransaction(
        api.tx.balances.transfer({ Id: TO }, AMOUNT),
        FROM
    );
  
    const newBalance = await api.query.system.account(TO);
    console.log('newBalance', newBalance.toString());

    const difference = newBalance.data.free - originalBalance.data.free
    const result =  difference === AMOUNT ? 1 : 0

    return result;
}

module.exports = { run }
