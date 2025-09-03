use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};

#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum MigrationStep {
    NotStarted,
    Ongoing,
    Finished,
}

impl Default for MigrationStep {
    fn default() -> Self {
        MigrationStep::NotStarted
    }
}
