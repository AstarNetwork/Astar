import { SetupOption } from '@acala-network/chopsticks-testing'
import { testingPairs } from '../helpers'

export type NetworkKind = 'polkadot' | 'kusama'

export type NetworkConfig = {
  name: string
  endpoint: string
}

export type Context = ReturnType<typeof testingPairs>

export type FullContext = Context &
  NetworkConfig & {
    network: NetworkKind
  }

export type Config<T = object> = {
  polkadot?: NetworkConfig & T
  kusama?: NetworkConfig & T
  config(context: FullContext & T): {
    storages?: Record<string, unknown>
    options?: Partial<SetupOption>
  }
}
