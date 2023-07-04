import { Context } from '../networks/types'
import { query, tx } from '../helpers/api'

import { basilisk } from '../networks/hydraDX'
import { shiden } from '../networks/astar'
import { statemine } from '../networks/statemint'

import buildTest from './shared'
import acala, { karura } from '../networks/acala'

const tests = [
  // statemine <-> shiden
  {
    from: 'statemine',
    to: 'shiden',
    name: 'USDT',
    test: {
      xcmPalletHorizontal: {
        tx: tx.xcmPallet.limitedReserveTransferAssetsV3(
          statemine.usdt,
          1e6,
          tx.xcmPallet.parachainV3(1, shiden.paraId)
        ),
        fromBalance: query.assets(statemine.usdtIndex),
        toBalance: query.assets(shiden.usdt),
      },
    },
  },
  {
    from: 'shiden',
    to: 'statemine',
    name: 'USDT',
    fromStorage: ({ alice }: Context) => ({
      Assets: {
        account: [[[shiden.usdt, alice.address], { balance: 10e8 }]],
      },
    }),
    test: {
      xtokenstHorizontal: {
        tx: tx.xtokens.transfer(shiden.usdt, 1e8, tx.xtokens.parachainV3(statemine.paraId)),
        fromBalance: query.assets(shiden.usdt),
        toBalance: query.assets(statemine.usdtIndex),
      },
    },
  },
] as const

export type TestType = (typeof tests)[number]

buildTest(tests)
