//! Backward compatibility layer.

use sp_std::prelude::*;

/// Interface for accessing the storage from within the runtime.
/// * Removed in substrate >2.0.0-alpha.6 but used in Plasm Mainnet genesis.
#[sp_runtime_interface::runtime_interface]
pub trait Storage {
    /// Read child key.
    fn child_get(
        &self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
        key: &[u8],
    ) -> Option<Vec<u8>> {
        sp_io::default_child_storage::get(storage_key, key)
    }

    /// Read child key.
    fn child_read(
        &self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
        key: &[u8],
        value_out: &mut [u8],
        value_offset: u32,
    ) -> Option<u32> {
        sp_io::default_child_storage::read(storage_key, key, value_out, value_offset)
    }

    /// Set a child storage value.
    fn child_set(
        &mut self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
        key: &[u8],
        value: &[u8],
    ) {
        sp_io::default_child_storage::set(storage_key, key, value)
    }

    /// Remove child key value.
    fn child_clear(
        &mut self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
        key: &[u8],
    ) {
        sp_io::default_child_storage::clear(storage_key, key)
    }

    /// Remove all child storage values.
    fn child_storage_kill(
        &mut self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
    ) {
        sp_io::default_child_storage::storage_kill(storage_key)
    }

    /// Check a child storage key.
    fn child_exists(
        &self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
        key: &[u8],
    ) -> bool {
        sp_io::default_child_storage::exists(storage_key, key)
    }

    /// Clear child key by prefix.
    fn child_clear_prefix(
        &mut self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
        prefix: &[u8],
    ) {
        sp_io::default_child_storage::clear_prefix(storage_key, prefix)
    }

    /// Child trie root calcualation.
    fn child_root(&mut self, storage_key: &[u8]) -> Vec<u8> {
        sp_io::default_child_storage::root(storage_key)
    }

    /// Child storage key iteration.
    fn child_next_key(
        &mut self,
        storage_key: &[u8],
        _child_definition: &[u8],
        _child_type: u32,
        key: &[u8],
    ) -> Option<Vec<u8>> {
        sp_io::default_child_storage::next_key(storage_key, key)
    }
}
