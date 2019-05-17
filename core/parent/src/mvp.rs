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

/// plasm
use merkle::{ProofTrait, MerkleProof};
use utxo::mvp::{Transaction};


/// Utxo is H: Hash, V: ChildValue, K: AccountId, B: BlockNumber;
#[derive(Clone, Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Utxo<V, K, H, B>(Transaction<V, K, H, B>, u32);

impl<V, K, H, B> UtxoTrait<H, V, K> for Utxo<V, K, H, B>
	where V: Parameter,
		  K: Parameter,
		  H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  B: Parameter {
	fn hash<Hashing: Hash<Output=H>>(&self) -> H {
		Hashing::hash_of(&(Hashing::hash_of(&self.0), self.1))
	}
	fn inputs<Hashing: Hash<Output=H>>(&self) -> Vec<H> {
		self.0.inputs
			.iter()
			.map(|inp| Hashing::hash_of(&(inp.tx_hash.clone(), inp.out_index)))
			.collect::<Vec<_>>()
	}
	fn value(&self) -> &V {
		&self.0.outputs[self.1 as usize].value
	}
	fn owners(&self) -> &Vec<K> {
		&self.0.outputs[self.1 as usize].keys
	}
	fn quorum(&self) -> u32 {
		self.0.outputs[self.1 as usize].quorum
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

#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct ChallengeStatus<H, V, K, B, U> {
	pub blk_num: B,
	pub utxo: U,
	_phantom: PhantomData<(H, V, K)>,
}

/// Implment ChallengeStatus
impl<H, V, K, B, U> ChallengeStatusTrait<B, U> for ChallengeStatus<H, V, K, B, U>
	where H: Codec + Member + MaybeSerializeDebug + rstd::hash::Hash + AsRef<[u8]> + AsMut<[u8]> + Copy + Default,
		  B: Parameter + Member + SimpleArithmetic + Default + Bounded + Copy
		  + rstd::hash::Hash,
		  U: Parameter + Default + UtxoTrait<H, V, K> {
	fn blk_num(&self) -> &B { &self.blk_num }
	fn utxo(&self) -> &U { &self.utxo }
}


/// E: ExitStatus, C: ChallengStatus
pub struct FraudProof<T>(PhantomData<T>);

impl<T: Trait> FraudProofTrait<T> for FraudProof<T> {
	fn verify<E, C>(target: &E, challenge: &C) -> Result
		where E: ExitStatusTrait<T::BlockNumber, T::Utxo, T::Moment, ExitState>,
			  C: ChallengeStatusTrait<T::BlockNumber, T::Utxo> {
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
	where T: Trait
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
}

/// This module's storage items.
decl_storage! {
	trait Store for Module < T: Trait > as Parent {
		TotalDeposit get(total_deposit) config(): <T as balances::Trait>::Balance;
		ChildChain get(child_chain): map T::BlockNumber => Option<T::Hash>;
		CurrentBlock get(current_block): T::BlockNumber = T::BlockNumber::zero();
		Operator get(operator) config() : Vec <T::AccountId>;
		ExitStatusStorage get(exit_status_storage): map T::Hash => Option<T::ExitStatus>;
		UnfinalizedExits get(unfinalized_exits): Vec<T::Hash> = Vec::<T::Hash>::new();
		Fee get(fee) config(): <T as balances::Trait>::Balance;
		ExitWaitingPeriod get(exit_waiting_period) config(): <T as timestamp::Trait>::Moment;
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
			if ! Self::operator().contains(&origin) { return Err("permission error submmit can be only operator."); }
			let current = Self::current_block();
			let next = current.checked_add(&T::BlockNumber::sa(1)).ok_or("block number is overflow.")?;

			// update
			<ChildChain<T>>::insert(&next, root);
			<CurrentBlock<T>>::put(next);
			Self::deposit_event(RawEvent::Submit(root, next));
			Ok(())
		}

		/// deposit balance parent chain to childchain.
		pub fn deposit(origin, value: <T as balances::Trait >::Balance) -> Result {
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
		pub fn exit_start(origin, blk_num: T::BlockNumber, depth: u32, index: u64, proofs: Vec<T::Hash>, utxo: T::Utxo) -> Result {
			let exitor = ensure_signed(origin)?;

			// validate
			// fee check
			let fee = Self::fee();
			let now_balance = <balances::Module<T>>::free_balance(&exitor);
			let new_balance = now_balance.checked_sub(&fee).ok_or("not enough fee.") ?;

			let now_total_deposit = Self::total_deposit();

			let new_total_deposit = now_total_deposit.checked_add(&fee).ok_or("overflow total deposit.") ?;

			// exist check
			let proof = MerkleProof{ proofs, depth, index};
			T::ExistProofs::is_exist(&blk_num, &utxo, &proof)?;

			// owner check
			T::ExitorHasChcker::check(&exitor, &utxo)?;

			let exit_id = utxo.hash::<T::Hashing>();
			let now =  <timestamp::Module <T>>::get();
			let exit_status = ExitStatus {
				blk_num: blk_num,
				utxo: utxo,
				started: now.clone(),
				expired: now + Self::exit_waiting_period(),
				state: ExitState::Exiting,
				_phantom: PhantomData::<(T::Hash, T::ChildValue, T::AccountId)>,
			};
			let exit_status = T::ExitStatus::decode(&mut &exit_status.encode()[..]).unwrap(); // TODO better how to.

			// update
			<balances::FreeBalance<T>>::insert(&exitor, new_balance); // exitor decrease fee.
			<TotalDeposit<T>>::put(new_total_deposit); // total increase fee.
			<ExitStatusStorage<T>>::insert( &exit_id, exit_status); //exit status join!
			<UnfinalizedExits<T>>::mutate( |e| { e.push(exit_id.clone()) }); // push to unfinalized exits.
			Self::deposit_event(RawEvent::ExitStart(exitor, exit_id));

			Ok(())
		}

		/// exit finalize parent chain from childchain.
		pub fn exit_finalize(origin, exit_id: T::Hash) -> Result {
			ensure_signed(origin)?;

			// validate
			let exit_status = <ExitStatusStorage<T>>::get(&exit_id).ok_or("invalid exit id.")?;
			// exit status validate
			T::Finalizer::validate(&exit_status)?;

			// exit value validate
			let pvalue = T::Exchanger::exchange(exit_status.utxo().value().clone());
			let now_total = <TotalDeposit<T>>::get();
			let new_total = now_total.checked_sub(&pvalue).ok_or("total deposit error.")?;

			let owner = &exit_status.utxo().owners()[0]; // TODO soo dangerous
			let now_balance = <balances::Module<T>>::free_balance(owner);
			let new_balance = now_balance.checked_add(&pvalue).ok_or("overflow error.")?;

			// fee check reverse fee.
			let fee = Self::fee();
			let new_balance = new_balance.checked_add(&fee).ok_or("not enough fee.") ?;
			let new_total = new_total.checked_sub(&fee).ok_or("overflow total deposit.") ?;

			// update
			<balances::FreeBalance<T>>::insert(owner, new_balance); // exit owner add fee and exit amount.
			<TotalDeposit<T>>::put(new_total); // total deposit decrease fee and exit amount
			<ExitStatusStorage<T>>::mutate(&exit_id, |e| e.as_mut().unwrap().set_state(ExitState::Finalized)); // exit status changed.
			<UnfinalizedExits<T>>::mutate( |e| {
				*e = e.iter()
					.filter(|v| exit_id != **v)
					.map(|v| *v)
					.collect::<Vec<_>>();
			}); // remove to unfinalized exits.
			Self::deposit_event(RawEvent::ExitFinalize(exit_id));
			Ok(())
		}

		/// exit challenge(fraud proofs) parent chain from child chain.
		pub fn exit_challenge(origin, exit_id: T::Hash, blk_num: T::BlockNumber, depth: u32, index: u64, proofs: Vec<T::Hash>, utxo: T::Utxo) -> Result {
			let challenger = ensure_signed(origin)?;

			// exist check
			let proof = MerkleProof{ proofs, depth, index};
			T::ExistProofs::is_exist(&blk_num, &utxo, &proof)?;

			// validate
			let exit_status = <ExitStatusStorage<T>>::get(&exit_id).ok_or("invalid exit id.")?;

			// challenge check
			let challenge_status = ChallengeStatus { blk_num, utxo,
			 	_phantom: PhantomData::<(T::Hash, T::ChildValue, T::AccountId)>,};
			T::FraudProof::verify(&exit_status, &challenge_status)?;

			// Success...

			// challenger fee gets
			let fee = Self::fee();
			let now_balance = <balances::Module<T>>::free_balance(&challenger);
			let new_balance = now_balance.checked_add(&fee).ok_or("overflow error.")?;

			let now_total = <TotalDeposit<T>>::get();
			let new_total = now_total.checked_sub(&fee).ok_or("total deposit error.")?;

			// update
			<balances::FreeBalance<T>>::insert(&challenger, new_balance); // challenger increase fee.
			<TotalDeposit<T>>::put(new_total); // total deposit decrease fee.
			<ExitStatusStorage<T>>::mutate(&exit_id, |e| e.as_mut().unwrap().set_state(ExitState::Challenged)); // exit status changed.
			Self::deposit_event(RawEvent::Challenged(exit_id));
			Ok(())
		}
	}
}

decl_event!(
	/// An event in this module.
	pub enum Event < T >
		where   Hash = < T as system::Trait >::Hash,
				BlockNumber = <T as system::Trait>::BlockNumber,
				AccountId = < T as system::Trait>::AccountId,
				Balance = < T as balances::Trait >::Balance,
	{
		/// Submit Events
		Submit(Hash, BlockNumber),
		/// Deposit Events to child operator.
		Deposit(AccountId, Balance),
		// Start Exit Events to child operator
		ExitStart(AccountId, Hash),
		/// Challenge Events
		Challenged(Hash),
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
	use support::impl_outer_origin;
	use sr_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header},
	};
	use utxo::mvp::{TransactionInput, TransactionOutput, Transaction};
	use merkle::{MerkleTreeTrait, ReadOnlyMerkleTreeTrait};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;

	type AccountId = u64;
	type BlockNumber = u64;

	#[derive(Clone, PartialEq, Eq, Encode, Decode)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub enum TestEvent {
		Some(RawEvent<H256, BlockNumber, AccountId, u64>),
		None,
	}

	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = BlockNumber;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = AccountId;
		type Lookup = IdentityLookup<AccountId>;
		type Header = Header;
		type Event = TestEvent;
		type Log = DigestItem;
	}

	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = TestEvent;
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
	}

	impl timestamp::Trait for Test {
		type Moment = u64;
		type OnTimestampSet = ();
	}

	impl Trait for Test {
		type ChildValue = u64;
		type Utxo = Utxo<Self::ChildValue, u64, Self::Hash, u64>;
		type Proof = MerkleProof<Self::Hash>;

		type ExitStatus = ExitStatus<Self::Hash, Self::ChildValue, AccountId, BlockNumber, Self::Utxo, Self::Moment, ExitState>;
		type ChallengeStatus = ChallengeStatus<Self::Hash, Self::ChildValue, AccountId, BlockNumber, Self::Utxo>;

		type FraudProof = FraudProof<Test>;
		// How to Fraud proof. to utxo from using utxo.
		type ExitorHasChcker = ExitorHasChcker<Test>;
		type ExistProofs = ExistProofs<Test>;
		type Exchanger = Exchanger<Self::Balance, Self::ChildValue>;
		type Finalizer = Finalizer<Test>;

		/// The overarching event type.
		type Event = TestEvent;
	}

	impl From<system::Event> for TestEvent {
		fn from(_e: system::Event) -> TestEvent {
			TestEvent::None
		}
	}

	impl From<balances::Event<Test>> for TestEvent {
		fn from(_e: balances::Event<Test>) -> TestEvent {
			TestEvent::None
		}
	}

	impl From<Event<Test>> for TestEvent {
		fn from(e: Event<Test>) -> TestEvent {
			TestEvent::Some(e)
		}
	}

	fn get_events() -> Vec<TestEvent> {
		<system::Module<Test>>::events()
			.iter()
			.filter(|e|
				match &e.event {
					TestEvent::Some(_) => true,
					_ => false,
				})
			.cloned()
			.map(|e| e.event)
			.collect::<Vec<_>>()
	}

	fn get_submit_hash_from_events() -> (H256, BlockNumber) {
		for e in get_events() {
			if let TestEvent::Some(RawEvent::Submit(hash, blkNum)) = e {
				return (hash, blkNum);
			}
		}
		Default::default()
	}

	fn get_exit_start_hash_from_events() -> (AccountId, H256) {
		for e in get_events() {
			if let TestEvent::Some(RawEvent::ExitStart(account_id, hash)) = e {
				return (account_id, hash);
			}
		}
		Default::default()
	}

	type Parent = Module<Test>;

	fn gen_tx_in(hash: H256, index: u32) -> TransactionInput<H256> {
		TransactionInput {
			tx_hash: hash,
			out_index: index,
		}
	}

	fn gen_tx_out(value: u64, out_key: AccountId) -> TransactionOutput<u64, AccountId> {
		TransactionOutput {
			value: value,
			keys: vec! {out_key, },
			quorum: 1,
		}
	}

	fn genesis_mvp_tx(value: u64, owner: u64) -> TestUtxo {
		Utxo::<u64, u64, H256, u64>(Transaction::<u64, u64, H256, u64> {
			inputs: vec! {},
			outputs: vec! {gen_tx_out(value, owner), },
			lock_time: 0,
		}, 0)
	}

	fn gen_mvp_tx(in_hash: H256, in_index: u32, value: u64, owner: u64) -> TestUtxo {
		Utxo::<u64, u64, H256, u64>(Transaction::<u64, u64, H256, u64> {
			inputs: vec! {gen_tx_in(in_hash, in_index), },
			outputs: vec! {gen_tx_out(value, owner), },
			lock_time: 0,
		}, 0)
	}

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sr_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(
			GenesisConfig::<Test> {
				total_deposit: 0,
				operator: vec! {0},
				fee: 1,
				exit_waiting_period: 1000, // 1000s
			}.build_storage().unwrap().0
		);
		t.extend(balances::GenesisConfig::<Test> {
			balances: vec![(0, 1000), (1, 100), (2, 100)],
			transaction_base_fee: 0,
			transaction_byte_fee: 0,
			transfer_fee: 0,
			creation_fee: 0,
			existential_deposit: 0,
			vesting: vec![],
		}.build_storage().unwrap().0);
		t.into()
	}

	type Tree = merkle::mock::MerkleTree<H256, BlakeTwo256>;
	type TestUtxo = Utxo<u64, u64, H256, u64>;

	fn test_submit(n: u64) {
		// submit
		assert_eq!(n, Parent::current_block());
		assert_eq!(Ok(()), Parent::submit(Origin::signed(0), Tree::new().root()));
		assert_ne!(Ok(()), Parent::submit(Origin::signed(1), H256::default()));

		assert_eq!(n + 1, Parent::current_block());
		assert_eq!(Tree::new().root(), Parent::child_chain(n + 1).unwrap());
	}

	fn exit_status_test(exit_status: &<Test as Trait>::ExitStatus, blk_num: u64, utxo: &TestUtxo, state: ExitState) {
		assert_eq!(&blk_num, exit_status.blk_num());
		assert_eq!(utxo, exit_status.utxo());
		assert_eq!(&(exit_status.started() + 1000), exit_status.expired());
		assert_eq!(&state, exit_status.state());
	}


	#[test]
	fn it_works_for_minimum() {
		with_externalities(&mut new_test_ext(), || {
			assert_eq!(0, Parent::operator()[0]);

			//  mock children...
			let genesis_utxo = genesis_mvp_tx(1000, 0);
			Tree::new().push(genesis_utxo.hash::<BlakeTwo256>());
			Tree::new().commit();

			// submit 0 -> 1
			test_submit(0);


			// check deposit
			assert_eq!(Ok(()), Parent::deposit(Origin::signed(1), 1));
			assert_eq!(1, Parent::total_deposit());
			// 100 - 1(deposit)
			assert_eq!(99, <balances::Module<Test>>::free_balance(1));

			// mock children...
			let utxo_1 = gen_mvp_tx(BlakeTwo256::hash_of(&genesis_utxo.0), 0, 1, 1);
			Tree::new().push(utxo_1.hash::<BlakeTwo256>());
			Tree::new().commit();


			// exit failed
			let proof = Tree::new().proofs(&utxo_1.hash::<BlakeTwo256>()).unwrap();
			assert_eq!(Tree::new().root(), proof.root::<BlakeTwo256>());
			assert_eq!(&utxo_1.hash::<BlakeTwo256>(), proof.leaf());
			//blk_num: T::BlockNumber, depth: u32, index: u64, proofs: Vec<T::Hash>, utxo: Vec<u8>
			assert_ne!(Ok(()), Parent::exit_start(Origin::signed(1), 2, proof.depth() as u32, proof.index(), proof.proofs().to_vec(), utxo_1.clone()));

			// submit 1 -> 2
			test_submit(1);

			// failed another user.
			assert_ne!(Ok(()), Parent::exit_start(Origin::signed(2), 2, proof.depth() as u32, proof.index(), proof.proofs().to_vec(), utxo_1.clone()));

			// check unfinalize exits empty.
			assert_eq!(Vec::<H256>::new(), Parent::unfinalized_exits());

			// success exit started after submit.
			assert_eq!(Ok(()), Parent::exit_start(Origin::signed(1), 2, proof.depth() as u32, proof.index(), proof.proofs().to_vec(), utxo_1.clone()));

			let exit_id = get_exit_start_hash_from_events().1;
			let exit_status = Parent::exit_status_storage(&exit_id).unwrap();
			exit_status_test(&exit_status, 2, &utxo_1, ExitState::Exiting);
			assert_eq!(98, <balances::Module<Test>>::free_balance(1)); // 100 - 1(deposit) - 1(fee)
			assert_eq!(2, Parent::total_deposit()); // 1(deposit) + 1(fee)

			// check unfinalize exits
			assert_eq!(vec!{exit_id.clone()}, Parent::unfinalized_exits());

			// error finalized before expired.
			assert_ne!(Ok(()), Parent::exit_finalize(Origin::signed(1), exit_id));

			// 1s wait.
			<timestamp::Module<Test>>::set_timestamp(1001);

			// success finalized.
			assert_eq!(Ok(()), Parent::exit_finalize(Origin::signed(1), exit_id));
			let exit_status = Parent::exit_status_storage(&exit_id).unwrap();
			exit_status_test(&exit_status, 2, &utxo_1, ExitState::Finalized);
			assert_eq!(100, <balances::Module<Test>>::free_balance(1)); // 100 - 1(exit) - 1(fee) + 1(exit) + 1(return fee)
			assert_eq!(0, Parent::total_deposit()); // +- 0

			// check unfinalize exits empty.
			assert_eq!(Vec::<H256>::new(), Parent::unfinalized_exits());
		});
	}
}
