use super::*;
use rstd::marker::PhantomData;
use sr_primitives::traits::{Member, MaybeSerializeDebug, Hash};
use parity_codec::Codec;

// mock merkle tree trie id name. no conflict.
// TODO: see https://github.com/paritytech/substrate/issues/2325
pub const MOCK_MERKLE_TREE_TRIE_ID: &'static [u8] = b":child_storage:default: mock_merkle_tree_trie_id";
const MOCK_MERKLE_TREE_DEPTH: u32 = 20;
/// must be 2^n.
const MOCK_MERKLE_TREE_LIMIT: u64 = (1 << MOCK_MERKLE_TREE_DEPTH as u64);

/// MerkleTree measn
/// 		0
/// 1	2		3	4
/// 5 6 7 8	  9 10 11 12
///
/// Alike SegmentTree. So fixed number of data.
#[derive(PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct MerkleTree<H, Hashing> {
	past: u64,
	_phantom: PhantomData<(H, Hashing)>,
}

// impl_merkle_accessor : Self::get_**, Self::push_**.
impl<H: Codec + Default, Hashing> MerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	pub fn get_hash(index: u64) -> H {
		MerkleDb::<u64, H>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, &index).unwrap_or(Default::default())
	}
	pub fn get_index(h: &H) -> u64 {
		Self::get_index_optionaly(h).unwrap_or(0)
	}
	pub fn get_index_optionaly(h: &H) -> Option<u64> {
		MerkleDb::<H, u64>::get(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h)
	}
	pub fn push_hash(index: u64, h: H) {
		MerkleDb::<u64, H>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, &index, h);
	}
	pub fn push_index(h: &H, index: u64) {
		MerkleDb::<H, u64>::push(&DirectMerkleDb, &MOCK_MERKLE_TREE_TRIE_ID, h, index);
	}

	/// get_index(default::Default()) = number of utxo.
	fn get_num_of_utxo() -> u64 {
		Self::get_index(&Default::default())
	}
	fn set_num_of_utxo(i: u64) {
		Self::push_index(&Default::default(), i);
	}

	/// get_index(hash_of(default::Default()) = number of proposed utxo.
	fn get_num_of_proposal() -> u64 {
		let h: H = Default::default();
		Self::get_index(&Hashing::hash_of(&h))
	}
	fn set_num_of_proposal(i: u64) {
		let h: H = Default::default();
		Self::push_index(&Hashing::hash_of(&h), i);
	}

	// get_root_oast(hash(root_hash)) = past_index.
	fn get_root_past(h: &H) -> Option<u64> {
		Self::get_index_optionaly(&Hashing::hash_of(h))
	}
	fn set_root_past(h: &H, i: u64) {
		Self::push_index(&Hashing::hash_of(h), i);
	}

	/// get_hash(MOCK_MERKLE_TREE_LIMIT << 1 + i) = i-th porposed utxo hash.
	fn get_proposal_hash(i: u64) -> H {
		Self::get_hash(MOCK_MERKLE_TREE_LIMIT << 1 + i)
	}
	fn set_proposal_hash(i: u64, h: H) {
		Self::push_hash(MOCK_MERKLE_TREE_LIMIT << 1 + i, h);
	}

	// get hash by past.
	fn get_hash_from_node(&self, index: u64) -> H {
		let mut cpy = index;
		let mut depth = 0; // depth 0-index.
		while cpy > 0 {
			cpy = (cpy - 1) >> 1;
			depth += 1;
		}

		let num_elm = 1 << depth; // number of elements this depth.
		let weight_nodes = MOCK_MERKLE_TREE_LIMIT / num_elm; // this nodes overwrap [l,r), r-l == weight-nodes.
		let node_index_with_depth = index + 1 - (1 << depth); // node index with this depth.
		let left = node_index_with_depth * weight_nodes;
		let right = left + weight_nodes;
		// dps is depth of node.
		self._get_hash_from_node(index, left, right)
	}

	// node-index, [left, right).
	fn _get_hash_from_node(&self, index: u64, left: u64, right: u64) -> H {
		if left >= self.past { // outer
			return Default::default();
		} else if right <= self.past { // inner
			return Self::get_hash(index);
		}
		concat_hash(&self._get_hash_from_node(2 * index + 1, left, (left + right) / 2),
					&self._get_hash_from_node(2 * index + 2, (left + right) / 2, right),
					Hashing::hash)
	}
}

