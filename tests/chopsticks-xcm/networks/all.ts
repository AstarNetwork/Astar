import { Config } from './types'
import acalaConfig from './acala'
import astarConfig from './astar'
import bifrostConfig from './bifrost'
import centrifugeConfig from './centrifuge'
import hydraDXConfig from './hydraDX'
import moonbeamConfig from './moonbeam'
import parallelConfig from './parallel'
import polkadotConfig from './polkadot'
import statemintConfig from './statemint'

const all = {
  polkadot: polkadotConfig,
  statemint: statemintConfig,
  acala: acalaConfig,
  astar: astarConfig,
  moonbeam: moonbeamConfig,
  hydraDX: hydraDXConfig,
  bifrost: bifrostConfig,
  centrifuge: centrifugeConfig,
  parallel: parallelConfig,
} satisfies Record<string, Config>

export default all
