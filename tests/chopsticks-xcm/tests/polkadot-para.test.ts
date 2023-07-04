import { Context } from '../networks/types'
import { query, tx } from '../helpers/api'

import { acala } from '../networks/acala'
import { hydraDX } from '../networks/hydraDX'
import { moonbeam } from '../networks/moonbeam'
import { statemint } from '../networks/statemint'
import { astar } from '../networks/astar'

import buildTest from './shared'


const tests = [
  // statemint <-> astar
  {
    from: 'statemint',
    to: 'astar',
    name: 'USDT',
    fromStorage: ({ alice }: Context) => ({
      System: {
        account: [[[astar.paraAccount], { data: { free: 10e10 } }]],
      },
      Assets: {
        account: [[[statemint.usdtIndex, alice.address], { balance: 1e8 }]],
        asset: [[[statemint.usdtIndex], { supply: 1e8 }]],
      },
    }),
    test: {
      xcmPalletHorizontal: {
        tx: tx.xcmPallet.limitedReserveTransferAssetsV3(statemint.usdt, 1e7, tx.xcmPallet.parachainV3(1, astar.paraId)),
        fromBalance: query.assets(statemint.usdtIndex),
        toBalance: query.assets(astar.usdt),
      },
    },
  },
] as const

export type TestType = (typeof tests)[number]

buildTest(tests)
