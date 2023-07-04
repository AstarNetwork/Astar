import { query, tx } from '../helpers/api'

import { karura } from '../networks/acala'
import { kusama } from '../networks/polkadot'
import { shiden } from '../networks/astar'

import buildTest from './shared'

const tests = [
  // shiden <-> kusama
  {
    from: 'shiden',
    to: 'kusama',
    name: 'KSM',
    test: {
      xtokensUp: {
        tx: tx.xtokens.transfer(shiden.ksm, 1e12, tx.xtokens.relaychainV3),
        balance: query.assets(shiden.ksm),
      },
    },
  },
  {
    from: 'kusama',
    to: 'shiden',
    name: 'KSM',
    test: {
      xcmPalletDown: {
        tx: tx.xcmPallet.limitedReserveTransferAssetsV3(shiden.ksm, 1e12, tx.xcmPallet.parachainV3(0, shiden.paraId)),
        balance: query.assets(shiden.ksm),
      },
    },
  },
] as const

export type TestType = (typeof tests)[number]

buildTest(tests)
