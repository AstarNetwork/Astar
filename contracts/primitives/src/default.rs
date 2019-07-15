pub type BlockNumber = u128;
pub type Range = super::Range<BlockNumber>;
pub type StateObject<T> = super::StateObject<T>;
pub type StateUpdate<T> = super::StateUpdate<T, BlockNumber>;
pub type Checkpoint<T> = super::Checkpoint<T, BlockNumber>;
pub type Transaction<T> = super::Transaction<T, BlockNumber>;
