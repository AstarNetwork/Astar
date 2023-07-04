import { query, tx } from '../helpers/api'

import { acala } from '../networks/acala'
import { astar } from '../networks/astar'
import { polkadot } from '../networks/polkadot'

import buildTest from './shared'

const tests = [
  // astar <-> polkadot

  {
    from: 'polkadot',
    to: 'astar',
    name: 'DOT',
    test: {
      xcmPalletDown: {
        tx: tx.xcmPallet.limitedReserveTransferAssetsV3(polkadot.dot, 1e12, tx.xcmPallet.parachainV3(0, astar.paraId)),
        balance: query.assets(astar.dot),
      },
    },
  },
] as const

export type TestType = (typeof tests)[number]

buildTest(tests)
