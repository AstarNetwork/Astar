#![cfg_attr(not(any(test, feature = "std")), no_std)]

use ink_core::{memory::{format, vec::Vec}, storage};
use core::option::Option;
use ink_lang::contract;
use predicate::ownership::{Signature, TransactionBody};
use primitives::default::*;
use predicate::traits::Predicate;

// TODO: Comment Japanese, because WIP specific implmentation.

contract! {
    #![env = ink_core::env::DefaultSrmlTypes]

	/// Ownership Plasma Standard Contract.
    struct Ownership {
        /// The current state of our flag.
        predicate: predicate::ownership::Predicate,
    }

    impl Deploy for Ownership {
        /// Initializes our state to `false` upon deploying our smart contract.
        fn deploy(&mut self,
            token_address: AccountId,
            chalenge_period: BlockNumber,
            exit_period: BlockNumber,
        ) {
            self.predicate.deploy(env, token_address, chalenge_period, exit_period);
        }
    }

    impl Ownership {
        ///			マークルルートを提出する。
        //			一般にオペレータのみが提出を許可される。
        pub(external) fn submit_block(&mut self, header: Hash) {

        }
        ///  checkpoint の作成を開始する。ある一定期間 challenge_checkpoint されなければチェックポイントが作成される
        ///  deposited_range_id は depositedRanges の始点
        pub(external) fn start_checkpoint(&mut self,
        	checkpoint: Checkpoint<AccountId>,
        	inclusion_proof: Vec<Hash>,
        	deposited_range_id: RangeNumber) {
        }

        //// 新しいチェックポイントを示すことで古いチェックポイントを消す
        pub(external) fn delete_exit_outdated(&mut self,
        	older_exit: Checkpoint<AccountId>,
        	newer_checkpoint: Checkpoint<AccountId>) {
        }

        /// チェックポイントに対するチャレンジ
        pub(external) fn challenge_checkpoint(&mut self, challenge: Challenge<AccountId>) {}

        /// チャレンジされているチェックポイントの削除
        pub(external) fn removeChallenge(&mut self, challenge: Challenge<AccountId>) {}

        /// Exit の開始
        pub(external) fn start_exit(&mut self,	checkpoint: Checkpoint<AccountId>){}

        /// Exit の無効化（要はチャレンジ）
        pub(external) fn deprecate_exit(&mut self,
            deprecated_exit: Checkpoint<AccountId>,
            transaction: Transaction<TransactionBody>,
            witness: Signature,
            post_state: StateUpdate<AccountId>) {}

        /// Exit finalize
        pub(external) fn finalizeExit(&mut self,
            checkpoint: Checkpoint<AccountId>,
            deposited_range_id: RangeNumber) {}

        ///	最新のブロックハッシュを取得する。
        pub(external) fn current_block(&self) -> BlockNumber { 1u64 }


        pub(external) fn block_hash(&self, number: BlockNumber) -> Option<Hash> { None }
    }
}

#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut contract = Ownership::deploy_mock();
        assert_eq!(contract.get(), false);
        contract.flip();
        assert_eq!(contract.get(), true);
    }
}
