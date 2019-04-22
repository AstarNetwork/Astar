use super::*;

/// substrate
use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, Parameter, dispatch::Result};
use system::ensure_signed;
use sr_primitives::traits::{Member, CheckedAdd, CheckedSub, Zero, As, MaybeSerializeDebug, Hash};

use parity_codec::{Encode, Decode, Codec};

/// rst
use rstd::ops::{Div, Mul};
use rstd::prelude::*;
use rstd::marker::PhantomData;
use rstd::mem;

/// plasm
use plasm_merkle::{ProofTrait, MerkleProof};
use plasm_utxo::{Transaction, TransactionInput, TransactionOutput};


/// Utxo is H: Hash, V: ChildValue, K: AccountId, B: BlockNumber;
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Utxo<H, V, K, B>(Transaction<TransactionInput<H>, TransactionOutput<V, K>, B>, usize);

impl<H, V, K, B> UtxoTrait<H, V, K> for Utxo<H, V, K, B>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  V: Parameter,
		  K: Parameter,
		  B: Parameter {
	fn hash<Hashing: Hash<Output=H>>(&self) -> H {
		plasm_utxo::mvp::utxo_hash::<Hashing, H>(&Hashing::hash_of(&self.0), &self.1)
	}
	fn inputs<Hashing: Hash<Output=H>>(&self) -> Vec<H> {
		self.0.inputs
			.iter()
			.map(|inp| plasm_utxo::mvp::utxo_hash::<Hashing, H>(&inp.tx_hash, &inp.out_index))
			.collect::<Vec<_>>()
	}
	fn value(&self) -> &V {
		&self.0.outputs[self.1].value
	}
	fn owners(&self) -> &Vec<K> {
		&self.0.outputs[self.1].keys
	}
	fn quorum(&self) -> u32 {
		self.0.outputs[self.1].quorum
	}
}

#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct ExitStatus<H: Codec, V: Codec, K: Codec, B: Codec, U: Codec, M: Codec, S: Codec> {
	pub blk_num: B,
	pub utxo: U,
	pub started: M,
	pub expired: M,
	pub state: S,
	_phantom: PhantomData<(H, V, K)>,
}

impl<H, V, K, B, U, M> ExitStatusTrait<B, U, M, ExitState> for ExitStatus<H, V, K, B, U, M, ExitState>
	where
		H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		V: Codec,
		K: Codec,
		B: Parameter + Member + SimpleArithmetic + Default + Copy + rstd::hash::Hash,
		U: Parameter + Default + UtxoTrait<H, V, K>,
		M: Parameter + Default + SimpleArithmetic
		+ Mul<B, Output=M> + Div<B, Output=M> {
	fn blk_num(&self) -> &B { &self.blk_num }
	fn utxo(&self) -> &U { &self.utxo }
	fn started(&self) -> &M { &self.started }
	fn expired(&self) -> &M { &self.expired }
	fn state(&self) -> &ExitState { &self.state }
	fn set_state(&mut self, s: ExitState) { self.state = s; }
}

pub struct ChallengeStatus<H, K, V, B, U> {
	pub blk_num: B,
	pub utxo: U,
	_phantom: PhantomData<(H, K, V)>,
}

/// Implment ChallengeStatus
impl<H, K, V, B, U> ChallengeStatusTrait<B, U> for ChallengeStatus<H, K, V, B, U>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  B: Parameter + Member + SimpleArithmetic + Default + Bounded + Copy
		  + rstd::hash::Hash,
		  U: Parameter + Default + UtxoTrait<H, K, V> {
	fn blk_num(&self) -> &B { &self.blk_num }
	fn utxo(&self) -> &U { &self.utxo }
}


/// E: ExitStatus, C: ChallengStatus
pub struct FraudProof<T, E, C>(PhantomData<(T, E, C)>);

impl<T, E, C> FraudProofTrait<T, E, C> for FraudProof<T, E, C>
	where
		T: Trait,
		E: ExitStatusTrait<T::BlockNumber, T::Utxo, T::Moment, ExitState>,
		C: ChallengeStatusTrait<T::BlockNumber, T::Utxo> {
	fn verify(target: &E, challenge: &C) -> Result {
		// double spending check.
		if target.blk_num() > challenge.blk_num() {
			for inp in target.utxo().inputs::<T::Hashing>().iter() {
				if challenge.utxo().inputs::<T::Hashing>().contains(inp) {
					return Ok(());
				}
			}
			return Err("challenge failed, not duplicate reference.");
		}
		Err("challenge failed, block number is not lower.")
	}
}

/// E: AccountId, U: Utxo
pub struct ExitorHasChcker<T>(PhantomData<T>);

impl<T: Trait> ExitorHasChckerTrait<T> for ExitorHasChcker<T> {
	fn check(exitor: &T::AccountId, utxo: &T::Utxo) -> Result {
		if utxo.owners().contains(exitor) && utxo.quorum() == 1 {
			return Ok(());
		}
		Err("invalid exitor.")
	}
}

