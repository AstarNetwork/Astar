// Rotates keys via `author_rotateKeysWithOwner` and checks `session.setKeys` enforces the proof; returns 1 on success.

const ROTATE_RPC = "author_rotateKeysWithOwner";
const TX_TIMEOUT_MS = 60_000;

// Resolves to null on success, or to the dispatch error name.
function submit(api, tx, signer) {
    return new Promise((resolve, reject) => {
        const timer = setTimeout(
            () => reject(new Error(`tx not in a block after ${TX_TIMEOUT_MS}ms`)),
            TX_TIMEOUT_MS
        );
        tx.signAndSend(signer, ({ status, dispatchError }) => {
            if (!status.isInBlock) return;
            clearTimeout(timer);
            if (!dispatchError) return resolve(null);
            resolve(
                dispatchError.isModule
                    ? api.registry.findMetaError(dispatchError.asModule).name
                    : dispatchError.toString()
            );
        }).catch((err) => {
            clearTimeout(timer);
            reject(err);
        });
    });
}

// Collator order depends on the client tags, so find the one that exposes the RPC.
async function connectToCollatorWithRpc(networkInfo) {
    const collators = Object.entries(networkInfo.nodesByName).filter(([name]) =>
        name.startsWith("collator")
    );
    for (const [name, { wsUri, userDefinedTypes }] of collators) {
        const api = await zombie.connect(wsUri, userDefinedTypes);
        const { methods } = await api.rpc.rpc.methods();
        if (methods.some((m) => m.toString() === ROTATE_RPC)) {
            return { name, wsUri, api };
        }
        await api.disconnect();
    }
    return null;
}

async function rotateAndRegister(name, wsUri, api) {
    const alice = new zombie.Keyring({ type: "sr25519" }).addFromUri("//Alice");
    const owner = "0x" + Buffer.from(alice.publicKey).toString("hex");

    // polkadot-js doesn't decorate this RPC, so call it through a raw provider.
    const provider = new zombie.WsProvider(wsUri);
    try {
        await provider.isReady;

        const { keys, proof } = await provider.send(ROTATE_RPC, [owner]);
        if (!proof) {
            console.log(`FAIL ${name} returned no proof: runtime lacks SessionKeys API v2`);
            return 0;
        }

        const badProof = proof.slice(0, -1) + (proof.endsWith("0") ? "1" : "0");
        const badResult = await submit(api, api.tx.session.setKeys(keys, badProof), alice);
        if (badResult !== "InvalidProof") {
            console.log(`FAIL tampered proof: expected InvalidProof, got ${badResult}`);
            return 0;
        }

        const result = await submit(api, api.tx.session.setKeys(keys, proof), alice);
        if (result !== null) {
            console.log(`FAIL setKeys with valid proof: ${result}`);
            return 0;
        }

        const next = await api.query.session.nextKeys(alice.address);
        if (next.isNone || next.unwrap().toHex() !== keys) {
            console.log(`FAIL session.nextKeys is ${next.toHex()}, expected ${keys}`);
            return 0;
        }

        if (!(await provider.send("author_hasSessionKeys", [keys]))) {
            console.log(`FAIL ${name} keystore lacks the rotated keys`);
            return 0;
        }

        console.log(`rotated session keys on ${name} and registered them with a proof`);
        return 1;
    } finally {
        await provider.disconnect();
    }
}

async function run(_nodeName, networkInfo) {
    const target = await connectToCollatorWithRpc(networkInfo);
    if (!target) {
        console.log(`FAIL no collator exposes ${ROTATE_RPC}: a stable2603+ client is required`);
        return 0;
    }

    const { name, wsUri, api } = target;
    try {
        return await rotateAndRegister(name, wsUri, api);
    } catch (err) {
        console.log(`FAIL ${err.message}`);
        return 0;
    } finally {
        await api.disconnect();
    }
}

module.exports = { run };
