//! # Plasma Module
//!
//! The Plasma module provides functionality for handling layer2 dispute logics.
//! This refer to: https://github.com/cryptoeconomicslab/ovm-contracts/blob/master/contracts/UniversalAdjudicationContract.sol
//!
//! - [`plasma::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//! Plasma Module is a module that is responsible for processing specific to Plasma.
//! It calls the OVM Module and the specified smart contract function.
//! The Plasma Module has one "Commitment" and "Deposit" address per application.
//! These are each defined by decl_child_storage. decl_child_storage! is a macro that
//! implements DB in SubTrie. This sets AccountId as the key value.
//! This is like a contract address. Specifically, implements with reference to AccountDb of contract module.
//!
//! This is modularized Commitment, Deposit and CompiledPredicate contracts in the Ethereum.
//!
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, Get},
    weights::{SimpleDispatchInfo, WeighData, Weight},
    StorageMap,
};
use frame_system::{self as system, ensure_signed};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Hash, RuntimeDebug};
use sp_std::{prelude::*, vec::Vec};

use pallet_ovm::Property;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Range<Balance> {
    start: Balance,
    end: Balance,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct StateUpdate<AccountId, Balance, BlockNumber> {
    deposit_contract_address: AccountId,
    ragne: Range<Balance>,
    block_number: BlockNumber,
    state_object: Property<AccountId>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Checkpoint<AccountId, Balance> {
    subsrange: Range<Balance>,
    state_update: Property<AccountId>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Exit<AccountId, Range, BlockNumber, Balance, Index> {
    state_update: StateUpdate<AccountId, Range, BlockNumber>,
    inclusion_proof: InclusionProof<AccountId, Balance, Index>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct InclusionProof<AccountId, Balance, Index> {
    address_inclusion_proof: AddressInclusionProof<AccountId, Index>,
    interval_inclusion_proof: IntervalInclusionProof<Balance, Index>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct IntervalInclusionProof<Balance, Index> {
    leaf_index: Index,
    leaf_position: Index,
    sibilings: Vec<IntervalTreeNode<Balance>>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct AddressInclusionProof<AccountId, Index> {
    leaf_index: AccountId,
    leaf_position: Index,
    siblings: Vec<AddressTreeNode<AccountId>>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct IntervalTreeNode<Balance> {
    data: Vec<u8>,
    start: Balance,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct AddressTreeNode<AccountId> {
    data: Vec<u8>,
    token_address: AccountId,
}

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type CheckpointOf<T> = Checkpoint<<T as frame_system::Trait>::AccountId, BalanceOf<T>>;
pub type RangeOf<T> = Range<BalanceOf<T>>;

pub trait Trait: system::Trait {
    /// Plasma Range's currency.
    type Currency: Currency<Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Plasma {
        // Commitment storage: Plapps address => Commitment Child Storage. ====

        /// Single operator address: OperatorId
        OperatorAddress get(fn operator_address): map hasher(twox_64_concat) T::AccountId => T::AccountId;
        /// Current block number of commitment chain: BlockNumber
        CurrentBlock get(fn current_block): map hasher(twox_64_concat) T::AccountId => T::BlockNumber;
        /// History of Merkle Root
        Blocks get(fn blocks): double_map hasher(twox_64_concat) T::AccountId, hasher(blake2_128_concat) u128 => T::Hash;


        // Deposit storage: Plapps address => Deposit Child Storage. ====
        /// mapping from Plapps address to ERC20 based contract address.
        ERC20 get(fn erc20): map hasher(twox_64_concat) T::AccountId => T::AccountId;
        /// mapping from Plapps address to StateUpdate predicate address.
        StateUpdatePredicate get(fn state_update_predicate): map hasher(twox_64_concat) T::AccountId => T::AccountId;

        /// TotalDeposited is the most right coin id which has been deposited.
        TotalDeposited get(fn total_deposited): map hasher(twox_64_concat) T::AccountId => BalanceOf<T>;
        /// DepositedRanges are currently deposited ranges.
        DepositedRanges get(fn deposited_ranges): double_map hasher(twox_64_concat) T::AccountId, hasher(blake2_128_concat) BalanceOf<T> => RangeOf<T>;
        /// Range's Checkpoints.
        Checkpoints get(fn checkpoints): double_map hasher(twox_64_concat) T::AccountId, hasher(blake2_128_concat) T::Hash => CheckpointOf<T>;

        /// predicate address => payout address
        PayoutContractAddress get(fn payout_contract_address): map hasher(twox_64_concat) T::AccountId => T::AccountId;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Hash = <T as system::Trait>::Hash,
        BlockNumber = <T as system::Trait>::BlockNumber,
        Range = RangeOf<T>,
        Checkpoint = CheckpointOf<T>,
    {
        // Event definitions (AccountID: PlappsAddress, BlockNumber, Hash: root)
        BlockSubmitted(AccountId, BlockNumber, Hash),
        // (checkpointId: Hash, checkpoint: Checkpoint);
        CheckpointFinalized(Hash, Checkpoint),
        // (exit_id: Hash)
        ExitFinalized(Hash),
        // (new_range: Range)
        DepositedRangeExtended(Range),
        // (removed_range: Range)
        DepositedRangeRemoved(Range),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            migrate::<T>();
            SimpleDispatchInfo::default().weigh_data(())
        }

        // Commitment callable methods. ========

        /// Submit root hash of Plasma chain.
        #[weight = SimpleDispatchInfo::default()]
        fn submit_root(origin, plapps_id: T::AccountId,
            blk_number: u64, root: T::Hash) {
        }

        /// verifyInclusion method verifies inclusion of message in Double Layer Tree.
        /// The message has range and token address and these also must be verified.
        /// Please see https://docs.plasma.group/projects/spec/en/latest/src/01-core/double-layer-tree.html for further details.
        /// - @param _leaf a message to verify its inclusion
        /// - @param _tokenAddress token address of the message
        /// - @param _range range of the message
        /// - @param _inclusionProof The proof data to verify inclusion
        /// - @param _blkNumber block number where the Merkle root is stored
        #[weight = SimpleDispatchInfo::default()]
        fn verifyInclusion(origin, plapps_id: T::AccountId,
            leaf: T::Hash, address, T::AccountId, range: RangeOf<T>, inclusionProof: InclusionProof, blk_number) {
        }


         // Deposit callable methods. ========
         /// deposit ERC20 token to deposit contract with initial state.
         /// following https://docs.plasma.group/projects/spec/en/latest/src/02-contracts/deposit-contract.html#deposit
         /// - @param _amount to deposit
         /// - @param _initialState The initial state of deposit
         fn deposit(origin, plapps_id: T::AccountId,
            amount: BalanceOf<T>, initial_state: T::Property) {

         }

         fn extend_deposited_ranges(origin, plapps_id: T::AccountId,
            amount: BalanceOf<T>) {

         }

         fn remove_deposited_range(origin, plapps_id: T::AccountId,
            range: RangeOf<T>, depositedRangeId: BalanceOf<T>) {

         }

         /// finalizeCheckpoint
         /// - @param _checkpointProperty A property which is instance of checkpoint predicate
         /// its first input is range to create checkpoint and second input is property for stateObject.
         fn finalize_check_point(origin, plapps_id: T::AccountId,
            checkpoint_property: T::Property) {

         }

         /// finalizeExit
         /// - @param _exitProperty A property which is instance of exit predicate and its inputs are range and StateUpdate that exiting account wants to withdraw.
         /// _exitProperty can be a property of ether ExitPredicate or ExitDepositPredicate.
         /// - @param _depositedRangeId Id of deposited range
         /// - @return return StateUpdate of exit property which is finalized.
         /// - @dev The steps of finalizeExit.
         /// 1. Serialize exit property
         /// 2. check the property is decided by Adjudication Contract.
         /// 3. Transfer asset to payout contract corresponding to StateObject.
         ///
         /// Please alse see https://docs.plasma.group/projects/spec/en/latest/src/02-contracts/deposit-contract.html#finalizeexit
         fn finalizeExit(origin, plapps_id: T::AccountId,
            exit_property: T::Property, deposited_range_id: BalanceOf<T>) {

         }
    }
}

fn migrate<T: Trait>() {
    // TODO: When runtime upgrade, migrate stroage.
    // if let Some(current_era) = CurrentEra::get() {
    //     let history_depth = HistoryDepth::get();
    //     for era in current_era.saturating_sub(history_depth)..=current_era {
    //         ErasStartSessionIndex::migrate_key_from_blake(era);
    //     }
    // }
}
