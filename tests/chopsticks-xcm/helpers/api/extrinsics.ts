import { AccountInfo } from '@polkadot/types/interfaces'
import { ApiPromise } from '@polkadot/api'
import { SubmittableExtrinsic } from '@polkadot/api-base/types'

export const balance = async (api: ApiPromise, address: string) => {
  const account = await api.query.system.account<AccountInfo>(address)
  return account.data.toJSON()
}

export const xTokensForRelayChain = (api: ApiPromise, amount: string, address: Uint8Array) => {
  return api.tx.xTokens.transfer(
    {
      Token: 'KSM',
    },
    amount,
    {
      V1: {
        parents: 1,
        interior: {
          X1: {
            AccountId32: {
              network: 'Any',
              id: address,
            },
          },
        },
      },
    },
    'Unlimited'
  )
}

export const xTokensForParaChain = (
  api: ApiPromise,
  token: object,
  parachainId: string,
  amount: string,
  address: Uint8Array
) => {
  return api.tx.xTokens.transfer(
    token,
    amount,
    {
      V1: {
        parents: 1,
        interior: {
          X2: [
            {
              Parachain: parachainId,
            },
            {
              AccountId32: {
                network: 'Any',
                id: address,
              },
            },
          ],
        },
      },
    },
    'Unlimited'
  )
}

export const xTokens = (
  api: ApiPromise,
  isRelayChain: boolean,
  parachainId: string,
  token: object,
  amount: string | bigint,
  address: Uint8Array
) => {
  const multiLocation = isRelayChain
    ? { X1: { AccountId32: { network: 'Any', id: address } } }
    : {
        X2: [
          {
            Parachain: parachainId,
          },
          {
            AccountId32: {
              network: 'Any',
              id: address,
            },
          },
        ],
      }

  return api.tx.xTokens.transfer(
    token,
    amount,
    {
      V1: {
        parents: 1,
        interior: multiLocation,
      },
    },
    'Unlimited'
  )
}

export const xTokensV3 = (
  api: ApiPromise,
  isRelayChain: boolean,
  parachainId: string,
  token: object,
  amount: string | bigint,
  address: Uint8Array
) => {
  const multiLocation = isRelayChain
    ? {
        X1: {
          AccountId32: {
            id: address,
          },
        },
      }
    : {
        X2: [
          {
            Parachain: parachainId,
          },
          {
            AccountId32: {
              id: address,
            },
          },
        ],
      }

  return api.tx.xTokens.transfer(
    token,
    amount,
    {
      V3: {
        parents: 1,
        interior: multiLocation,
      },
    } as any,
    'Unlimited'
  )
}

export const relayChainV3limitedReserveTransferAssets = (
  api: ApiPromise,
  parachainId: string,
  amount: string,
  address: Uint8Array
) => {
  return api.tx.xcmPallet.limitedReserveTransferAssets(
    {
      V3: {
        parents: 0,
        interior: {
          X1: { Parachain: parachainId },
        },
      },
    },
    {
      V3: {
        parents: 0,
        interior: {
          X1: {
            AccountId32: {
              id: address,
            },
          },
        },
      },
    },
    {
      V3: [
        {
          id: { Concrete: { parents: 0, interior: 'Here' } },
          fun: { Fungible: amount },
        },
      ],
    },
    0,
    'Unlimited'
  )
}

export const xTokensTransferMulticurrencies = (
  api: ApiPromise,
  foreignAssetId: string,
  amount: string,
  parachainId: string,
  address: Uint8Array
) => {
  return api.tx.xTokens.transferMulticurrencies(
    [
      [
        {
          ForeignAsset: foreignAssetId,
        },
        amount,
      ],
      [
        {
          Token: 'KSM',
        },
        '16000000000',
      ],
    ],
    '1',
    {
      V1: {
        parents: 1,
        interior: {
          X2: [
            {
              Parachain: parachainId,
            },
            {
              AccountId32: {
                network: 'Any',
                id: address,
              },
            },
          ],
        },
      },
    },
    'Unlimited'
  )
}

export const xTokensTransferTransferMultiasset = (
  api: ApiPromise,
  interior: any[],
  amount: string,
  parachainId: string,
  address: Uint8Array
) => {
  return api.tx.xTokens.transferMultiasset(
    {
      V1: {
        fun: {
          Fungible: amount,
        },
        id: {
          Concrete: {
            parents: 1,
            interior: {
              X3: interior,
              // X3: [{ Parachain: 1000 }, { PalletInstance: 50 }, { GeneralIndex: 1984 }],
            },
          },
        },
      },
    },
    {
      V1: {
        parents: 1,
        interior: {
          X2: [
            {
              Parachain: parachainId,
            },
            {
              AccountId32: {
                network: 'Any',
                id: address,
              },
            },
          ],
        },
      },
    },
    'Unlimited'
  )
}

export const xTokensTransferTransferMultiassetV3 = (
  api: ApiPromise,
  interior: any[],
  amount: string,
  parachainId: string,
  address: Uint8Array
) => {
  return api.tx.xTokens.transferMultiasset(
    {
      V3: {
        fun: {
          Fungible: amount,
        },
        id: {
          Concrete: {
            parents: 1,
            interior: {
              X3: interior,
              // X3: [{ Parachain: 1000 }, { PalletInstance: 50 }, { GeneralIndex: 1984 }],
            },
          },
        },
      },
    },
    {
      V3: {
        parents: 1,
        interior: {
          X2: [
            {
              Parachain: parachainId,
            },
            {
              AccountId32: {
                id: address,
              },
            },
          ],
        },
      },
    } as any,
    'Unlimited'
  )
}

export const xTokensTransferMulticurrenciesV3 = (
  api: ApiPromise,
  foreignAssetId: string,
  amount: string,
  parachainId: string,
  address: Uint8Array
) => {
  return api.tx.xTokens.transferMulticurrencies(
    [
      [
        {
          ForeignAsset: foreignAssetId,
        },
        amount,
      ],
      [
        {
          Token: 'KSM',
        },
        '16000000000',
      ],
    ],
    '1',
    {
      V3: {
        parents: 1,
        interior: {
          X2: [
            {
              Parachain: parachainId,
            },
            {
              AccountId32: {
                id: address,
              },
            },
          ],
        },
      },
    } as any,
    'Unlimited'
  )
}

export const closeLoanHasDebitByDex = (api: ApiPromise, token: string, max_collateral_amount: string) => {
  return api.tx.honzon.closeLoanHasDebitByDex({ Token: token }, max_collateral_amount)
}

export const mint = (api: ApiPromise, amount: string) => {
  return api.tx.homa.mint(amount)
}

export const transactionPaymentWithFeeCurrency = (
  api: ApiPromise,
  currencyId: object,
  call: SubmittableExtrinsic<'promise'>
) => {
  return api.tx.transactionPayment.withFeeCurrency(currencyId, call)
}

export const sudo = (api: ApiPromise, call: SubmittableExtrinsic<'promise'>) => {
  return api.tx.sudo.sudo(call)
}
