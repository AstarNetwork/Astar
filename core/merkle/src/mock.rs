use super::*;
use rstd::marker::PhantomData;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash};
use parity_codec::Codec;

// mock merkle tree trie id name. no conflict.
// TODO: see https://github.com/paritytech/substrate/issues/2325
pub const MOCK_MERKLE_TREE_TRIE_ID: &'static [u8] = b":child_storage:default: mock_merkle_tree_trie_id";
const MOCK_MERKLE_TREE_DEPTH: u8 = 20;
/// must be 2^n.
const MOCK_MERKLE_TREE_LIMIT: u64 = (1 << MOCK_MERKLE_TREE_DEPTH as u64);

/// MerkleTree measn
/// 		0
/// 1	2		3	4
/// 5 6 7 8	  9 10 11 12
///
/// Alike SegmentTree. So fixed number of data.
pub struct MerkleTree<H, Hashing>(PhantomData<(H, Hashing)>);

// impl_merkle_accessor : Self::get_**, Self::push_**.
macro_rules! impl_merkle_accessor {
	( $x:ident ) => {
		impl<H: Codec + Default, B> $x<H, B> {
			pub fn get_hash(index: u64) -> H {
				MerkleDb::<u64, H>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, &index).unwrap_or(Default::default())
			}
			pub fn get_index(h: &H) -> u64 {
				MerkleDb::<H, u64>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h).unwrap_or(0)
			}
			pub fn push_hash(index: u64, h: H) {
				MerkleDb::<u64, H>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, &index, h);
			}
			pub fn push_index(h: &H, index: u64) {
				MerkleDb::<H, u64>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h, index);
			}
		}
	}
}

impl_merkle_accessor!(MerkleTree);

/// Implemention of MerkleTree.
///
/// get_index(default::Default()) = number of utxo.
/// get_index(hash_of(default::Default()) = number of proposed utxo.
/// get_index(leaf) = index of leaf. value is [0, MOCK_MERKLE_TREE_LIMIT).
///
/// get_hash(MOCK_MERKLE_TREE_LIMIT << 1 + i) = i-th porposed utxo hash.
/// get_hash([0, MOCK_MERKLE_TREE_LIMIT<<1)) = i-th node hash. [MOCK_MERKLE_TREE_LIMIT - 1, MOCK_MERKLE_TREE_LIMIT<<1) is leaf.
impl<H, Hashing> MerkleTreeTrait<H, Hashing> for MerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	fn root() -> H {
		Self::get_hash(0)
	}

	fn proofs(leaf: &H) -> MerkleProof<H> {
		let mut index: u64 = Self::get_index(leaf);
		let ret_index = index;
		let mut proofs = vec! {leaf.clone()};

		index += MOCK_MERKLE_TREE_LIMIT - 1;
		while index > 0 {
			let lr: bool = (index & 1) == 1;
			index = (index - 1) / 2;
			match lr {
				true => proofs.push(Self::get_hash(2 * index + 2)),    // left leafs.
				false => proofs = vec! {Self::get_hash(2 * index + 1)}
					.iter().chain(proofs.iter()).map(|x| *x).collect::<Vec<_>>(), // right leafs.
			}
		}
		MerkleProof {
			proofs: proofs,
			depth: MOCK_MERKLE_TREE_DEPTH,
			index: ret_index,
		}
	}

	fn push(leaf: H) {
		let h: H = Default::default();
		let x = Hashing::hash_of(&h);
		let cnt = Self::get_index(&x); // [DefaultHash] = number of proposed tx.
		Self::push_index(&x, cnt + 1);
		Self::push_hash(MOCK_MERKLE_TREE_LIMIT << 1 + cnt, leaf); // [temporary_save_index] = leaf hash
	}

	fn commit() {
		let h: H = Default::default();
		let x = Hashing::hash_of(&h); // DefaultHash
		let cnt = Self::get_index(&x); // [DefaultHash] = number of proposed tx.
		Self::push_index(&x, 0);
		for i in 0..cnt {
			let leaf = Self::get_hash(MOCK_MERKLE_TREE_LIMIT << 1 + i);
			let mut index: u64 = Self::get_index(&h);
			Self::push_index(&Default::default(), index + 1); // increments...
			Self::push_index(&leaf, index);

			index += MOCK_MERKLE_TREE_LIMIT - 1;
			Self::push_hash(index, leaf);
			while index > 0 {
				index = (index - 1) / 2;
				Self::push_hash(index,
								concat_hash(&Self::get_hash(2 * index + 1),
											&Self::get_hash(2 * index + 2),
											Hashing::hash));
			}
		}
	}
}