pub struct ExistProofs<T>(PhantomData<T>);

impl<T: Trait> ExistProofsTrait<T> for ExistProofs<T> {
	fn is_exist<P: ProofTrait<T::Hash>>(blk_num: &T::BlockNumber, utxo: &T::Utxo, proof: &P) -> Result {
		if let Some(root) = <ChildChain<T>>::get(blk_num) {
			if root != proof.root::<T::Hashing>() {
				return Err("not exist proof, invalid root hash.");
			}
			if utxo.hash::<T::Hashing>() != *proof.leaf() {
				return Err("not exit proof, invalid leaf hash.");
			}
			return Ok(());
		}
		Err("not exist proof, invalid block number.")
	}
}


/// P: Parent Value, C: ChildValue
pub struct Exchanger<P, C>(PhantomData<(P, C)>);

impl<P, C> ExchangerTrait<P, C> for Exchanger<P, C>
	where
		P: As<u64>,
		C: As<u64> {
	fn exchange(c: C) -> P {
		P::sa(c.as_())
	}
}

pub struct Finalizer<T>(PhantomData<T>);

impl<T: Trait> FinalizerTrait<T> for Finalizer<T>
	where T: Trait,
		  E: ExitStatusTrait<T::BlockNumber, T::Utxo, T::Moment, ExitState>,
{
	fn validate(e: &T::ExitStatus) -> Result {
		match e.state() {
			ExitState::Exiting => {
				if <timestamp::Module<T>>::now() > *e.expired() {
					return Ok(());
				}
				return Err("not yet challenging interval.");
			}
			ExitState::Challenged => return Err("it is challenged exits. so can not finalize."),
			ExitState::Finalized => return Err("it is finalized exit yet."),
			_ => Err("unexpected state error."),
		}
	}

	fn update(e: &T::ExitStatus) -> Result {
		let p = T::Exchanger::exchange(e.utxo().value());
		let total = <TotalDeposit<T>>::get();
		let new_total = total.checked_sub(p).is_ok("deposits value error.")?;
	}
}

const EXIT_WATING_MOMENT: u64 = 24 * 60 * 60 * 1000;

