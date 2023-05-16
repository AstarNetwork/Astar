import { afterAll, beforeAll, describe, it, expect } from 'vitest';
import { setupContext, testingPairs } from '@acala-network/chopsticks-testing';
import { readFileSync } from 'node:fs';
import path from 'node:path';

const endpoints = {
  shibuya: 'wss://shibuya-rpc.dwellir.com',
  shiden: 'wss://shiden.api.onfinality.io/public-ws',
  astar: 'wss://astar.api.onfinality.io/public-ws',
};

const specVersion = async (api) => {
  const version = await api.rpc.state.getRuntimeVersion();
  return version.specVersion.toNumber();
};

describe('runtime upgrade', async () => {
  const { alice } = testingPairs();

  const runtime = process.env.RUNTIME || 'shibuya';
  const cxt = await setupContext({ endpoint: endpoints[runtime] });
  const { api, dev } = cxt;

  beforeAll(async () => {
    await dev.setStorage({
      Sudo: {
        Key: alice.address,
      },
      System: {
        Account: [[[alice.address], { data: { free: 100n * 10n ** 18n } }]],
      },
    });
  });

  afterAll(async () => {
    await cxt.teardown();
  });

  // Execution hook before runtime upgrade. To test storage migrations, set up the storage items
  // via transactions, set storage etc.
  const beforeUpgrade = async () => {};

  // Execution hook after runtime upgrade. To verify storage migrations work, query the migrated
  // storage items or send transactions that interact with them.
  const afterUpgrade = async () => {
    // Dummy test. Change it to test your storage migrations.
    let isFinalized = false;
    let unsub = await api.tx.system
      .remark('Hello World')
      .signAndSend(alice, (result) => {
        if (result.status.isFinalized) {
          isFinalized = true;
          unsub();
        }
      });
    await dev.newBlock();
    await new Promise((resolve) => setTimeout(resolve, 2000));
    expect(isFinalized).toBe(true);
  };

  it('runtime upgrade works', async () => {
    await beforeUpgrade();

    const prevSpecVersion = await specVersion(api);
    console.log('SpecVersion before upgrade: ', prevSpecVersion);

    console.log(`Upgrading ${runtime} runtime...`);
    const codePath = path.join(
      __dirname,
      `../../../target/release/wbuild/${runtime}-runtime/${runtime}_runtime.compact.compressed.wasm`
    );
    const code = readFileSync(codePath);
    await api.tx.sudo
      .sudoUncheckedWeight(
        api.tx.system.setCode('0x' + code.toString('hex')),
        0
      )
      .signAndSend(alice);

    // Do block production.
    await dev.newBlock({ count: 2 });

    // The spec version is increased.
    const curSpecVersion = await specVersion(api);
    console.log('SpecVersion before upgrade: ', curSpecVersion);
    expect(curSpecVersion).toBeGreaterThan(prevSpecVersion);

    await afterUpgrade();
  });
});
