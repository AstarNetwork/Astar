import { Config } from './types'
import acalaConfig from './acala'
import astarConfig from './astar'
import moonbeamConfig from './moonbeam'
import polkadotConfig from './polkadot'
import statemintConfig from './statemint'

const all = {
  polkadot: polkadotConfig,
  statemint: statemintConfig,
  acala: acalaConfig,
  astar: astarConfig,
  moonbeam: moonbeamConfig,
} satisfies Record<string, Config>

export default all