/// This module's storage items.
decl_storage! {
	trait Store for Module < T: Trait > as Parent {
		TotalDeposit get(total_deposit) config(): <T as balances::Trait>::Balance;
		ChildChain get(child_chain): map T::BlockNumber => Option<T::Hash>;
		CurrentBlock get(current_block): T::BlockNumber = T::BlockNumber::zero();
		Operator get(operator) config() : Vec <T::AccountId> = Default::default();
		ExitStatusStorage get(exit_status_storage): map T::Hash => Option<T::ExitStatus>;
		Fee get(fee) config(): <T as balances::Trait>::Balance;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		/// submit childchain merkle root to parant chain.
		pub fn submit(origin, root: T::Hash) -> Result {
			let origin = ensure_signed(origin) ?;

			// validate
			if ! Self::operator().contains( &origin) { return Err("permission error submmit can be only operator."); }
			let current = Self::current_block();
			let next = current.checked_add( & T::BlockNumber::sa(1)).ok_or("block number is overflow.")?;

			// update
			<ChildChain<T>>::insert( & next, root);
			<CurrentBlock<T>>::put(next);
			Self::deposit_event(RawEvent::Submit(root));
			Ok(())
		}

		/// deposit balance parent chain to childchain.
		pub fn deposit(origin, # [compact] value: < T as balances::Trait >::Balance) -> Result {
			let depositor = ensure_signed(origin) ?;

			// validate
			let now_balance = <balances::Module<T>>::free_balance(&depositor);
			let new_balance = now_balance.checked_sub( & value).ok_or("not enough balance.") ?;

			let now_total_deposit = Self::total_deposit();
			let new_total_deposit = now_total_deposit.checked_add(& value).ok_or("overflow total deposit.") ?;

			// update
			<balances::FreeBalance<T>>::insert(&depositor, new_balance);
			<TotalDeposit<T>>::put(new_total_deposit);
			Self::deposit_event(RawEvent::Deposit(depositor, value));
			Ok(())
		}

		/// exit balances start parent chain from childchain.
		pub fn exit_start(origin, blk_num: T::BlockNumber, depth: u32, index: u64, proofs: Vec<T::Hash>, utxo: Vec<u8>) -> Result {
			let exitor = ensure_signed(origin)?;

			// validate
			// fee check
			let fee = Self::fee();
			let now_balance = <balances::Module<T>>::free_balance(&exitor);
			let new_balance = now_balance.checked_sub(&fee).ok_or("not enough fee.") ?;

			let now_total_deposit = Self::total_deposit();
			let new_total_deposit = now_total_deposit.checked_add(&fee).ok_or("overflow total deposit.") ?;

			// exist check
			let utxo = T::Utxo::decode( &mut &utxo[..]).ok_or("undecodec utxo binary.")?;
			let depth = depth as u8;
			let proof = MerkleProof{ proofs, depth, index};
			T::ExistProofs::is_exist(&blk_num, &utxo, &proof)?;

			// owner check
			T::ExitorHasChcker::check(&exitor, &utxo);

			let exit_status = ExitStatus {
				blk_num: blk_num,
				utxo: utxo,
				started: <timestamp::Module <T>>::now(),
				expired: <timestamp::Module <T>>::now() + <T as timestamp::Trait>::Moment::sa(EXIT_WATING_MOMENT),
				state: ExitState::Exiting,
				_phantom: PhantomData::<(T::Hash, T::ChildValue, T::AccountId)>,
			};
			let exit_id = T::Hashing::hash_of(&exit_status);
			let exit_status = T::ExitStatus::decode(&mut &exit_status.encode()[..]).unwrap(); // TODO better how to.

			// update
			<balances::FreeBalance<T>>::insert(&depositor, new_balance);
			<TotalDeposit<T>>::put(new_total_deposit);
			<ExitStatusStorage<T>>::insert( &exit_id, exit_status);

			Self::deposit_event(RawEvent::ExitStart(exitor, exit_id));

			Ok(())
		}

		/// exit finalize parent chain from childchain.
		pub fn exit_finalize(origin, exitId: T::Hash) -> Result {
			ensure_signed(origin)?;

			// validate
			let exit_status = <ExitStatusStorage<T>>::get(&exit_id).is_ok("invalid exit id.")?;
			// exit status validate
			T::FinalizeChecker::check(exit_status)?;

			// total deposit validate
			let p = T::Exchanger::exchange(e.utxo().value());
			let now_total = <TotalDeposit<T>>::get();
			let new_total = total.checked_sub(&p).is_ok("total deposit error.")?;

			let owner = exit_status.utxo().owners()[0];
			let now_balance = <balances::Module<T>>::free_balance(&owner);
			let new_balance = now_balance.checked_add( & value).ok_or("overflow error.")?;

			// fee check reverse fee.
			let fee = Self::fee();
			let new_balance = new_balance.checked_add(&fee).ok_or("not enough fee.") ?;
			let new_total_deposit = new_total_deposit.checked_sub(&fee).ok_or("overflow total deposit.") ?;

			// update
			<TotalDeposit<T>>::put(new_total);
			<balances::Module<T>>::insert(&owner, new_balance);
			<ExitStatusStorage<T>>::mutate(&exit_id, |e| e.as_mut().set_state(ExitState::Finalized));
			Self::deposit_event(RawEvent::ExitFinalize(exit_id));
			Ok(())
		}

		/// exit challenge(fraud proofs) parent chain from child chain.
		pub fn exit_challenge(origin, target: T::Hash, blk_num: T::BlockNumber, depth: u32, index: u64, proofs: Vec<T::Hash>, utxo: Vec<u8>) -> Result {
			let challenger = ensure_signed(origin)?;

			// exist check
			let utxo = T::Utxo::decode( &mut &utxo[..]).ok_or("undecodec utxo binary.")?;
			let depth = depth as u8;
			let proof = MerkleProof{ proofs, depth, index};
			T::ExistProofs::is_exist(&blk_num, &utxo, &proof)?;

			// validate
			let exit_status = <ExitStatusStorage<T>>::get(&exit_id).is_ok("invalid exit id.")?;

			// challenge check
			let challenge_status = ChallengeStatus { blk_num, utxo}
			T::FraudProof::verify(&exit_status, &challenge_status)?;

			// Success...

			// challenger fee gets
			let fee = Self::fee();
			let now_balance = <balances::Module<T>>::free_balance(&challenger);
			let new_balance = now_balance.checked_add(&fee).ok_or("overflow error.")?;

			let now_total = <TotalDeposit<T>>::get();
			let new_total = total.checked_sub(&fee).is_ok("total deposit error.")?;

			// update
			<TotalDeposit<T>>::put(new_total);
			<balances::Module<T>>::insert(&owner, new_balance);
			<ExitStatusStorage<T>>::mutate(&exit_id, |e| e.as_mut().set_state(ExitState::Challenged));
			Self::deposit_event(RawEvent::Challenge(exit_id));
			Ok(())
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event < T >
		where    Hash = < T as system::Trait >::Hash,
				AccountId = < T as system::Trait>::AccountId,
				Balance = < T as balances::Trait >::Balance,
	{
		/// Submit Events
		Submit(Hash),
		/// Deposit Events to child operator.
		Deposit(AccountId, Balance),
		// Start Exit Events to child operator
		ExitStart(AccountId, Hash),
		/// Challenge Events
		Challenge(u32),
		/// Exit Finalize Events
		ExitFinalize(Hash),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use sr_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok};
	use sr_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header},
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;

	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}

	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = ();
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
	}
//
//	impl Trait for Test {
//		type Event = ();
//	}

	// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
	fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {});
	}
}
