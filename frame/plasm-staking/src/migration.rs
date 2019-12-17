//! Storage migrations for plams-session.

#[cfg(not(feature = "migrate"))]
mod inner {
    pub(super) fn perform_migrations<T>() {}
}

/// Perform all necessary storage migrations to get storage into the expected stsate for current
/// logic. No-op if fully upgraded.
pub(crate) fn perform_migrations<T: crate::Trait>() {
    inner::perform_migrations::<T>();
}
