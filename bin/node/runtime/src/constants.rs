//! A set of constant values used in substrate runtime.

/// Money matters.
pub mod currency {
    use plasm_primitives::Balance;

    pub const MILLIPLM: Balance = 1_000_000_000_000;
    pub const PLM: Balance = 1_000 * MILLIPLM;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 150 * MILLIPLM + (bytes as Balance) * 60 * MILLIPLM
    }
}

/// Time constants.
pub mod time {
    use plasm_primitives::{BlockNumber, Moment};

    pub const MILLISECS_PER_BLOCK: Moment = 10000;
    pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

    // These time units are defined in number of blocks.
    pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}
