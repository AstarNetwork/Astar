// TODO: document this and sort it out
pub trait IsContract: Default {
    /// Used to check whether the struct represents a valid contract or not.
    fn is_valid(&self) -> bool;
}
