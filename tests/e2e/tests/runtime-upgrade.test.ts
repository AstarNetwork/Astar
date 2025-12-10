import { afterAll, beforeAll, describe, it, expect } from 'vitest';
import { setupContext, testingPairs } from '@acala-network/chopsticks-testing';
import { readFileSync } from 'node:fs';
import path from 'node:path';

const endpoints = {
  shibuya: 'wss://rpc.shibuya.astar.network',
  shiden: 'wss://rpc.shiden.astar.network',
  astar: 'wss://rpc.astar.network',
};

describe('runtime upgrade', async () => {
  const { alice } = testingPairs();

  const runtime = process.env.RUNTIME || 'shibuya';
  const { api, dev, teardown } = await setupContext({ endpoint: endpoints[runtime], timeout: 300_000 });

  beforeAll(async () => {
    await dev.setStorage({
      System: {
        AuthorizedUpgrade: null,
        Account: [[[alice.address], { providers: 1, data: { free: 1000n * 10n ** 18n } }]],
      },
    });
  });

  afterAll(async () => {
    await teardown();
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

    const prevSpecVersion = api.runtimeVersion.specVersion.toNumber();
    console.log('SpecVersion before upgrade: ', prevSpecVersion);

    console.log(`Upgrading ${runtime} runtime...`);
    const codePath = path.join(
      __dirname,
      `../../../target/release/wbuild/${runtime}-runtime/${runtime}_runtime.compact.compressed.wasm`
    );
    const code = readFileSync(codePath);
    const codeHash = api.registry.hash(code);

    await dev.setStorage({
      System: {
        AuthorizedUpgrade: {
          code_hash: codeHash,
          check_version: true,
        },
      },
    });

    await api.tx.system
        .applyAuthorizedUpgrade('0x' + code.toString('hex'))
        .signAndSend(alice);

    // Do block production.
    await dev.newBlock({ count: 2 });
    // wait a bit for pjs/api to reflect runtimeVersion change
    await new Promise((r) => setTimeout(r, 1000));

    // The spec version is increased.
    const curSpecVersion = api.runtimeVersion.specVersion.toNumber();
    console.log('SpecVersion after upgrade: ', curSpecVersion);
    expect(curSpecVersion).toBeGreaterThan(prevSpecVersion);

    await afterUpgrade();
  });
});
