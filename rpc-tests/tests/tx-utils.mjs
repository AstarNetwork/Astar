    export async function sendTransaction(transaction, sender) {
      return new Promise((resolve, reject) => {
        let unsubscribe;
        let timeout;
        const SPAWNING_TIME = 500000;

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
