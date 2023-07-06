import { beforeEach, describe, it, afterAll } from 'vitest'
import { sendTransaction } from '@acala-network/chopsticks-testing'

import { Network, NetworkNames, createContext, createNetworks } from '../networks'
import { check, checkEvents, checkHrmp, checkSystemEvents, checkUmp } from '../helpers'

import type { TestType as KusamaParaTestType } from './kusama-para.test'
import type { TestType as KusamaRelayTestType } from './kusama-relay.test'
import type { TestType as PolkadotParaTestType } from './polkadot-para.test'
import type { TestType as PolkadotRelayTestType } from './polkadot-relay.test'

type TestType = KusamaParaTestType
  | KusamaRelayTestType
  | PolkadotRelayTestType
  | PolkadotParaTestType


export default function buildTest(tests: ReadonlyArray<TestType>) {
  describe.each(tests)('$from -> $to xcm transfer $name', async ({ from, to, test, ...opt }) => {
    let fromChain: Network
    let toChain: Network
    let routeChain: Network

    const ctx = createContext()
    const { alice } = ctx

    let fromAccount = alice
    if ('fromAccount' in opt) {
      fromAccount = opt.fromAccount(ctx)
    }

    let toAccount = alice
    if ('toAccount' in opt) {
      toAccount = opt.toAccount(ctx)
    }

    let precision = 3
    if ('precision' in opt) {
      precision = opt.precision
    }

    beforeEach(async () => {
      const networkOptions = {
        [from]: undefined,
        [to]: undefined,
      } as Record<NetworkNames, undefined>
      if ('route' in opt) {
        networkOptions[opt.route] = undefined
      }
      const chains = await createNetworks(networkOptions, ctx)

      fromChain = chains[from]
      toChain = chains[to]
      if ('route' in opt) {
        routeChain = chains[opt.route]
      }

      if ('fromStorage' in opt) {
        const override = typeof opt.fromStorage === 'function' ? opt.fromStorage(ctx) : opt.fromStorage
        await fromChain.dev.setStorage(override)
      }

      if ('toStorage' in opt) {
        const override = typeof opt.toStorage === 'function' ? opt.toStorage(ctx) : opt.toStorage
        await toChain.dev.setStorage(override)
      }

      return async () => {
        await toChain.teardown()
        await fromChain.teardown()
        if (routeChain) {
          await routeChain.teardown()
        }
      }
    })

    if ('xtokensUp' in test) {
      const { balance, tx } = test.xtokensUp

      it('xtokens transfer', async () => {
        const tx0 = await sendTransaction(tx(fromChain, toAccount.addressRaw).signAsync(fromAccount))

        await fromChain.chain.newBlock()

        await check(balance(fromChain, fromAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on from chain')
        await checkEvents(tx0, 'xTokens').redact({ number: precision }).toMatchSnapshot('tx events')
        await checkUmp(fromChain).toMatchSnapshot('from chain ump messages')

        await toChain.chain.newBlock()

        await check(toChain.api.query.system.account(toAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on to chain')
        await checkSystemEvents(toChain, 'ump', 'messageQueue').toMatchSnapshot('to chain ump events')
      })
    }

    if ('xcmPalletDown' in test) {
      const { balance, tx } = test.xcmPalletDown

      it('xcmPallet transfer', async () => {
        const tx0 = await sendTransaction(tx(fromChain, toAccount.addressRaw).signAsync(fromAccount))

        await fromChain.chain.newBlock()

        await check(fromChain.api.query.system.account(fromAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on from chain')
        await checkEvents(tx0, 'xcmPallet').redact({ number: precision }).toMatchSnapshot('tx events')

        await toChain.chain.newBlock()

        await check(balance(toChain, toAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on to chain')
        await checkSystemEvents(toChain, 'parachainSystem', 'dmpQueue').toMatchSnapshot('to chain dmp events')
      })
    }

    if ('xcmPalletUp' in test) {
      const { balance, tx } = test.xcmPalletUp

      it('xcmPallet transfer', async () => {
        const tx0 = await sendTransaction(tx(fromChain, toAccount.addressRaw).signAsync(fromAccount))

        await fromChain.chain.newBlock()

      //   await check(balance(fromChain, fromAccount.address))
      //   .redact({ number: precision })
      //   .toMatchSnapshot('balance on from chain')
      //   await checkEvents(tx0, 'xcmPallet').redact({ number: precision }).toMatchSnapshot('tx events')
      //   await checkUmp(fromChain).toMatchSnapshot('from chain ump messages')

      //   await toChain.chain.newBlock()

      //   await check(toChain.api.query.system.account(toAccount.address))
      //     .redact({ number: precision })
      //     .toMatchSnapshot('balance on to chain')
      //     await checkSystemEvents(toChain, 'ump', 'messageQueue').toMatchSnapshot('to chain ump events')
      })
    }

    if ('xcmPalletHorizontal' in test) {
      const { fromBalance, toBalance, tx, ...testOpt } = test.xcmPalletHorizontal

      it('xcmPallet transfer', async () => {
        const tx0 = await sendTransaction(tx(fromChain, toAccount.addressRaw).signAsync(fromAccount))

        await fromChain.chain.newBlock()

        await check(fromBalance(fromChain, fromAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on from chain')
        await checkEvents(tx0, 'polkadotXcm').redact({ number: precision }).toMatchSnapshot('tx events')

        if ('checkUmp' in testOpt) {
          await checkUmp(fromChain).toMatchSnapshot('from chain ump messages')
        } else {
          await checkHrmp(fromChain).toMatchSnapshot('from chain hrmp messages')
        }

        if (routeChain) {
          await routeChain.chain.newBlock()
        }
        await toChain.chain.newBlock()

        await check(toBalance(toChain, toAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on to chain')
        await checkSystemEvents(toChain, 'xcmpQueue', 'dmpQueue').toMatchSnapshot('to chain xcm events')
      })
    }

    if ('xtokenstHorizontal' in test) {
      const { fromBalance, toBalance, tx, ...testOpt } = test.xtokenstHorizontal

      it('xtokens transfer', async () => {
        const txx = tx(fromChain, toAccount.addressRaw)
        const tx0 = await sendTransaction(txx.signAsync(fromAccount))

        await fromChain.chain.newBlock()

        await check(fromBalance(fromChain, fromAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on from chain')
        await checkEvents(tx0, 'xTokens').toMatchSnapshot('tx events')

        if ('checkUmp' in testOpt) {
          await checkUmp(fromChain).toMatchSnapshot('from chain ump messages')
        } else {
          await checkHrmp(fromChain).toMatchSnapshot('from chain hrmp messages')
        }

        if (routeChain) {
          await routeChain.chain.newBlock()
        }
        await toChain.chain.newBlock()

        await check(toBalance(toChain, toAccount.address))
          .redact({ number: precision })
          .toMatchSnapshot('balance on to chain')
        await checkSystemEvents(toChain, 'xcmpQueue', 'dmpQueue').toMatchSnapshot('to chain xcm events')
      })
    }
  })
}
