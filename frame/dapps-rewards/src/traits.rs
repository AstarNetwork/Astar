use super::*;

pub trait OnDistributeRewards<AccountId, Balance> {
    fn on_ditribute_rewards(dapps: Vec<AccountId>, rewards: Balance);
}
