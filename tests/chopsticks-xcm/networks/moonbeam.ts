import { Config } from './types'

export default {
  polkadot: {
    name: 'moonbeam' as const,
    endpoint: 'wss://wss.api.moonbeam.network',
  },
  kusama: {
    name: 'moonriver' as const,
    endpoint: 'wss://wss.api.moonriver.moonbeam.network',
  },
  config: ({ alith }) => ({
    storages: {
      System: {
        Account: [[[alith.address], { data: { free: 1000n * 10n ** 18n } }]],
      },
      AuthorFilter: {
        EligibleRatio: 100,
        EligibleCount: 100,
      },
    },
  }),
} satisfies Config

export const moonbeam = {
  paraId: 2004,
  dot: 42259045809535163221576417993425387648n,
} as const

export const moonriver = {
  paraId: 2023,
} as const
