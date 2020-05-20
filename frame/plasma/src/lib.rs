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
    traits::Currency,
    weights::{SimpleDispatchInfo, WeighData, Weight},
    StorageDoubleMap, StorageMap,
};
use frame_system::{self as system, ensure_signed};
use sp_core::crypto::UncheckedFrom;
use sp_runtime::{
    traits::{Hash, One},
    RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*, vec::Vec};

use pallet_ovm::{Property, PropertyOf};
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
pub struct Exit<AccountId, Range, BlockNumber, Balance> {
    state_update: StateUpdate<AccountId, Range, BlockNumber>,
    inclusion_proof: InclusionProof<AccountId, Balance>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct InclusionProof<AccountId, Balance> {
    address_inclusion_proof: AddressInclusionProof<AccountId, Balance>,
    interval_inclusion_proof: IntervalInclusionProof<Balance>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct IntervalInclusionProof<Balance> {
    leaf_index: Balance,
    leaf_position: Balance,
    sibilings: Vec<IntervalTreeNode<Balance>>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct AddressInclusionProof<AccountId, Balance> {
    leaf_index: AccountId,
    leaf_position: Balance,
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
pub type InclusionProofOf<T> = InclusionProof<<T as frame_system::Trait>::AccountId, BalanceOf<T>>;

pub trait PlappsAddressFor<Hash, AccountId> {
    fn plapps_address_for(hash: &Hash, origin: &AccountId) -> AccountId;
}

/// Simple plapps address determiner.
///
/// Address calculated from the code (of the constructor), input data to the constructor,
/// and the account id that requested the account creation.
///
/// Formula: `blake2_256(blake2_256(code) + blake2_256(data) + origin)`
pub struct SimpleAddressDeterminer<T: Trait>(PhantomData<T>);
impl<T: Trait> PlappsAddressFor<T::Hash, T::AccountId> for SimpleAddressDeterminer<T>
where
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
{
    fn plapps_address_for(hash: &T::Hash, origin: &T::AccountId) -> T::AccountId {
        let mut buf = Vec::new();
        buf.extend_from_slice(hash.as_ref());
        buf.extend_from_slice(origin.as_ref());

        UncheckedFrom::unchecked_from(T::Hashing::hash(&buf[..]))
    }
}

pub trait Trait: system::Trait {
    /// Plasma Range's currency.
    type Currency: Currency<Self::AccountId>;

    /// A function type to get the contract address given the instantiator.
    type DeterminePlappsAddress: PlappsAddressFor<Self::Hash, Self::AccountId>;

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Plasma {
        // Commitment storage: Plapps address => Commitment Child Storage. ====

        /// Single aggregator address: AggregatorId
        AggregatorAddress get(fn aggregator_address): map hasher(twox_64_concat) T::AccountId => T::AccountId;
        /// Current block number of commitment chain: BlockNumber
        CurrentBlock get(fn current_block): map hasher(twox_64_concat) T::AccountId => T::BlockNumber;
        /// History of Merkle Root
        Blocks get(fn blocks): double_map hasher(twox_64_concat) T::AccountId, hasher(blake2_128_concat) T::BlockNumber => T::Hash;


        // Deposit storage: Plapps address => Deposit Child Storage. ====
        /// mapping from Plapps address to ERC20 based contract address.
        ERC20 get(fn erc20): map hasher(twox_64_concat) T::AccountId => T::AccountId;
        /// mapping from Plapps address to StateUpdate predicate address.
        StateUpdatePredicate get(fn state_update_predicate): map hasher(twox_64_concat) T::AccountId => T::AccountId;
        /// mapping from Plapps address to Exit predicate address.
        ExitPredicate get(fn exit_predicate): map hasher(twox_64_concat) T::AccountId => T::AccountId;
        /// mapping from Plapps address to ExitDeposit predicate address.
        ExitDepositPredicate get(fn exit_deposit_predicate): map hasher(twox_64_concat) T::AccountId => T::AccountId;

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
        /// Deplpoyed Plapps. (creator: AccountId, plapps_id: AccountId)
        Deploy(AccountId, AccountId),
        /// Event definitions (AccountID: PlappsAddress, BlockNumber, Hash: root)
        BlockSubmitted(AccountId, BlockNumber, Hash),
        /// (checkpointId: Hash, checkpoint: Checkpoint);
        CheckpointFinalized(Hash, Checkpoint),
        /// (exit_id: Hash)
        ExitFinalized(Hash),
        /// (new_range: Range)
        DepositedRangeExtended(Range),
        /// (removed_range: Range)
        DepositedRangeRemoved(Range),
    }
);

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Sender isn't valid aggregator.
        IsNotAggregator,
        /// blkNumber should be next block.
        BlockNumberShouldBeNextBlock,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_runtime_upgrade() -> Weight {
            migrate::<T>();
            SimpleDispatchInfo::default().weigh_data(())
        }

        /// Commitment constructor + Deposit constructor
        #[weight = SimpleDispatchInfo::default()]
        fn deploy(
            origin,
            aggregator_id: T::AccountId,
            erc20: T::AccountId,
            state_update_predicate: T::AccountId,
            exit_predicate: T::AccountId,
            exit_deposit_predicate: T::AccountId,
        ) {
            let sender = ensure_signed(origin)?;
            let plapps_hash = T::Hashing::hash_of(&(
                T::Hashing::hash_of(&aggregator_id),
                T::Hashing::hash_of(&erc20),
                T::Hashing::hash_of(&state_update_predicate),
                T::Hashing::hash_of(&exit_predicate),
                T::Hashing::hash_of(&exit_deposit_predicate),
            ));
            let plapps_id = T::DeterminePlappsAddress::plapps_address_for(&plapps_hash, &sender);
            <AggregatorAddress<T>>::insert(&plapps_id, aggregator_id);
            <ERC20<T>>::insert(&plapps_id, erc20);
            <StateUpdatePredicate<T>>::insert(&plapps_id, state_update_predicate);
            <ExitPredicate<T>>::insert(&plapps_id, exit_predicate);
            <ExitDepositPredicate<T>>::insert(&plapps_id, exit_deposit_predicate);
            Self::deposit_event(RawEvent::Deploy(sender, plapps_id));
        }

        // Commitment callable methods. ========

        /// Submit root hash of Plasma chain.
        #[weight = SimpleDispatchInfo::default()]
        fn submit_root(origin, plapps_id: T::AccountId,
            block_number: T::BlockNumber, root: T::Hash) {
            let aggregator = ensure_signed(origin)?;
            Self::ensure_aggregator(&plapps_id, &aggregator)?;
            ensure!(
                Self::current_block(&plapps_id) + T::BlockNumber::one() == block_number,
                Error::<T>::BlockNumberShouldBeNextBlock,
            );

            <Blocks<T>>::insert(&plapps_id, &block_number, root.clone());
            <CurrentBlock<T>>::insert(&plapps_id, block_number.clone());
            Self::deposit_event(RawEvent::BlockSubmitted(plapps_id, block_number, root));
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
        fn verify_inclusion(origin, plapps_id: T::AccountId,
            leaf: T::Hash, address: T::AccountId, range: RangeOf<T>, inclusion_proof: InclusionProofOf<T>, block_number: T::BlockNumber) {
        }


        // Deposit callable methods. ========

        /// deposit ERC20 token to deposit contract with initial state.
        /// following https://docs.plasma.group/projects/spec/en/latest/src/02-contracts/deposit-contract.html#deposit
        /// - @param amount to deposit
        /// - @param initial_state The initial state of deposit
        #[weight = SimpleDispatchInfo::default()]
        fn deposit(origin, plapps_id: T::AccountId,
            amount: BalanceOf<T>, initial_state: PropertyOf<T>) {

        }

        #[weight = SimpleDispatchInfo::default()]
        fn extend_deposited_ranges(origin, plapps_id: T::AccountId, amount: BalanceOf<T>) {

        }

        #[weight = SimpleDispatchInfo::default()]
        fn remove_deposited_range(origin, plapps_id: T::AccountId,
            range: RangeOf<T>, deposited_range_id: BalanceOf<T>) {

        }

        /// finalizeCheckpoint
        /// - @param _checkpointProperty A property which is instance of checkpoint predicate
        /// its first input is range to create checkpoint and second input is property for stateObject.
        #[weight = SimpleDispatchInfo::default()]
        fn finalize_check_point(origin, plapps_id: T::AccountId,
            checkpoint_property: PropertyOf<T>) {

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
        #[weight = SimpleDispatchInfo::default()]
        fn finalize_exit(origin, plapps_id: T::AccountId,
            exit_property: PropertyOf<T>, deposited_range_id: BalanceOf<T>) {

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

impl<T: Trait> Module<T> {
    fn ensure_aggregator(plapps_id: &T::AccountId, sender: &T::AccountId) -> DispatchResult {
        ensure!(
            sender != &Self::aggregator_address(plapps_id),
            Error::<T>::IsNotAggregator,
        );
        Ok(())
    }
}
