import { ApiPromise } from '@polkadot/api'

export const xtokens = {
  relaychainV2: (acc: any) => ({
    V1: {
      parents: 1,
      interior: {
        X1: {
          AccountId32: {
            network: 'Any',
            id: acc,
          },
        },
      },
    },
  }),
  relaychainV3: (acc: any) => ({
    V3: {
      parents: 1,
      interior: {
        X1: {
          AccountId32: {
            id: acc,
          },
        },
      },
    },
  }),
  parachainV2: (paraId: number) => (acc: any) => ({
    V1: {
      parents: 1,
      interior: {
        X2: [
          { Parachain: paraId },
          {
            AccountId32: {
              network: 'Any',
              id: acc,
            },
          },
        ],
      },
    },
  }),
  parachainAccountId20V2: (paraId: number) => (acc: any) => ({
    V1: {
      parents: 1,
      interior: {
        X2: [
          { Parachain: paraId },
          {
            AccountKey20: {
              network: 'Any',
              key: acc,
            },
          },
        ],
      },
    },
  }),
  parachainV3: (paraId: number) => (acc: any) => ({
    V3: {
      parents: 1,
      interior: {
        X2: [
          { Parachain: paraId },
          {
            AccountId32: {
              id: acc,
            },
          },
        ],
      },
    },
  }),
  transfer:
    (token: any, amount: any, dest: (dest: any) => any, weight: any = 'Unlimited') =>
    ({ api }: { api: ApiPromise }, acc: any) =>
      api.tx.xtokens.transfer(token, amount, dest(acc), weight),

  transferMultiasset:
  (asset: any, amount: any, dest: (dest: any) => any, weight: any = 'Unlimited') =>
  ({ api }: { api: ApiPromise }, acc: any) =>
    api.tx.xtokens.transferMultiasset(
      {
        V3: 
           {
            Concrete : { parents: 1, interior: { X3: [ 
              { Parachain : 1000 },
              {
              PalletInstance: 50
              },
              {
              GeneralIndex : 1984
              }
            ]}
          },
          Fungible : {
            Funglible : amount
        },
        }
      }, 
      dest(acc), 
      weight),

  transferMulticurrencies:
    (token: any, amount: any, feeToken: any, feeAmount: any, dest: (dest: any) => any) =>
    ({ api }: { api: ApiPromise }, acc: any) =>
      api.tx.xTokens.transferMulticurrencies(
        [
          [token, amount],
          [feeToken, feeAmount],
        ],
        1,
        dest(acc),
        'Unlimited'
      ),
}

export const xcmPallet = {
  parachainV2: (parents: number, paraId: number) => ({
    V1: {
      parents,
      interior: {
        X1: { Parachain: paraId },
      },
    },
  }),
  relaychainV3: (acc: any) => ({
    V3: {
      parents: 1,
      interior: {
        X1: {
          AccountId32: {
            network: 'Any',
            id: acc,
          },
        },
      },
    },
  }),
  parachainV3: (parents: number, paraId: any) => ({
    V3: {
      parents,
      interior: {
        X1: { Parachain: paraId },
      },
    },
  }),
  limitedReserveTransferAssetsV2:
    (token: any, amount: any, dest: any) =>
    ({ api }: { api: ApiPromise }, acc: any) =>
      (api.tx.xcmPallet || api.tx.polkadotXcm).limitedReserveTransferAssets(
        dest,
        {
          V1: {
            parents: 0,
            interior: {
              X1: {
                AccountId32: {
                  network: 'Any',
                  id: acc,
                },
              },
            },
          },
        },
        {
          V1: [
            {
              id: token,
              fun: { Fungible: amount },
            },
          ],
        },
        0,
        'Unlimited'
      ),
  limitedReserveTransferAssetsV3:
    (token: any, amount: any, dest: any) =>
    ({ api }: { api: ApiPromise }, acc: any) =>
      (api.tx.xcmPallet || api.tx.polkadotXcm).limitedReserveTransferAssets(
        dest,
        {
          V3: {
            parents: 0,
            interior: {
              X1: {
                AccountId32: {
                  id: acc,
                },
              },
            },
          },
        },
        {
          V3: [
            {
              id: token,
              fun: { Fungible: amount },
            },
          ],
        },
        0,
        'Unlimited'
      ),
}

export const tx = {
  xtokens,
  xcmPallet,
}

export const query = {
  balances: ({ api }: { api: ApiPromise }, address: string) => api.query.system.account(address),
  tokens:
    (token: any) =>
    ({ api }: { api: ApiPromise }, address: string) =>
      api.query.tokens.accounts(address, token),
  assets:
    (token: number | bigint) =>
    ({ api }: { api: ApiPromise }, address: string) =>
      api.query.assets.account(token, address),
  evm:
    (contract: string, slot: string) =>
    ({ api }: { api: ApiPromise }, _address: string) =>
      api.query.evm.accountStorages(contract, slot),
}
