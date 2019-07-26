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
    ) -> Self {
        let mut tree = MerkleIntervalTree::<T, F> {
            tree: Vec::new(),
            children: Vec::new(),
            _phantom: PhantomData::<(T, F)>,
        };

        // Emtpy Tree
        if leaf_nodes.len() == 0 {
            return tree;
        }

        // Leaves intersect
        leaf_nodes.sort_by_key(|a| a.start);
        for i in (0..leaf_nodes.len() - 1) {
            if leaf_nodes[i].end >= leaf_nodes[i + 1].start {
                return tree;
            }
        }

        // leaf add to self.children
        for leaf in leaf_nodes.iter() {
            tree.children.push(MerkleIntervalTreeInternalNode {
                index: leaf.start.clone(),
                hash: hash_func(&leaf.encode()[..]),
            });
        }
        tree
    }

    pub fn generate_internal_nodes(&mut self, hash_func: &F) {
        if self.children.len() == 1 {
            return;
        }
        let mut parents: Vec<MerkleIntervalTreeInternalNode<RangeNumber>> = Vec::new();
        for i in (0..self.children.len()) {
            let left_child: MerkleIntervalTreeInternalNode<RangeNumber>;
            let right_child: MerkleIntervalTreeInternalNode<RangeNumber>;
            if i % 2 == 0 {
                let left_child = self.children[i].clone();
                if i + 1 == self.children.len() {
                    right_child = MerkleIntervalTreeInternalNode {
                        index: left_child.index.clone(),
                        hash: Hash::decode(&mut &[0; 32][..]).expect("hash decodec default."),
                    };
                } else {
                    right_child = self.children[i + 1].clone();
                }
                parents.push(MerkleIntervalTreeInternalNode {
                    index: left_child.index,
                    hash: default::concat_hash(&left_child, &right_child, hash_func),
                })
            }
        }
        self.children = parents.clone();
        self.tree.push(parents);
        self.generate_internal_nodes(hash_func);
    }
}
