import { ApiPromise } from '@polkadot/api'

export const queryBalance = (api: ApiPromise, address: string) => {
  return api.query.system.account(address)
}

export const queryTokenBalance = (api: ApiPromise, token: object, address: string) => {
  return api.query.tokens.accounts(address, token)
}

export const queryRedeemRequests = (api: ApiPromise, address: string) => {
  return api.query.homa.redeemRequests(address)
}

export const queryPositions = (api: ApiPromise, token: string, address: string) => {
  return api.query.loans.positions({ Token: token }, address)
}

export const querySharesAndWithdrawnRewards = (api: ApiPromise, poolsId: object, address: string) => {
  return api.query.rewards.sharesAndWithdrawnRewards(poolsId, address)
}
