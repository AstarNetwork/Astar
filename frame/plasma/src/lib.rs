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
    weights::Weight,
    StorageDoubleMap, StorageMap,
};
use frame_system::{self as system, ensure_signed};
use pallet_contracts::Gas;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::{
    traits::{Bounded, Hash, One, SaturatedConversion, Saturating, Zero},
    DispatchError, RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*, vec::Vec};

pub use pallet_ovm::{Decision, Property, PropertyOf};

mod deserializer;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use deserializer::Deserializer;
pub type DispatchResultT<T> = Result<T, DispatchError>;

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Range<Balance> {
    start: Balance,
    end: Balance,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct StateUpdate<AccountId, Balance, BlockNumber> {
    deposit_contract_address: AccountId,
    range: Range<Balance>,
    block_number: BlockNumber,
    state_object: Property<AccountId>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Checkpoint<AccountId> {
    state_update: Property<AccountId>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct Exit<AccountId, BlockNumber, Balance, Hash> {
    state_update: StateUpdate<AccountId, Balance, BlockNumber>,
    inclusion_proof: InclusionProof<AccountId, Balance, Hash>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct InclusionProof<AccountId, Balance, Hash> {
    address_inclusion_proof: AddressInclusionProof<AccountId, Balance, Hash>,
    interval_inclusion_proof: IntervalInclusionProof<Balance, Hash>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct IntervalInclusionProof<Balance, Hash> {
    leaf_index: Balance,
    leaf_position: Balance,
    siblings: Vec<IntervalTreeNode<Balance, Hash>>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct AddressInclusionProof<AccountId, Balance, Hash> {
    leaf_index: AccountId,
    leaf_position: Balance,
    siblings: Vec<AddressTreeNode<AccountId, Hash>>,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct IntervalTreeNode<Balance, Hash> {
    data: Hash,
    start: Balance,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct AddressTreeNode<AccountId, Hash> {
    data: Hash,
    token_address: AccountId,
}

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
pub struct ExitDeposit<AccountId, Balance, BlockNumber> {
    state_update: StateUpdate<AccountId, Balance, BlockNumber>,
    checkpoint: Checkpoint<AccountId>,
}

pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type CheckpointOf<T> = Checkpoint<<T as frame_system::Trait>::AccountId>;
pub type ExitDepositOf<T> = ExitDeposit<
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    <T as frame_system::Trait>::BlockNumber,
>;
pub type RangeOf<T> = Range<BalanceOf<T>>;
pub type ExitOf<T> = Exit<
    <T as frame_system::Trait>::AccountId,
    <T as frame_system::Trait>::BlockNumber,
    BalanceOf<T>,
    <T as frame_system::Trait>::Hash,
>;
pub type StateUpdateOf<T> = StateUpdate<
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    <T as frame_system::Trait>::BlockNumber,
>;
pub type InclusionProofOf<T> = InclusionProof<
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    <T as frame_system::Trait>::Hash,
>;
pub type IntervalInclusionProofOf<T> =
    IntervalInclusionProof<BalanceOf<T>, <T as frame_system::Trait>::Hash>;
pub type IntervalTreeNodeOf<T> = IntervalTreeNode<BalanceOf<T>, <T as frame_system::Trait>::Hash>;
pub type AddressInclusionProofOf<T> = AddressInclusionProof<
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    <T as frame_system::Trait>::Hash,
>;
pub type AddressTreeNodeOf<T> =
    AddressTreeNode<<T as frame_system::Trait>::AccountId, <T as frame_system::Trait>::Hash>;
pub trait PlappsAddressFor<Hash, AccountId> {
    fn plapps_address_for(hash: &Hash, origin: &AccountId) -> AccountId;
}

/// Simple plapps address determiner.
///
/// Address calculated from the code (of the constructor), input data to the constructor,
/// and the account id that requested the account creation.
///
/// Formula: `blake2_256(plapps_hash + origin)`
/// ```plapps_hash = blake2_256(&(
///     blake2_256(&aggregator_id),
///     blake2_256(&balances),
///     blake2_256(&state_update_predicate),
///     blake2_256(&exit_predicate),
///     blake2_256(&exit_deposit_predicate),
// ));```
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

pub trait Trait: pallet_ovm::Trait + pallet_contracts::Trait {
    /// Plasma Range's currency.
    type Currency: Currency<Self::AccountId>;

    /// A function type to get the contract address given the instantiator.
    type DeterminePlappsAddress: PlappsAddressFor<Self::Hash, Self::AccountId>;

    /// The using initial right over token address.
    type MaximumTokenAddress: Get<Self::AccountId>;

    /// The hashing system (algorithm) being used in the Plasma module (e.g. Keccak256).
    type PlasmaHashing: Hash<Output = Self::Hash>;

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
        Checkpoints get(fn checkpoints): double_map hasher(twox_64_concat) T::AccountId, hasher(blake2_128_concat) T::Hash => bool;

        /// predicate address => payout address
        Payout get(fn payout): map hasher(twox_64_concat) T::AccountId => T::AccountId;
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
        /// (AccountID: PlappsAddress, checkpointId: Hash, checkpoint: Checkpoint);
        CheckpointFinalized(AccountId, Hash, Checkpoint),
        /// (AccountID: PlappsAddress, exit_id: Hash)
        ExitFinalized(AccountId, Hash),
        /// (AccountID: PlappsAddress, new_range: Range)
        DepositedRangeExtended(AccountId, Range),
        /// (AccountID: PlappsAddress, removed_range: Range)
        DepositedRangeRemoved(AccountId, Range),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        const MaximumTokenAddress: <T as system::Trait>::AccountId = T::MaximumTokenAddress::get();

        fn on_runtime_upgrade() -> Weight {
            migrate::<T>();
            T::MaximumBlockWeight::get()
        }

        /// Commitment constructor + Deposit constructor
        /// TODO: weight
        #[weight = 100_000]
        fn deploy(
            origin,
            aggregator_id: T::AccountId,
            erc20: T::AccountId,
            state_update_predicate: T::AccountId,
            exit_predicate: T::AccountId,
            exit_deposit_predicate: T::AccountId,
        ) {
            let sender = ensure_signed(origin)?;
            let plapps_hash = Self::generate_plapps_hash(
                &aggregator_id,
                &erc20,
                &state_update_predicate,
                &exit_predicate,
                &exit_deposit_predicate,
            );
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
        /// TODO: weight
        #[weight = 100_000]
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

        /// deposit ERC20 token to deposit contract with initial state.
        /// following https://docs.plasma.group/projects/spec/en/latest/src/02-contracts/deposit-contract.html#deposit
        /// - @param amount to deposit
        /// - @param initial_state The initial state of deposit
        /// TODO: weight
        #[weight = 100_000]
        fn deposit(origin, plapps_id: T::AccountId,
            amount: BalanceOf<T>, initial_state: PropertyOf<T>, gas_limit: Gas) {
            let _ = ensure_signed(origin)?;
            let total_deposited = Self::total_deposited(&plapps_id);
            ensure!(
                total_deposited < BalanceOf::<T>::max_value().saturating_sub(amount),
                Error::<T>::TotalDepositedExceedMaxBalance,
            );
            // TODO: transfer_from origin -> plapps_id (amount) at balances.
            // let _ = contracts::bare_call(
            //     origin,
            //     Self::balances(&plapps_id),fAccountID
            //     BalanceOf<T>::zero(),
            //     gas_limit,
            //     "transfer_from(origin, plapps_id, amount)",
            // )?;

            let deposit_range = RangeOf::<T> {
                start: total_deposited,
                end: total_deposited.saturating_add(amount.clone()),
            };
            let state_update = PropertyOf::<T> {
                predicate_address: Self::state_update_predicate(&plapps_id),
                inputs: vec![
                    plapps_id.encode(),
                    deposit_range.encode(),
                    Self::get_latest_plasma_block_number(&plapps_id).encode(),
                    initial_state.encode(),
                ],
            };
            let checkpoint = Checkpoint {
                state_update: state_update,
            };
            Self::bare_extend_deposited_ranges(&plapps_id, amount);
            let checkpoint_id = Self::get_checkpoint_id(&checkpoint);
            <Checkpoints<T>>::insert(plapps_id.clone(), &checkpoint_id, true);
            Self::deposit_event(RawEvent::CheckpointFinalized(plapps_id, checkpoint_id, checkpoint));
        }

        /// TODO: weight, not external
        #[weight = 100_000]
        fn extend_deposited_ranges(origin, plapps_id: T::AccountId, amount: BalanceOf<T>) {
            ensure_signed(origin)?;
            Self::bare_extend_deposited_ranges(&plapps_id, amount);
        }

        /// TODO: weight, not external
        #[weight = 100_000]
        fn remove_deposited_range(origin, plapps_id: T::AccountId,
            range: RangeOf<T>, deposited_range_id: BalanceOf<T>) {
            ensure_signed(origin)?;
            Self::bare_remove_deposited_range(
                &plapps_id,
                &range,
                &deposited_range_id,
            )?;
        }

        /// finalizeCheckpoint
        /// - @param _checkpointProperty A property which is instance of checkpoint predicate
        /// its first input is range to create checkpoint and second input is property for stateObject.
        /// TODO: weight
        #[weight = 100_000]
        fn finalize_checkpoint(origin, plapps_id: T::AccountId,
            checkpoint_property: PropertyOf<T>) {
            ensure!(
                <pallet_ovm::Module<T>>::is_decided(&checkpoint_property) != Decision::True,
                Error::<T>::ClaimMustBeDecided,
            );
            let property: PropertyOf<T> = Decode::decode(&mut &checkpoint_property.inputs[0][..])
                .map_err(|_| Error::<T>::MustBeDecodable)?;
            let checkpoint = Checkpoint {
                state_update: property,
            };

            let checkpoint_id = Self::get_checkpoint_id(&checkpoint);
            // store the checkpoint
            <Checkpoints<T>>::insert(&plapps_id, &checkpoint_id, true);
            Self::deposit_event(RawEvent::CheckpointFinalized(plapps_id, checkpoint_id, checkpoint));
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
        /// TODO: weight
        #[weight = 50_000_000]
        fn finalize_exit(origin, plapps_id: T::AccountId,
            exit_property: PropertyOf<T>, deposited_range_id: BalanceOf<T>, _owner: T::AccountId) {
            let origin = ensure_signed(origin)?;
            let state_update = Self::bare_finalize_exit(
                &plapps_id,
                &exit_property,
                &deposited_range_id
            )?;
            let owner: T::AccountId = Decode::decode(&mut &state_update.state_object.inputs[0][..])
                .map_err(|_| Error::<T>::MustBeDecodable)?;
            let _amount = state_update.range.end - state_update.range.start;
            ensure!(
                origin == owner,
                Error::<T>::OriginMustBeOwner,
            );
            // TODO: finalize_exit payout -> owner[state_update.state_objects.inputs[0]] (amount[state_update.range]) at payout.
            // let _ = contracts::bare_call(
            //     plapps_id,
            //     Self::payout(&plapps_id),
            //     BalanceOf<T>::zero(),
            //     gas_limit,
            //     "finalize_exit(state_update)",
            // )?;
        }
    }
}

decl_error! {
    /// Error for the staking module.
    pub enum Error for Module<T: Trait> {
        /// Sender isn't valid aggregator.
        IsNotAggregator,
        /// blkNumber should be next block.
        BlockNumberShouldBeNextBlock,
        /// leftStart must be less than _rightStart
        LeftMustBeLessThanRight,
        /// firstRightSiblingStart must be greater than siblingStart
        FirstRightMustBeGreaterThanSibling,
        /// required range must not exceed the implicit range
        RangeMustNotExceedTheImplicitRange,
        /// required address must not exceed the implicit address
        AddressMustNotExceedTheImplicitAddress,
        /// DepositContract: totalDeposited exceed max uint256
        TotalDepositedExceedMaxBalance,
        /// must approved
        MustApproved,
        /// range must be of a depostied range (the one that has not been exited
        RangeMustBeOfDepositedRange,
        /// Checkpointing claim must be decied
        ClaimMustBeDecided,
        /// Must decode from checkpointInputs[0] to Property.
        MustBeDecodable,
        /// Exit must be decided after this block
        ExitMustBeDecided,
        /// finalizeExit must be called from payout contract
        FinalizeExitMustBeCalledFromPayout,
        /// origin must be owner
        OriginMustBeOwner,
        /// checkpoint must be finalized
        CheckpointMustBeFinalized,
        /// depositContractAddress must be same
        DepositContractAddressMustBeSame,
        /// blockNumber must be same,
        BlockNumberMustBeSame,
        /// range must be subrange of checkpoint
        RangeMustBeSubrangeOfCheckpoint,
        /// StateUpdate.depositContractAddress must be this contract address
        DepositContractAddressMustBePlappsId,
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

/// Public callable Plasma commitment module methods.
impl<T: Trait> Module<T> {
    // Plasma Commitment parts ====
    pub fn retrieve(plapps_id: T::AccountId, block_number: T::BlockNumber) -> T::Hash {
        <Blocks<T>>::get(&plapps_id, &block_number)
    }

    /// verifyInclusion method verifies inclusion of message in Double Layer Tree.
    /// The message has range and token address and these also must be verified.
    /// Please see https://docs.plasma.group/projects/spec/en/latest/src/01-core/double-layer-tree.html for further details.
    /// - @param leaf a message to verify its inclusion
    /// - @param token_address token address of the message
    /// - @param range range of the message
    /// - @param inclusion_proof The proof data to verify inclusion
    /// - @param block_number block number where the Merkle root is stored
    pub fn verify_inclusion(
        plapps_id: T::AccountId,
        leaf: T::Hash,
        token_address: T::AccountId,
        range: RangeOf<T>,
        inclusion_proof: InclusionProofOf<T>,
        block_number: T::BlockNumber,
    ) -> DispatchResultT<bool> {
        let root = <Blocks<T>>::get(&plapps_id, &block_number);
        Self::verify_inclusion_with_root(leaf, token_address, range, inclusion_proof, root)
    }

    pub fn verify_inclusion_with_root(
        leaf: T::Hash,
        token_address: T::AccountId,
        range: RangeOf<T>,
        inclusion_proof: InclusionProofOf<T>,
        root: T::Hash,
    ) -> DispatchResultT<bool> {
        // Calcurate the root of interval tree
        let (computed_root, implicit_end) = Self::compute_interval_tree_root(
            &leaf,
            &inclusion_proof.interval_inclusion_proof.leaf_index,
            &inclusion_proof.interval_inclusion_proof.leaf_position,
            &inclusion_proof.interval_inclusion_proof.siblings,
        )?;

        ensure!(
            range.start >= inclusion_proof.interval_inclusion_proof.leaf_index
                && range.end <= implicit_end,
            Error::<T>::RangeMustNotExceedTheImplicitRange,
        );

        // Calcurate the root of address tree
        let (computed_root, implicit_address) = Self::compute_address_tree_root(
            &computed_root,
            &token_address,
            &inclusion_proof.address_inclusion_proof.leaf_position,
            &inclusion_proof.address_inclusion_proof.siblings,
        )?;

        ensure!(
            token_address <= implicit_address,
            Error::<T>::AddressMustNotExceedTheImplicitAddress,
        );
        return Ok(computed_root == root);
    }
}

/// Private(Helper) Plasma commitment module methods.
impl<T: Trait> Module<T> {
    // Plasma Commitment Parts
    fn ensure_aggregator(sender: &T::AccountId, plapps_id: &T::AccountId) -> DispatchResult {
        ensure!(
            sender != &Self::aggregator_address(plapps_id),
            Error::<T>::IsNotAggregator,
        );
        Ok(())
    }

    /// @dev computeIntervalTreeRoot method calculates the root of Interval Tree.
    /// Please see https://docs.plasma.group/projects/spec/en/latest/src/01-core/merkle-interval-tree.html for further details.
    fn compute_interval_tree_root(
        computed_root: &T::Hash,
        computed_start: &BalanceOf<T>,
        interval_tree_merkle_path: &BalanceOf<T>,
        interval_tree_proof: &Vec<IntervalTreeNodeOf<T>>,
    ) -> DispatchResultT<(T::Hash, BalanceOf<T>)> {
        let mut first_right_sibling_start = BalanceOf::<T>::max_value();
        let mut is_first_right_sibling_start_set = false;
        let mut ret_computed_root: T::Hash = computed_root.clone();
        let mut ret_computed_start: BalanceOf<T> = computed_start.clone();
        for (i, node) in interval_tree_proof.iter().enumerate() {
            let sibling = &node.data;
            let sibling_start = &node.start;
            let is_computed_right_sibling =
                interval_tree_merkle_path.clone().saturated_into::<usize>() >> i;
            if is_computed_right_sibling == 1 {
                ret_computed_root = Self::get_parent(
                    sibling,
                    sibling_start,
                    &ret_computed_root,
                    &ret_computed_start,
                )?;
            } else {
                if !is_first_right_sibling_start_set {
                    first_right_sibling_start = sibling_start.clone();
                    is_first_right_sibling_start_set = true;
                }
                ensure!(
                    &first_right_sibling_start <= sibling_start,
                    Error::<T>::FirstRightMustBeGreaterThanSibling,
                );
                ret_computed_root = Self::get_parent(
                    &ret_computed_root,
                    &ret_computed_start,
                    sibling,
                    sibling_start,
                )?;
                ret_computed_start = sibling_start.clone();
            }
        }
        Ok((ret_computed_root, first_right_sibling_start))
    }

    /// @dev computeAddressTreeRoot method calculates the root of Address Tree.
    /// Address Tree is almost the same as Merkle Tree.
    /// But leaf has their address and we can verify the address each leaf belongs to.
    fn compute_address_tree_root(
        computed_root: &T::Hash,
        compute_address: &T::AccountId,
        address_tree_merkle_path: &BalanceOf<T>,
        address_tree_proof: &Vec<AddressTreeNodeOf<T>>,
    ) -> DispatchResultT<(T::Hash, T::AccountId)> {
        let mut first_right_sibling_address = T::MaximumTokenAddress::get();
        let mut is_first_right_sibling_address_set = false;
        let mut ret_computed_root: T::Hash = computed_root.clone();
        let mut ret_compute_address: T::AccountId = compute_address.clone();
        for (i, node) in address_tree_proof.iter().enumerate() {
            let sibling = &node.data;
            let sibling_address = &node.token_address;
            let is_computed_right_sibling =
                (address_tree_merkle_path.clone().saturated_into::<usize>() >> i) & 1;
            if is_computed_right_sibling == 1 {
                ret_computed_root = Self::get_parent_of_address_tree_node(
                    sibling,
                    sibling_address,
                    &ret_computed_root,
                    &ret_compute_address,
                );
                ret_compute_address = sibling_address.clone();
            } else {
                if !is_first_right_sibling_address_set {
                    first_right_sibling_address = sibling_address.clone();
                    is_first_right_sibling_address_set = true;
                }
                ensure!(
                    &first_right_sibling_address <= sibling_address,
                    Error::<T>::FirstRightMustBeGreaterThanSibling,
                );
                ret_computed_root = Self::get_parent_of_address_tree_node(
                    &ret_computed_root,
                    &ret_compute_address,
                    sibling,
                    sibling_address,
                );
            }
        }
        Ok((ret_computed_root, first_right_sibling_address))
    }

    fn get_parent(
        left: &T::Hash,
        left_start: &BalanceOf<T>,
        right: &T::Hash,
        right_start: &BalanceOf<T>,
    ) -> DispatchResultT<T::Hash> {
        ensure!(
            right_start >= left_start,
            Error::<T>::LeftMustBeLessThanRight,
        );
        return Ok(T::PlasmaHashing::hash_of(&(
            left,
            left_start,
            right,
            right_start,
        )));
    }
}

/// Public callable Plasma deposit module methods.
impl<T: Trait> Module<T> {
    pub fn bare_extend_deposited_ranges(plapps_id: &T::AccountId, amount: BalanceOf<T>) {
        let total_deposited = Self::total_deposited(plapps_id);
        let old_range = Self::deposited_ranges(plapps_id, &total_deposited);
        let new_start = if old_range.start == BalanceOf::<T>::zero()
            && old_range.end == BalanceOf::<T>::zero()
        {
            // Creat a new range when the rightmost range has been removed
            total_deposited
        } else {
            // Delete the old range and make a new one with the total length
            <DepositedRanges<T>>::remove(plapps_id, old_range.end);
            old_range.start
        };

        let new_end = total_deposited.saturating_add(amount.clone());
        let new_range = Range {
            start: new_start,
            end: new_end,
        };
        <DepositedRanges<T>>::insert(plapps_id, &new_end, new_range.clone());
        <TotalDeposited<T>>::insert(plapps_id, total_deposited.saturating_add(amount));
        Self::deposit_event(RawEvent::DepositedRangeExtended(
            plapps_id.clone(),
            new_range,
        ));
    }

    pub fn bare_remove_deposited_range(
        plapps_id: &T::AccountId,
        range: &RangeOf<T>,
        deposited_range_id: &BalanceOf<T>,
    ) -> DispatchResult {
        let deposited_ranges = Self::deposited_ranges(plapps_id, deposited_range_id);
        ensure!(
            Self::is_subrange(range, &deposited_ranges),
            Error::<T>::RangeMustBeOfDepositedRange,
        );

        /*
         * depositedRanges makes O(1) checking existence of certain range.
         * Since _range is subrange of encompasingRange, we only have to check is each start and end are same or not.
         * So, there are 2 patterns for each start and end of _range and encompasingRange.
         * There are nothing todo for _range.start is equal to encompasingRange.start.
         */
        // Check start of range
        if range.start != deposited_ranges.start {
            let left_split_range = Range {
                start: deposited_ranges.start,
                end: range.start,
            };
            <DepositedRanges<T>>::insert(plapps_id, left_split_range.end, left_split_range);
        }
        // Check end of range
        if range.end == deposited_ranges.end {
            /*
             * Deposited range Id is end value of the range, we must remove the range from depositedRanges
             *     when range.end is changed.
             */
            <DepositedRanges<T>>::remove(plapps_id, &deposited_ranges.end);
        } else {
            <DepositedRanges<T>>::insert(
                plapps_id,
                &deposited_ranges.end,
                Range {
                    start: range.end,
                    end: deposited_ranges.end,
                },
            );
        }
        Self::deposit_event(RawEvent::DepositedRangeRemoved(
            plapps_id.clone(),
            range.clone(),
        ));
        Ok(())
    }

    /// bare_finalize_exit
    /// called by this module.
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
    pub fn bare_finalize_exit(
        plapps_id: &T::AccountId,
        exit_property: &PropertyOf<T>,
        deposited_range_id: &BalanceOf<T>,
    ) -> DispatchResultT<StateUpdateOf<T>> {
        let state_update = Self::verify_exit_property(plapps_id, exit_property)?;
        let exit_id = Self::get_exit_id(exit_property);
        // get payout contract address
        let _payout = Self::payout(plapps_id);

        // Check that we are authorized to finalize this exit
        ensure!(
            <pallet_ovm::Module<T>>::is_decided(exit_property) != Decision::True,
            Error::<T>::ExitMustBeDecided,
        );

        ensure!(
            &state_update.deposit_contract_address == plapps_id,
            Error::<T>::DepositContractAddressMustBePlappsId,
        );

        // Remove the deposited range
        Self::bare_remove_deposited_range(plapps_id, &state_update.range, deposited_range_id)?;
        // Transfer tokens to its predicate
        let _amount = state_update.range.end - state_update.range.start;

        // TODO: transfer plapps_id -> payout (amount) at balances.
        // let _ = contracts::bare_call(
        //     origin,
        //     Self::balances(&plapps_id),
        //     BalanceOf<T>::zero(),
        //     gas_limit,
        //     "transfer(payout, amount)",
        // )?;
        Self::deposit_event(RawEvent::ExitFinalized(plapps_id.clone(), exit_id));
        Ok(state_update)
    }

    /// @dev verify StateUpdate in Exit property.
    /// _exitProperty must be instance of ether ExitPredicate or ExitDepositPredicate.
    /// if _exitProperty is instance of ExitDepositPredicate, check _exitProperty.su is subrange of _exitProperty.checkpoint.
    pub fn verify_exit_property(
        plapps_id: &T::AccountId,
        exit_property: &PropertyOf<T>,
    ) -> DispatchResultT<StateUpdateOf<T>> {
        if exit_property.predicate_address == Self::exit_predicate(plapps_id) {
            let exit: ExitOf<T> = <Deserializer<T>>::deserialize_exit(exit_property)?;
            // TODO: check inclusion proof
            return Ok(exit.state_update);
        } else if exit_property.predicate_address == Self::exit_deposit_predicate(plapps_id) {
            let exit_deposit = <Deserializer<T>>::deserialize_exit_deposit(exit_property)?;
            let checkpoint = exit_deposit.checkpoint;
            let state_update =
                <Deserializer<T>>::deserialize_state_update(&checkpoint.state_update)?;
            ensure!(
                Self::checkpoints(plapps_id, Self::get_checkpoint_id(&checkpoint)),
                Error::<T>::CheckpointMustBeFinalized,
            );
            ensure!(
                state_update.deposit_contract_address
                    == exit_deposit.state_update.deposit_contract_address,
                Error::<T>::DepositContractAddressMustBeSame,
            );
            ensure!(
                state_update.block_number == exit_deposit.state_update.block_number,
                Error::<T>::BlockNumberMustBeSame,
            );
            ensure!(
                Self::is_subrange(&exit_deposit.state_update.range, &state_update.range),
                Error::<T>::RangeMustBeSubrangeOfCheckpoint,
            );
            return Ok(exit_deposit.state_update);
        }
        Err(DispatchError::Other(
            "verify_exit_property not return value.",
        ))
    }
}

/// Priavte(Helper) callable Plasma deposit module methods.
impl<T: Trait> Module<T> {
    fn get_parent_of_address_tree_node(
        left: &T::Hash,
        left_address: &T::AccountId,
        right: &T::Hash,
        right_address: &T::AccountId,
    ) -> T::Hash {
        T::PlasmaHashing::hash_of(&(left, left_address, right, right_address))
    }

    fn get_latest_plasma_block_number(plapps_id: &T::AccountId) -> T::BlockNumber {
        return Self::current_block(&plapps_id);
    }

    fn get_checkpoint_id(checkpoint: &CheckpointOf<T>) -> T::Hash {
        <T as system::Trait>::Hashing::hash_of(checkpoint)
    }

    fn get_exit_id(exit: &PropertyOf<T>) -> T::Hash {
        <T as system::Trait>::Hashing::hash_of(exit)
    }

    fn is_subrange(subrange: &RangeOf<T>, surrounding_range: &RangeOf<T>) -> bool {
        subrange.start >= surrounding_range.start && subrange.end <= surrounding_range.end
    }

    fn generate_plapps_hash(
        aggregator_id: &T::AccountId,
        erc20: &T::AccountId,
        state_update_predicate: &T::AccountId,
        exit_predicate: &T::AccountId,
        exit_deposit_predicate: &T::AccountId,
    ) -> T::Hash {
        T::Hashing::hash_of(&(
            T::Hashing::hash_of(&aggregator_id),
            T::Hashing::hash_of(&erc20),
            T::Hashing::hash_of(&state_update_predicate),
            T::Hashing::hash_of(&exit_predicate),
            T::Hashing::hash_of(&exit_deposit_predicate),
        ))
    }
}
