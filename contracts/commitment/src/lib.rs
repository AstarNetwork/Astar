#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{env::DefaultSrmlTypes, memory::format, storage};
use ink_lang::contract;
use primitives::default::*;

contract! {
    #![env = DefaultSrmlTypes]

    /// Event deposited when a submit merkle root to parent chain contract(this contract) from child chain.
    ///
    /// ```
    /// event BlockSubmitted(
    ///		uint256 _number,
    ///		bytes _header
    /// );
    /// ```
    event BlockSubmitted {
        number: BlockNumber,
        header: Hash,
    }

    /// Each plasma chain MUST have at least one commitment contract.
    /// Commitment contracts hold the block headers for the plasma chain.
    /// Whenever the operator creates a new plasma block, they MUST publish this block to the commitment contract.
    struct Commitment {
           /// Block number of the most recently published plasma block.
        /// ```
        /// uint256 public currentBlock;
        /// ```
        current_block: storage::Value<BlockNumber>,

        /// Mapping from block number to block header.
        /// ```
        /// mapping (uint256 => bytes) public blocks;
        /// ```
        blocks: storage::HashMap<BlockNumber, Hash>,
    }

    impl Deploy for Commitment {
        /// Initializes our state to `false` upon deploying our smart contract.
        fn deploy(&mut self) {
            self.current_block.set(0)
        }
    }

    impl Commitment {
        /// Returns the current block number.
        pub(external) fn current_block(&self) -> BlockNumber {
            let current_block = *self.current_block;
            env.println(&format!("Commitment::current_block = {:?}", current_block));
            current_block
        }

        /// Returns the balance of the given AccountId.
        pub(external) fn block_hash(&self, number: BlockNumber) -> Option<Hash> {
            if let Some(block) = self.blocks.get(&number) {
                env.println(&format!("Commitment::block_hash(number = {:?}) = {:?}", number, block));
                return Some(block.clone());
            }
            env.println(&format!("Commitment::block_hash(number = {:?}) = None)", number));
            None
        }

        /// Allows a user to submit a block with the given header.
        /// ```
        /// function submitBlock(bytes _header) public
        /// ```
        pub(external) fn submit_block(&mut self, header: Hash) {
            *self.current_block += 1;
            self.blocks.insert(*self.current_block, header.clone());
            env.emit(BlockSubmitted {
                number: *self.current_block,
                header: header,
            });
        }
    }
}

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;
    use parity_codec::Decode;

    #[test]
    fn it_works() {
        let mut contract = Commitment::deploy_mock();
        assert_eq!(contract.current_block(), 0);
        assert_eq!(contract.block_hash(0), None);

        let header_1: Hash = Hash::decode(&mut &[1u8; 32].to_vec()[..]).unwrap();
        contract.submit_block(header_1.clone());
        assert_eq!(contract.current_block(), 1);
        assert_eq!(contract.block_hash(0), None);
        assert_eq!(contract.block_hash(1), Some(header_1));

        let header_2: Hash = Hash::decode(&mut &[2u8; 32].to_vec()[..]).unwrap();
        contract.submit_block(header_2.clone());
        assert_eq!(contract.current_block(), 2);
        assert_eq!(contract.block_hash(0), None);
        assert_eq!(contract.block_hash(1), Some(header_1));
        assert_eq!(contract.block_hash(2), Some(header_2));
    }
}
