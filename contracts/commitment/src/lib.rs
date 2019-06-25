#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{
	memory::{format, vec::Vec},
	storage,
};
use ink_lang::contract;

contract! {
    /// Event deposited when a submit merkle root to parent chain contract(this contract) from child chain.
    ///
    /// ```
    /// event BlockSubmitted(
    ///		uint256 _number,
    ///		bytes _header
    /// );
    /// ```
    event BlockSubmitted {
    	number: u128,
    	header: Vec<u8>,
    }

    /// Each plasma chain MUST have at least one commitment contract.
    /// Commitment contracts hold the block headers for the plasma chain.
    /// Whenever the operator creates a new plasma block, they MUST publish this block to the commitment contract.
    struct Commitment {
       	/// Block number of the most recently published plasma block.
    	/// ```
    	/// uint256 public currentBlock;
    	/// ```
    	current_block: storage::Value<u128>,

        /// Mapping from block number to block header.
        /// ```
        /// mapping (uint256 => bytes) public blocks;
        /// ```
        blocks: storage::HashMap<u128, Vec<u8>>,
    }

    impl Deploy for Commitment {
        /// Initializes our state to `false` upon deploying our smart contract.
        fn deploy(&mut self) {
            self.current_block.set(0)
        }
    }

    impl Commitment {
        /// Returns the current block number.
        pub(external) fn current_block(&self) -> u128 {
            let current_block = *self.current_block;
            env.println(&format!("Commitment::current_block = {:?}", current_block));
            current_block
        }

        /// Returns the balance of the given AccountId.
        pub(external) fn block_hash(&self, number: u128) -> Vec<u8> {
            if let Some(block) = self.blocks.get(&number) {
            	env.println(&format!("Commitment::block_hash(number = {:?}) = {:?}", number, block));
            	return block.clone();
            }
            env.println(&format!("Commitment::block_hash(number = {:?}) = None)", number));
            Vec::new()
        }

    	/// Allows a user to submit a block with the given header.
    	/// ```
    	/// function submitBlock(bytes _header) public
    	/// ```
  		pub(external) fn submit_block(&mut self, header: Vec<u8>) {
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

	#[test]
	fn it_works() {
		let mut contract = Commitment::deploy_mock();
		assert_eq!(contract.current_block(), 0);
		assert_eq!(contract.block_hash(0), vec! {});


		let header_1 = [1u8; 32].to_vec();
		contract.submit_block(header_1.clone());
		assert_eq!(contract.current_block(), 1);
		assert_eq!(contract.block_hash(0), vec! {});
		assert_eq!(contract.block_hash(1), header_1);

		let header_2 = [2u8; 32].to_vec();
		contract.submit_block(header_2.clone());
		assert_eq!(contract.current_block(), 2);
		assert_eq!(contract.block_hash(0), vec! {});
		assert_eq!(contract.block_hash(1), header_1);
		assert_eq!(contract.block_hash(2), header_2);
	}
}
