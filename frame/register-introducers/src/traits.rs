pub trait RegisteredIntroducersChecker<AccountId> {
    fn is_registered(account_id: &AccountId) -> bool;
}
