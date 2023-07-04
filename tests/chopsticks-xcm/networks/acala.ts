import { Config } from './types'

export type Vars = {
  relayToken: string
  relayLiquidToken: string
  stableToken: string
}

export default {
  polkadot: {
    name: 'acala' as const,
    endpoint: 'wss://acala-rpc-0.aca-api.network',
    relayToken: 'DOT',
    relayLiquidToken: 'LDOT',
    stableToken: 'AUSD',
  },
  kusama: {
    name: 'karura' as const,
    endpoint: 'wss://karura-rpc-0.aca-api.network',
    relayToken: 'KSM',
    relayLiquidToken: 'LKSM',
    stableToken: 'KUSD',
  },
  config: ({ alice, relayToken, relayLiquidToken, stableToken }) => ({
    storages: {
      System: {
        account: [[[alice.address], { data: { free: 10 * 1e12 } }]],
      },
      Tokens: {
        accounts: [
          [[alice.address, { Token: relayToken }], { free: 10 * 1e12 }],
          [[alice.address, { Token: relayLiquidToken }], { free: 100 * 1e12 }],
          [[alice.address, { Token: stableToken }], { free: 1000 * 1e12 }],
        ],
      },
      Sudo: {
        key: alice.address,
      },
      EvmAccounts: {
        accounts: [[['0x82a258cb20e2adb4788153cd5eb5839615ece9a0'], alice.address]],
        evmAddresses: [[[alice.address], '0x82a258cb20e2adb4788153cd5eb5839615ece9a0']],
      },
      Homa: {
        // avoid impact test outcome
        $removePrefix: ['redeemRequests', 'unbondings', 'toBondPool'],
        // so that bump era won't trigger unbond
        relayChainCurrentEra: '0x64000000',
      },
      PolkadotXcm: {
        // avoid sending xcm version change notifications to makes things faster
        $removePrefix: ['versionNotifyTargets', 'versionNotifiers', 'supportedVersion'],
      },
    },
  }),
} satisfies Config<Vars>

export const acala = {
  paraId: 2000,
  paraAccount: '13YMK2eYoAvStnzReuxBjMrAvPXmmdsURwZvc62PrdXimbNy',
  dot: { Token: 'DOT' },
  ldot: { Token: 'LDOT' },
  dai: { Erc20: '0x54a37a01cd75b616d63e0ab665bffdb0143c52ae' },
  wbtc: { ForeignAsset: 5 },
  ausd: { Token: 'AUSD' },
  aca: { Token: 'ACA' },
} as const

export const karura = {
  paraId: 2000,
  paraAccount: '13YMK2eYoAvStnzReuxBjMrAvPXmmdsURwZvc62PrdXimbNy',
  ksm: { Token: 'KSM' },
  lksm: { Token: 'LKSM' },
  usdt: { ForeignAsset: 7 },
  rmrk: { ForeignAsset: 0 },
  dai: { Erc20: '0x4bb6afb5fa2b07a5d1c499e1c3ddb5a15e709a71' },
  ausd: { Token: 'KUSD' },
  kar: { Token: 'KAR' },
} as const
