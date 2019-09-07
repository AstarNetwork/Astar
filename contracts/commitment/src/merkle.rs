//! Has a implementation of merkle interval tree for plasma contracts tests.
//! Refer to https://docs.plasma.group/projects/spec/en/latest/src/01-core/merkle-interval-tree.html.

use super::*;
use primitives::{default::RangeNumber, traits};

use core::marker::PhantomData;

pub type Nodes = Vec<MerkleIntervalTreeInternalNode<RangeNumber>>;
pub type Tree = Vec<Nodes>;

pub struct MerkleIntervalTreeGenerator<T, F>
where
    T: traits::Member + Codec,
    F: FnOnce(&[u8]) -> Hash,
{
    _phantom: PhantomData<(T, F)>,
}

impl<T, F> MerkleIntervalTreeGenerator<T, F>
where
    T: traits::Member + Codec,
    F: Fn(&[u8]) -> Hash + Copy,
{
    pub fn generate_leafs(
        leaf_nodes: &mut Vec<primitives::default::StateUpdate<T>>,
        hash_func: F,
    ) -> Nodes {
        let mut children: Nodes = Vec::new();

        // Emtpy Tree
        if leaf_nodes.len() == 0 {
            return children;
        }

        // Leaves intersect
        leaf_nodes.sort_by_key(|a| a.range.start);
        for i in (0..leaf_nodes.len() - 1) {
            if leaf_nodes[i].range.end > leaf_nodes[i + 1].range.start {
                return children;
            }
        }

        // leaf add to children
        for leaf in leaf_nodes.iter() {
            children.push(MerkleIntervalTreeInternalNode {
                index: leaf.range.start.clone(),
                hash: hash_func(&leaf.encode()[..]),
            });
        }
        children
    }

    pub fn generate_internal_nodes(children: &Nodes, hash_func: F) -> Tree {
        if children.len() == 1 {
            return vec![children.clone()];
        }
        let mut parents: Nodes = Vec::new();
        for i in (0..children.len()) {
            let left_child: MerkleIntervalTreeInternalNode<RangeNumber>;
            let right_child: MerkleIntervalTreeInternalNode<RangeNumber>;
            if i % 2 == 0 {
                let left_child = children[i].clone();
                if i + 1 == children.len() {
                    right_child = MerkleIntervalTreeInternalNode {
                        index: left_child.index.clone(),
                        hash: Hash::decode(&mut &[0; 32][..]).expect("hash decodec default."),
                    };
                } else {
                    right_child = children[i + 1].clone();
                }
                parents.push(MerkleIntervalTreeInternalNode {
                    index: left_child.index,
                    hash: default::concat_hash(&left_child, &right_child, hash_func),
                })
            }
        }
        let mut tree = Self::generate_internal_nodes(&parents, hash_func);
        tree.push(children.clone());
        tree
    }

    pub fn generate_proof(
        tree: &Tree,
        leaf_node: &primitives::default::StateUpdate<T>,
        hash_func: F,
    ) -> primitives::Result<InclusionProof<RangeNumber>> {
        let leaves = &tree[tree.len() - 1];
        let mut leaf_index: usize = 0;
        while leaf_index < leaves.len() {
            if &hash_func(&leaf_node.encode()[..]) == &leaves[leaf_index].hash {
                break;
            }
            leaf_index += 1;
        }
        if leaf_index == leaves.len() {
            return Err("have not contains leaf_ndoe.");
        }
        Ok(InclusionProof::<RangeNumber> {
            proofs: Self::find_siblings(tree, tree.len() - 1, leaf_index, Vec::new()),
            idx: leaf_index as u128,
        })
    }

    pub fn find_siblings(
        tree: &Tree,
        height: usize,
        child_index: usize,
        mut proof: Nodes,
    ) -> Nodes {
        if height == 0 {
            return proof;
        }

        proof.push(tree[height][Self::get_sibling(child_index)].clone());
        let parent_index = child_index / 2;
        Self::find_siblings(tree, height - 1, parent_index, proof)
    }

    fn get_sibling(index: usize) -> usize {
        index + 1 - (index & 1) * 2
    }
}