impl<H, Hashing> RecoverableMerkleTreeTrait<H, Hashing> for MerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	type Out = PastMerkleTree<H, Hashing>;

	/// loading root_hash state.
	fn load(root: &H) -> Self::Out {
		let past = Self::get_index(root);
		PastMerkleTree::<H, Hashing> {
			past: past,
			_phantom: Default::default(),
		}
	}

	/// [root_hash] = current number of utxo.
	fn save() {
		let root = Self::root();
		let index = Self::get_index(&Default::default());
		Self::push_index(&root, index);
	}
}

pub struct PastMerkleTree<H, Hashing> {
	past: u64,
	_phantom: PhantomData<(H, Hashing)>,
}

impl_merkle_accessor!(PastMerkleTree);
impl<H: Codec + Default, B> PastMerkleTree<H, B> {
	// get hash by past.
	fn get_hash_from_node(&self, index: u64) -> H {
		let mut cpy = index;
		let mut depths = 0; // depth 0-index.
		while cpy >= 0 {
			cpy = (cpy - 1) >> 1;
			depths += 1;
		}

		let num_elm = (1 << depth); // number of elements this depth.
		let weight_nodes = MOCK_MERKLE_TREE_LIMIT / num_elm; // this nodes overwrap [l,r), r-l == weight-nodes.
		let node_index_with_depth = index - (1 << depth) + 1; // node index with this depths.
		let left = node_index_with_depth * weight_nodes;
		let right = left + weight_nodes;
		// dps is depth of node.
		self._get_hash_from_node(index, left, right)
	}

	// node-index, [left, right).
	fn _get_hash_from_node(&self, index: u64, left: u64, right: u64) -> H {
		if left >= past { // outer
			return Default::default();
		} else if right <= past { // inner
			return get_hash(&index);
		}
		concat_hash(self._get_hash_from_node(2 * index + 1, left, (left + right) / 2),
					self._get_hash_from_node(2 * index + 2, (left + right) / 2, right),
					Hashing::hash)
	}
}

impl<H, Hashing> MerkleTreeTrait<H, Hashing> for PastMerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	fn root() -> H {
		// TODO

	}

	fn proofs(leaf: &H) -> MerkleProof<H> {
		let mut index: u64 = Self::get_index(leaf);
		let ret_index = index;
		let mut proofs = vec! {leaf.clone()};

		index += MOCK_MERKLE_TREE_LIMIT - 1;
		while index > 0 {
			let lr: bool = (index & 1) == 1;
			index = (index - 1) / 2;
			match lr {
				true => proofs.push(Self::get_hash(2 * index + 2)),    // left leafs.
				false => proofs = vec! {Self::get_hash(2 * index + 1)}
					.iter().chain(proofs.iter()).map(|x| *x).collect::<Vec<_>>(), // right leafs.
			}
		}
		MerkleProof {
			proofs: proofs,
			depth: MOCK_MERKLE_TREE_DEPTH,
			index: ret_index,
		}
	}

	fn push(leaf: H) {
		let h: H = Default::default();
		let x = Hashing::hash_of(&h);
		let cnt = Self::get_index(&x);
		Self::push_index(&x, cnt + 1);
		Self::push_hash(MOCK_MERKLE_TREE_LIMIT << 1 + cnt, leaf);
	}

	fn commit() {
		let h: H = Default::default();
		let x = Hashing::hash_of(&h);
		let cnt = Self::get_index(&x);
		Self::push_index(&x, 0);
		for i in 0..cnt {
			let leaf = Self::get_hash(MOCK_MERKLE_TREE_LIMIT << 1 + i);
			let mut index: u64 = Self::get_index(&h);
			Self::push_index(&Default::default(), index + 1); // increments...
			Self::push_index(&leaf, index);

			index += MOCK_MERKLE_TREE_LIMIT - 1;
			Self::push_hash(index, leaf);
			while index > 0 {
				index = (index - 1) / 2;
				Self::push_hash(index,
								concat_hash(&Self::get_hash(2 * index + 1),
											&Self::get_hash(2 * index + 2),
											Hashing::hash));
			}
		}
	}
}







