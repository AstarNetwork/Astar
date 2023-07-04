import { Config } from './types'

export default {
  polkadot: {
    name: 'polkadot' as const,
    endpoint: 'wss://rpc.polkadot.io',
  },
  kusama: {
    name: 'kusama' as const,
    endpoint: 'wss://kusama-rpc.polkadot.io',
  },
  config: ({ alice }) => ({
    storages: {
      System: {
        Account: [[[alice.address], { providers: 1, data: { free: 10 * 1e12 } }]],
      },
      ParasDisputes: {
        // those can makes block building super slow
        $removePrefix: ['disputes'],
      },
      Dmp: {
        // clear existing dmp to avoid impact test result
        $removePrefix: ['downwardMessageQueues'],
      },
    },
  }),
} satisfies Config

export const polkadot = {
  dot: { Concrete: { parents: 0, interior: 'Here' } },
} as const

export const kusama = {
  ksm: { Concrete: { parents: 0, interior: 'Here' } },
} as const
