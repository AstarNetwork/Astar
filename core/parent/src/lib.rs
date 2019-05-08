#![cfg_attr(not(feature = "std"), no_std)]

pub mod mvp;

/// substrate
use support::{Parameter, dispatch::Result};
use sr_primitives::traits::{Member, Bounded, SimpleArithmetic, As, MaybeSerializeDebug, Hash};

use parity_codec::{Encode, Decode, Codec, Input, Output};

/// rst
use rstd::prelude::*;

/// plasm
use merkle::ProofTrait;

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ExitState {
	Exiting,
	Challenging,
	Challenged,
	Finalized,
}

const EXIT_STATE_EXITING: u8 = 1;
const EXIT_STATE_CHALLENGING: u8 = 2;
const EXIT_STATE_CHALLENGED: u8 = 4;
const EXIT_STATE_FINALIZED: u8 = 8;

impl Encode for ExitState {
	fn encode_to<T: Output>(&self, output: &mut T) {
		match self {
			ExitState::Exiting => output.push_byte(EXIT_STATE_EXITING),
			ExitState::Challenging => output.push_byte(EXIT_STATE_CHALLENGING),
			ExitState::Challenged => output.push_byte(EXIT_STATE_CHALLENGED),
			ExitState::Finalized => output.push_byte(EXIT_STATE_FINALIZED),
		}
	}
}

impl Decode for ExitState {
	fn decode<I: Input>(input: &mut I) -> Option<Self> {
		Some(match input.read_byte()? {
			EXIT_STATE_EXITING => ExitState::Exiting,
			EXIT_STATE_CHALLENGING => ExitState::Challenging,
			EXIT_STATE_CHALLENGED => ExitState::Challenged,
			EXIT_STATE_FINALIZED => ExitState::Finalized,
			_ => ExitState::Exiting,
		})
	}
}

impl Default for ExitState {
	fn default() -> Self { ExitState::Exiting }
}


/// The module's configuration trait.
pub trait Trait: balances::Trait + timestamp::Trait {
	type ChildValue: Parameter + Default + As<u64>;
	type Utxo: Parameter + Default + UtxoTrait<Self::Hash, Self::ChildValue, Self::AccountId>;
	type Proof: ProofTrait<Self::Hash>;

	type ExitStatus: Parameter + Default + ExitStatusTrait<Self::BlockNumber, Self::Utxo, Self::Moment, ExitState>;
	type ChallengeStatus: Default + ChallengeStatusTrait<Self::BlockNumber, Self::Utxo>;

	type FraudProof: FraudProofTrait<Self>;
	// How to Fraud proof. to utxo from using utxo.
	type ExitorHasChcker: ExitorHasChckerTrait<Self>;
	type ExistProofs: ExistProofsTrait<Self>;
	type Exchanger: ExchangerTrait<Self::Balance, Self::ChildValue>;
	type Finalizer: FinalizerTrait<Self>;

	/// The overarching event type.
	type Event: From<mvp::Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// ExitStatusTrait
/// B: BlockNumber, P: Proof,
/// Innner Gen
pub trait ExitStatusTrait<B, U, M, S> {
	fn blk_num(&self) -> &B;
	fn utxo(&self) -> &U;
	fn started(&self) -> &M;
	fn expired(&self) -> &M;
	fn state(&self) -> &S;
	fn set_state(&mut self, s: S);
}

/// ChallengeStatusTrait
/// B: BlockNumber, U: UTXO
/// Innner Gen
pub trait ChallengeStatusTrait<B, U> {
	fn blk_num(&self) -> &B;
	fn utxo(&self) -> &U;
}

/// Utxo must be parent chain.
/// I: Inputs(references), V: ChildValue
/// Outer Gen
pub trait UtxoTrait<H, V, K>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default
{
	fn hash<Hashing: Hash<Output=H>>(&self) -> H;
	fn inputs<Hashing: Hash<Output=H>>(&self) -> Vec<H>;
	fn value(&self) -> &V;
	fn owners(&self) -> &Vec<K>;
	fn quorum(&self) -> u32;
}

/// Used UTXO by Exit = target, it challenged (fraud proof) from another UTXO.
/// E is ExitStatus, C is ChllengeStatus or proofed exitsting utxo.
pub trait FraudProofTrait<T: Trait> {
	fn verify<E, C>(target: &E, challenge: &C) -> Result
		where E: ExitStatusTrait<T::BlockNumber, T::Utxo, T::Moment, ExitState>,
			  C: ChallengeStatusTrait<T::BlockNumber, T::Utxo>;
}

/// Check exitor has UTXO.
pub trait ExitorHasChckerTrait<T: Trait> {
	fn check(exitor: &T::AccountId, utxo: &T::Utxo) -> Result;
}

/// ある UTXO の存在証明が正しいか否かを返す。
/// T: Trait, B: BlockNumber, U: UTXO, P: Proof;
pub trait ExistProofsTrait<T: Trait> {
	fn is_exist<P: ProofTrait<T::Hash>>(blk_num: &T::BlockNumber, utxo: &T::Utxo, proof: &P) -> Result;
}

/// Child Value exchanges to Parent Value.
/// P: Parent Value, C: Child Value.
pub trait ExchangerTrait<P, C> {
	fn exchange(c: C) -> P;
}

/// Finalize Check
/// T: Trait, E: ExitStatus
pub trait FinalizerTrait<T: Trait> {
	fn validate(e: &T::ExitStatus) -> Result;
}
