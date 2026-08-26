// post_upgrade.js
//
// Argument: "<minBlocksAfterUpgrade>"
//
// Waits for `minBlocksAfterUpgrade` blocks to be produced after the block that
// emitted `system.CodeUpdated` (zombienet runs a js-script only once, so the
// waiting has to happen in here), then passes (returns 1) iff:
//   * at least `minBlocksAfterUpgrade` blocks have been produced,
//   * every registered aura authority has authored at least one of those
//     blocks (so single-node authoring after the upgrade fails the check),
//   * aura slots are strictly increasing across the post-upgrade range.
//
// Anything else returns 0.

// Leave a margin below the 450s zndsl timeout so we can report a real failure
// instead of being killed mid-wait.
const WAIT_TIMEOUT_MS = 400_000;
const POLL_INTERVAL_MS = 3_000;

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function run(nodeName, networkInfo, args) {
    const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    const minBlocks = Number(args[0]);

    let head = (await api.rpc.chain.getHeader()).number.toNumber();

    // Walk back to the CodeUpdated block (bounded lookback).
    let upgradeAt = null;
    for (let n = head; n >= Math.max(0, head - 300); n--) {
        const hash = await api.rpc.chain.getBlockHash(n);
        const at = await api.at(hash);
        const evs = await at.query.system.events();
        if (evs.some((r) => api.events.system.CodeUpdated.is(r.event))) {
            upgradeAt = n;
            break;
        }
    }
    if (upgradeAt === null) {
        console.log("FAIL no CodeUpdated event in last 300 blocks");
        return 0;
    }

    // Wait for enough blocks on top of the upgrade before judging authorship.
    const deadline = Date.now() + WAIT_TIMEOUT_MS;
    while (head - upgradeAt < minBlocks) {
        if (Date.now() >= deadline) {
            console.log(
                `FAIL only ${head - upgradeAt}/${minBlocks} blocks since CodeUpdated at #${upgradeAt}`
            );
            return 0;
        }
        console.log(
            `WAIT ${head - upgradeAt}/${minBlocks} blocks since CodeUpdated at #${upgradeAt}`
        );
        await sleep(POLL_INTERVAL_MS);
        head = (await api.rpc.chain.getHeader()).number.toNumber();
    }

    const authorities = await api.query.aura.authorities();
    const nAuth = authorities.length;
    const seen = new Set();
    let prevSlot = null;

    for (let n = upgradeAt; n <= head; n++) {
        const hash = await api.rpc.chain.getBlockHash(n);
        const header = await api.rpc.chain.getHeader(hash);
        const pre = header.digest.logs.find(
            (l) =>
                l.isPreRuntime &&
                Buffer.from(l.asPreRuntime[0]).toString() === "aura"
        );
        if (!pre) {
            console.log(`FAIL #${n} missing aura pre-digest`);
            return 0;
        }
        const slot = Buffer.from(pre.asPreRuntime[1]).readBigUInt64LE(0);
        if (prevSlot !== null && slot <= prevSlot) {
            console.log(`FAIL #${n} slot regressed (${slot} <= ${prevSlot})`);
            return 0;
        }
        prevSlot = slot;
        seen.add(Number(slot % BigInt(nAuth)));
    }

    const authored = [...seen].sort((a, b) => a - b);
    console.log(
        `upgradeAt=${upgradeAt} head=${head} ` +
            `authored=[${authored.join(",")}]/${nAuth}`
    );
    return seen.size === nAuth ? 1 : 0;
}

module.exports = { run };
