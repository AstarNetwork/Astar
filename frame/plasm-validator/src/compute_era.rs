//! The ComputeEraOnModule is customizable compute era.
//! This will change depending on the timing of the chain.
use super::*;

pub trait ComputeEraOnModule<Param> {
    fn compute(era: &EraIndex) -> Param;
}

/// This is first validator rewards algorithm.
impl<T: Trait> ComputeEraOnModule<u32> for Module<T> {
    fn compute(era: &EraIndex) -> u32 {
        match <ElectedValidators<T>>::get(era) {
            Some(validators) => validators.len() as u32,
            None => 0,
        }
    }
}
