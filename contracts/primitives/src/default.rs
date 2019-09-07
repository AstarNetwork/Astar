//! Default type parameters.

pub type RangeNumber = u128;

pub type Range = super::Range<RangeNumber>;
pub type StateObject<T> = super::StateObject<T>;
pub type StateUpdate<T> = super::StateUpdate<T, RangeNumber>;
pub type Checkpoint<T> = super::Checkpoint<T, RangeNumber>;
pub type Transaction<T> = super::Transaction<T, RangeNumber>;
pub type Challenge<T> = super::Challenge<T, RangeNumber>;
