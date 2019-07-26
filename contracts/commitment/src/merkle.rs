use super::*;
use primitives::{default::RangeNumber, traits};
use std::marker::PhantomData;

struct MerkleIntervalTree<T, F>
where
    T: traits::Member + Codec,
    F: FnOnce(&[u8]) -> Hash,
{
    pub tree: Vec<Vec<MerkleIntervalTreeInternalNode<RangeNumber>>>,
    pub children: Vec<MerkleIntervalTreeInternalNode<RangeNumber>>,
    _phantom: PhantomData<(T, F)>,
}

impl<T, F> MerkleIntervalTree<T, F>
where
    T: traits::Member + Codec,
    F: Fn(&[u8]) -> Hash,
{
    pub fn generate_tree(
        leaf_nodes: &mut Vec<MerkleIntervalTreeLeafNode<RangeNumber, T>>,
        hash_func: F,
    ) {
        let mut tree = MerkleIntervalTree::<T, F> {
            tree: Vec::new(),
            children: Vec::new(),
            _phantom: PhantomData::<(T, F)>,
        };

        // Emtpy Tree
        if leaf_nodes.len() == 0 {
            return;
        }

        // Leaves intersect
        leaf_nodes.sort_by_key(|a| a.start);
        for i in (0..leaf_nodes.len() - 1) {
            if leaf_nodes[i].end >= leaf_nodes[i + 1].start {
                return;
            }
        }

        // leaf add to children.
        for leaf in leaf_nodes.iter() {
            tree.children.push(MerkleIntervalTreeInternalNode {
                index: leaf.start.clone(),
                hash: hash_func(&leaf.encode()[..]),
            });
        }
    }

    pub fn generate_internal_nodes(&mut self) {
        if self.children.len() == 1 {
            return;
        }
    }
}
