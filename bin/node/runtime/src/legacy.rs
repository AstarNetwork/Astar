//! Backward compatibility layer. 

use sp_core::storage::{ChildInfo, ChildType};
use sp_std::prelude::*;

fn deprecated_storage_key_prefix_check(storage_key: &[u8]) {
    let prefix = ChildType::ParentKeyId.parent_prefix();
    if !storage_key.starts_with(prefix) {
        panic!("Invalid storage key");
    }
}

fn resolve_child_info(child_type: u32, storage_key: &[u8]) -> Option<ChildInfo> {
    match ChildType::new(child_type) {
        Some(ChildType::ParentKeyId) => {
            Some(ChildInfo::new_default(storage_key))
        },
        None => None,
    }
}

/// Interface for accessing the storage from within the runtime.
/// * Removed in substrate >2.0.0-alpha.6 but used in Plasm Mainnet genesis.
#[sp_runtime_interface::runtime_interface]
pub trait Storage {
    /// Read child key.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::get`).
	fn child_get(
		&self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
		key: &[u8],
	) -> Option<Vec<u8>> {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.child_storage(&child_info, key).map(|s| s.to_vec())
	}

	/// Read child key.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::read`).
	fn child_read(
		&self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
		key: &[u8],
		value_out: &mut [u8],
		value_offset: u32,
	) -> Option<u32> {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.child_storage(&child_info, key)
			.map(|value| {
				let value_offset = value_offset as usize;
				let data = &value[value_offset.min(value.len())..];
				let written = std::cmp::min(data.len(), value_out.len());
				value_out[..written].copy_from_slice(&data[..written]);
				value.len() as u32
			})
	}

	/// Set a child storage value.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::set`).
	fn child_set(
		&mut self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
		key: &[u8],
		value: &[u8],
	) {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.set_child_storage(&child_info, key.to_vec(), value.to_vec());
	}

	/// Remove child key value.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::clear`).
	fn child_clear(
		&mut self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
		key: &[u8],
	) {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.clear_child_storage(&child_info, key);
	}

	/// Remove all child storage values.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::storage_kill`).
	fn child_storage_kill(
		&mut self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
	) {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.kill_child_storage(&child_info);
	}

	/// Check a child storage key.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::exists`).
	fn child_exists(
		&self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
		key: &[u8],
	) -> bool {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.exists_child_storage(&child_info, key)
	}

	/// Clear child key by prefix.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::clear_prefix`).
	fn child_clear_prefix(
		&mut self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
		prefix: &[u8],
	) {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.clear_child_prefix(&child_info, prefix);
	}

	/// Child trie root calcualation.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::clear_root`).
	fn child_root(
		&mut self,
		storage_key: &[u8],
	) -> Vec<u8> {
		let prefix = ChildType::ParentKeyId.parent_prefix();
		if !storage_key.starts_with(prefix) {
			panic!("Invalid storage key");
		}
		let storage_key = &storage_key[prefix.len()..];
		let child_info = resolve_child_info(ChildType::ParentKeyId as u32, storage_key)
			.expect("Invalid storage key");
		self.child_storage_root(&child_info)
	}

	/// Child storage key iteration.
	///
	/// Deprecated, please use dedicated runtime apis (`sp_io::default_child_storage::next_key`).
	fn child_next_key(
		&mut self,
		storage_key: &[u8],
		child_definition: &[u8],
		child_type: u32,
		key: &[u8],
	) -> Option<Vec<u8>> {
		deprecated_storage_key_prefix_check(storage_key);
		let child_info = resolve_child_info(child_type, child_definition)
			.expect("Invalid child definition");
		self.next_child_storage_key(&child_info, key)
	}
}