impl<H, Hashing> ReadOnlyMerkleTreeTrait<H, Hashing> for MerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	fn root(&self) -> H {
		self.get_hash_from_node(0)
	}
	fn proofs(&self, leaf: &H) -> Option<MerkleProof<H>> {
		let mut index: u64;
		match Self::get_index_optionaly(leaf) {
			Some(i) => index = i,
			None => return None,
		}

		let ret_index = index;
		let mut proofs = vec! {};

		index += MOCK_MERKLE_TREE_LIMIT - 1;

		index += 1;
		let mut top = 0;
		while index >=(1<<(top+1)){top+=1};
		for is_one in (0..top).rev(){
			let temp = index>>is_one;
			if temp&1!=0 {
				proofs.push(self.get_hash_from_node((temp^1)-1));

			}
		}
		proofs.push(self.get_hash_from_node(index-1));
		for is_zero in 0..top{
			let temp = index>>is_zero;
			if temp&1==0 {
				proofs.push(self.get_hash_from_node((temp^1)-1));
			}
		}

		Some(MerkleProof {
			proofs: proofs,
			depth: MOCK_MERKLE_TREE_DEPTH,
			index: ret_index,
		})
	}
}

/// Implemention of MerkleTree.
///.
/// get_index(leaf) = index of leaf. value is [0, MOCK_MERKLE_TREE_LIMIT).
/// get_hash([0, MOCK_MERKLE_TREE_LIMIT<<1)) = i-th node hash. [MOCK_MERKLE_TREE_LIMIT - 1, MOCK_MERKLE_TREE_LIMIT<<1) is leaf
impl<H, Hashing> MerkleTreeTrait<H, Hashing> for MerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H>
{
	fn new() -> Self {
		MerkleTree::<H, Hashing> {
			past: MOCK_MERKLE_TREE_LIMIT,
			_phantom: Default::default(),
		}
	}

	fn push(&self, leaf: H) {
		let cnt = Self::get_num_of_proposal();
		Self::set_num_of_proposal(cnt + 1);
		Self::set_proposal_hash(cnt, leaf);
	}

	fn commit(&self) {
		let cnt = Self::get_num_of_proposal();
		Self::set_num_of_proposal(0);
		for i in 0..cnt {
			let leaf = Self::get_proposal_hash(i);
			let mut index: u64 = Self::get_num_of_utxo();
			Self::set_num_of_utxo(index + 1); // increments...
			Self::push_index(&leaf, index);

			index += MOCK_MERKLE_TREE_LIMIT - 1;
			Self::push_hash(index, leaf);
			while index > 0 {
				index = (index - 1) / 2;
				Self::push_hash(index,
								concat_hash(&self.get_hash_from_node(2 * index + 1),
											&self.get_hash_from_node(2 * index + 2),
											Hashing::hash));
			}
		}
	}
}

impl<H, Hashing> RecoverableMerkleTreeTrait<H, Hashing> for MerkleTree<H, Hashing>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  Hashing: Hash<Output=H> {
	type Out = Self;
	/// loading root_hash state.
	fn load(root: &H) -> Option<Self::Out> {
		match Self::get_root_past(root) {
			Some(past) => return Some(MerkleTree::<H, Hashing> {
				past: past,
				_phantom: Default::default(),
			}),
			None => return None,
		}
	}

	/// [root_hash] = current number of utxo.
	fn save(&self) {
		let root = self.root();
		let index = Self::get_index(&Default::default());
		Self::set_root_past(&root, index);
	}
}





