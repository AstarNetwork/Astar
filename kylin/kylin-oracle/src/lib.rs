#![cfg_attr(not(feature = "std"), no_std)]
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use sp_core::crypto::KeyTypeId;

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
/// ocpf mean off-chain worker price fetch
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ocpf");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
	};
	use sp_runtime::{MultiSignature, MultiSigner};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;
	// implemented for ocw-runtime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{log, dispatch::DispatchResultWithPostInfo, pallet_prelude::*, traits::UnixTime};
	use frame_system::pallet_prelude::*;
	use frame_system::Config as SystemConfig;
	use codec::{Decode, Encode};
	use sp_std::str;
	use sp_std::vec::Vec;
	use sp_std::borrow::ToOwned;
	use frame_support::storage::IterableStorageMap;
	use frame_system::{
		self as system,
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer,SubmitTransaction
		}
	};
	use sp_runtime::{
		traits::Zero,
		offchain::{http, Duration, storage::{MutateStorageError, StorageRetrievalError, StorageValueRef}},
	};
	
	use cumulus_primitives_core::ParaId;
	use cumulus_pallet_xcm::{Origin as CumulusOrigin, ensure_sibling_para};
	use xcm::v0::{Xcm, Error as XcmError,OriginKind,Junction, MultiLocation, SendXcm};

	enum TransactionType {
		Signed,
		UnsignedForAny,
		UnsignedForAll,
		Raw,
		None,
	}

	#[derive(Encode, Decode, Default, PartialEq, Eq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct PriceFeedingData<BlockNumber> {
		para_id: ParaId,
		currencies: Vec<u8>,
		requested_block_number: BlockNumber,
		processed_block_number: Option<BlockNumber>,
		requested_timestamp:u128,
		processed_timestamp:Option<u128>,
		payload: Vec<u8>,
	}

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		type Origin: From<<Self as SystemConfig>::Origin> + Into<Result<CumulusOrigin, <Self as Config>::Origin>>;
		// type Origin: From<<Self as SystemConfig>::Origin>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching dispatch call type.
		type Call: From<Call<Self>> + Encode;

		type XcmSender: SendXcm;

		type UnixTime: UnixTime;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::type_value]
	pub fn InitialDataId<T: Config>() -> u64 { 10000000u64 }
	
	#[pallet::storage]
	// pub type DataId<T: Config> = StorageValue<_, u64>;
	pub type DataId<T: Config> =	StorageValue<_, u64, ValueQuery, InitialDataId<T>>;


	#[pallet::storage]
	#[pallet::getter(fn price_feeding_requests)]
	pub type PriceFeedingRequests<T: Config> = StorageMap<_, Identity, u64, PriceFeedingData< T::BlockNumber>, ValueQuery>;


	#[pallet::storage]
	#[pallet::getter(fn saved_price_feeding_requests)]
	pub type SavedPriceFeedingRequests<T: Config> = StorageMap<_, Identity, u64, PriceFeedingData< T::BlockNumber>, ValueQuery>;


	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [data, who]
		FetchedOffchainData(u64, T::AccountId),

		FetchedOffchainDataViaXCM(ParaId, Vec<u8>),
		RequestedOffchainDataViaXCM(ParaId, Vec<u8>),
		RequestPriceFeed(ParaId, Vec<u8>),
		ProcessedPriceFeedRequest(ParaId, Vec<u8>, Vec<u8>,),

		ResponseSent(ParaId,T::BlockNumber,Vec<u8>),
		ErrorSendingResponse(XcmError,ParaId,T::BlockNumber,Vec<u8>),
		ResponseReceived(ParaId,T::BlockNumber,Vec<u8>),

		ErrorRequestingData(XcmError, ParaId, Vec<u8>),
		ErrorFetchingData(XcmError, ParaId, Vec<u8>),
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(
			_source: TransactionSource,
			call: &Self::Call,
		) -> TransactionValidity {
			if let Call::submit_price_request_unsigned(block_number,_key, _data) = call {
					Self::validate_transaction(block_number)
				} else if let Call::clear_processed_requests_unsigned(block_number,_processed_requests) = call {
					Self::validate_transaction(block_number)
				}
				else {
					InvalidTransaction::Call.into()
				}
		}
	}

	// // Errors inform users that something went wrong.
	// #[pallet::error]
	// pub enum Error<T> {
	// 	/// Error names should be descriptive.
	// 	NoneValue,
	// 	/// Errors should have helpful documentation associated with them.
	// 	StorageOverflow,
	// }
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

		fn offchain_worker(block_number: T::BlockNumber) {
			// Note that having logs compiled to WASM may cause the size of the blob to increase
			// significantly. You can use `RuntimeDebug` custom derive to hide details of the types
			// in WASM. The `sp-api` crate also provides a feature `disable-logging` to disable
			// all logging and thus, remove any logging from the WASM.

			let parent_hash = <system::Pallet<T>>::block_hash(block_number - 1u32.into());
			log::debug!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);

			// It's a good practice to keep `fn offchain_worker()` function minimal, and move most
			// of the code to separate `impl` block.
			// Here we call a helper function to calculate current average price.
			// This function reads storage entries of the current state.


			let should_send = Self::choose_transaction_type(block_number);
			let res = match should_send {
				TransactionType::Signed => Self::fetch_data_and_send_signed(),
				TransactionType::Raw |TransactionType::UnsignedForAll | TransactionType::UnsignedForAny  => Self::fetch_data_and_send_raw_unsigned(block_number),
				_ => Ok(()),
			};
			if let Err(e) = res {
				log::error!("Error: {}", e);
			}

		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn submit_request_data(origin: OriginFor<T>,  block_number: T::BlockNumber, key: u64, data: Vec<u8>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://substrate.dev/docs/en/knowledgebase/runtime/origin
			ensure_signed(origin.clone())?;
			Self::save_data_response_onchain(block_number, key, data);
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn submit_price_request_unsigned(origin: OriginFor<T>,block_number: T::BlockNumber, key: u64,data: Vec<u8>) -> DispatchResult {
			ensure_none(origin.clone())?;
			Self::save_data_response_onchain(block_number, key, data);
			Self::send_response_to_parachain(block_number, key)
		}


		#[pallet::weight(0)]
		pub fn request_price_feed(_origin: OriginFor<T>,  requester_para_id:ParaId, requested_currencies: Vec<u8>) -> DispatchResult
		{
			let index = DataId::<T>::get();
			let current_block_number = <system::Pallet<T>>::block_number();
			let current_timestamp = T::UnixTime::now().as_millis();

			DataId::<T>::put(index + 1u64);
			<PriceFeedingRequests<T>>::insert(index, PriceFeedingData {
				para_id: requester_para_id,
				currencies: requested_currencies.clone(),
				requested_block_number:current_block_number,
				processed_block_number:None,
				requested_timestamp:current_timestamp,
				processed_timestamp: None,
				payload: Vec::new(),
			});
			
			Self::deposit_event(Event::RequestPriceFeed(requester_para_id, requested_currencies.clone()));
			Ok(())
		}


		#[pallet::weight(0)]
		pub fn request_price_feed_via_xcm(origin: OriginFor<T>,  requested_currencies: Vec<u8>) -> DispatchResult
		{
			let requester_para_id = ensure_sibling_para(<T as Config>::Origin::from(origin))?;
			let current_block_number = <system::Pallet<T>>::block_number();
			let current_timestamp = T::UnixTime::now().as_millis();

			let index = DataId::<T>::get();
			DataId::<T>::put(index + 1u64);

			<PriceFeedingRequests<T>>::insert(index, PriceFeedingData {
				para_id: requester_para_id,
				currencies: requested_currencies.clone(),
				requested_block_number:current_block_number,
				processed_block_number:None,
				requested_timestamp:current_timestamp,
				processed_timestamp: None,
				payload: Vec::new(),
			});
			
			Self::deposit_event(Event::RequestPriceFeed(requester_para_id, requested_currencies.clone()));
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn receive_response_from_parachain(origin: OriginFor<T>, response:Vec<u8>) -> DispatchResult {
			let para_id = ensure_sibling_para(<T as Config>::Origin::from(origin))?;
			let block_number = <system::Pallet<T>>::block_number();
			log::info!("Response received from Parachain {:?}. Received....{}", para_id,str::from_utf8(&response).unwrap());
			Self::deposit_event(Event::ResponseReceived(para_id,block_number,response.clone()));
			Ok(())
		}

		#[pallet::weight(0 + T::DbWeight::get().writes(1))]
		pub fn clear_processed_requests_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			processed_requests: Vec<u64>
		) -> DispatchResultWithPostInfo {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;

			for key in processed_requests.iter(){
				let saved_request = Self::saved_price_feeding_requests(key);
				Self::deposit_event(Event::ProcessedPriceFeedRequest(saved_request.para_id, saved_request.currencies.clone(), saved_request.payload.clone()));
				let current_block = <system::Pallet<T>>::block_number();
				<PriceFeedingRequests<T>>::remove(&key);
				<NextUnsignedAt<T>>::put(current_block);
			}
			Ok(().into())
		}

	}

	impl<T: Config> Pallet<T> {
		fn choose_transaction_type(block_number: T::BlockNumber) -> TransactionType {
			/// A friendlier name for the error that is going to be returned in case we are in the grace
			/// period.
			const RECENTLY_SENT: () = ();
	
			// Start off by creating a reference to Local Storage value.
			// Since the local storage is common for all offchain workers, it's a good practice
			// to prepend your entry with the module name.
			let val = StorageValueRef::persistent(b"kylin_oracle::last_send");
			// The Local Storage is persisted and shared between runs of the offchain workers,
			// and offchain workers may run concurrently. We can use the `mutate` function, to
			// write a storage entry in an atomic fashion. Under the hood it uses `compare_and_set`
			// low-level method of local storage API, which means that only one worker
			// will be able to "acquire a lock" and send a transaction if multiple workers
			// happen to be executed concurrently.
			let res = val.mutate(|last_send: Result<Option<T::BlockNumber>, StorageRetrievalError>| {
				match last_send {
					// If we already have a value in storage and the block number is recent enough
					// we avoid sending another transaction at this time.
					Ok(Some(block)) if block_number < block => {
						Err(RECENTLY_SENT)
					},
					// In every other case we attempt to acquire the lock and send a transaction.
					_ => Ok(block_number)
				}
			});
	
			// The result of `mutate` call will give us a nested `Result` type.
			// The first one matches the return of the closure passed to `mutate`, i.e.
			// if we return `Err` from the closure, we get an `Err` here.
			// In case we return `Ok`, here we will have another (inner) `Result` that indicates
			// if the value has been set to the storage correctly - i.e. if it wasn't
			// written to in the meantime.
			match res {
				// The value has been set correctly, which means we can safely send a transaction now.
				Ok(block_number) => {
					// Depending if the block is even or odd we will send a `Signed` or `Unsigned`
					// transaction.
					// Note that this logic doesn't really guarantee that the transactions will be sent
					// in an alternating fashion (i.e. fairly distributed). Depending on the execution
					// order and lock acquisition, we may end up for instance sending two `Signed`
					// transactions in a row. If a strict order is desired, it's better to use
					// the storage entry for that. (for instance store both block number and a flag
					// indicating the type of next transaction to send).
					let transaction_type = block_number % 3u32.into();
					if transaction_type == Zero::zero() { TransactionType::Signed }
					else if transaction_type == T::BlockNumber::from(1u32) { TransactionType::UnsignedForAny }
					else if transaction_type == T::BlockNumber::from(2u32) { TransactionType::UnsignedForAll }
					else { TransactionType::Raw }
				},
				// We are in the grace period, we should not send a transaction this time.
				Err(MutateStorageError::ValueFunctionFailed(RECENTLY_SENT)) => TransactionType::None,
				// We wanted to send a transaction, but failed to write the block number (acquire a
				// lock). This indicates that another offchain worker that was running concurrently
				// most likely executed the same logic and succeeded at writing to storage.
				// Thus we don't really want to send the transaction, knowing that the other run
				// already did.
				Err(MutateStorageError::ConcurrentModification(_)) => TransactionType::None,
			}
		}

		fn save_data_response_onchain(block_number:T::BlockNumber, key: u64,response: Vec<u8>) -> ()  {
			
			let price_feeding_data = Self::price_feeding_requests(key);
			let current_timestamp = T::UnixTime::now().as_millis();

			<SavedPriceFeedingRequests<T>>::insert(key, PriceFeedingData {
				para_id: price_feeding_data.para_id,
				currencies: price_feeding_data.currencies.clone(),
				requested_block_number:price_feeding_data.requested_block_number,
				processed_block_number:Some(block_number),
				requested_timestamp:price_feeding_data.requested_timestamp,
				processed_timestamp: Some(current_timestamp),
				payload: response

			});
		}

		fn send_response_to_parachain(block_number: T::BlockNumber, key:u64) -> DispatchResult {
			let saved_request = Self::saved_price_feeding_requests(key);
			match T::XcmSender::send_xcm(
				MultiLocation::X2(Junction::Parent, Junction::Parachain(saved_request.para_id.into())),
				Xcm::Transact {
					origin_type: OriginKind::Native,
					require_weight_at_most: 1_000,
					call: <T as Config>::Call::from(Call::<T>::receive_response_from_parachain(saved_request.payload.clone())).encode().into(),
				},
			) {
				Ok(()) => Self::deposit_event(Event::ResponseSent(saved_request.para_id, block_number, saved_request.payload.clone())),
				Err(e) => Self::deposit_event(Event::ErrorSendingResponse(e, saved_request.para_id, block_number, saved_request.payload.clone())),
			}
			Ok(())

		}



		/// A helper function to fetch the price and send signed transaction.
		fn fetch_data_and_send_signed() -> Result<(), &'static str> {
			
			let signer = Signer::<T, T::AuthorityId>::all_accounts();
			if !signer.can_sign() {
				return Err(
					"No local accounts available. Consider adding one via `author_insertKey` RPC.",
				)?;
			}
			let block_number = <system::Pallet<T>>::block_number();
			let mut processed_requests: Vec<u64>  = Vec::new();
			for (key, val) in <PriceFeedingRequests<T> as IterableStorageMap<_, _>>::iter() {
				let currencies = str::from_utf8(&val.currencies).unwrap();
				let split_currencies:Vec<&str> = currencies.split("_").collect();
				let api_url = str::from_utf8(b"https://min-api.cryptocompare.com/data/price?fsym=").unwrap();
				let url = api_url.clone().to_owned() + split_currencies[0].clone() + "&tsyms=" + &split_currencies[1].clone();
				let response = Self::fetch_http_get_result(&url.clone()).unwrap_or("Failed fetch data".as_bytes().to_vec());
				processed_requests.push(key);
				let results = signer.send_signed_transaction(|_account| Call::submit_request_data(block_number, key, response.clone()));
				for (acc, res) in &results {
					match res {
						Ok(()) => log::info!("[{:?}] Submitted data {}", acc.id, key),
						Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
					}
				}
			}
			let results = signer.send_signed_transaction(|_account| Call::clear_processed_requests_unsigned(block_number, processed_requests.clone()));
			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!("[{:?}] Clearing out processed requests.", acc.id),
					Err(e) => log::error!("[{:?}] Failed to clear out processed requests: {:?}", acc.id, e),
				}
			}

			Ok(())
		}

		fn fetch_data_and_send_raw_unsigned(block_number: T::BlockNumber) -> Result<(), &'static str> {
			let next_unsigned_at = <NextUnsignedAt<T>>::get();
			if next_unsigned_at > block_number {
				return Err("Too early to send unsigned transaction")
			}
			
			let mut processed_requests: Vec<u64>  = Vec::new();
			

			// for (key, val) in <PriceFeedingRequests<T> as <PriceFeedingRequests<T> as IterableStorageMapExtended<_, _>>::iter() {
			for (key, val) in <PriceFeedingRequests<T> as IterableStorageMap<_, _>>::iter() {
				let currencies = str::from_utf8(&val.currencies).unwrap();
				let split_currencies:Vec<&str> = currencies.split("_").collect();
				let api_url = str::from_utf8(b"https://min-api.cryptocompare.com/data/price?fsym=").unwrap();
				let url = api_url.clone().to_owned() + split_currencies[0].clone() + "&tsyms=" + &split_currencies[1].clone();
				let response = Self::fetch_http_get_result(&url.clone()).unwrap_or("Failed fetch data".as_bytes().to_vec());
				processed_requests.push(key);
				let result = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(Call::submit_price_request_unsigned(block_number,key, response).into());
				if let Err(e) = result {
					log::error!("Error submitting unsigned transaction: {:?}", e);
				}
			}
			let result = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(Call::clear_processed_requests_unsigned(block_number, processed_requests).into());
			if let Err(e) = result {
				log::error!("Error clearing queue: {:?}", e);
			}
			Ok(())
		}

		/// Fetch current price and return the result in cents.
		fn fetch_http_get_result(url: &str) -> Result<Vec<u8>, http::Error> {
			// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
			// deadline to 2s to complete the external call.
			// You can also wait idefinitely for the response, however you may still get a timeout
			// coming from the host machine.
			let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
			// Initiate an external HTTP GET request.
			// This is using high-level wrappers from `sp_runtime`, for the low-level calls that
			// you can find in `sp_io`. The API is trying to be similar to `reqwest`, but
			// since we are running in a custom WASM execution environment we can't simply
			// import the library here.
			let request = http::Request::get(url);
			// We set the deadline for sending of the request, note that awaiting response can
			// have a separate deadline. Next we send the request, before that it's also possible
			// to alter request headers or stream body content in case of non-GET requests.
			let pending = request
				.deadline(deadline)
				.send()
				.map_err(|_| http::Error::IoError)?;

			// The request is already being processed by the host, we are free to do anything
			// else in the worker (we can send multiple concurrent requests too).
			// At some point however we probably want to check the response though,
			// so we can block current thread and wait for it to finish.
			// Note that since the request is being driven by the host, we don't have to wait
			// for the request to have it complete, we will just not read the response.
			let response = pending
				.try_wait(deadline)
				.map_err(|_| http::Error::DeadlineReached)??;
			// Let's check the status code before we proceed to reading the response.
			if response.code != 200 {
				log::info!("Unexpected status code: {}", response.code);
				return Err(http::Error::Unknown);
			}

			// Next we want to fully read the response body and collect it to a vector of bytes.
			// Note that the return object allows you to read the body in chunks as well
			// with a way to control the deadline.
			let body = response.body().collect::<Vec<u8>>();

			// Create a str slice from the body.
			let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
				log::info!("No UTF8 body");
				http::Error::Unknown
			})?;
			log::info!("fetch_http_get_result Got {} result: {}", url, body_str);

			Ok(body_str.as_bytes().to_vec())
		}
	

	fn validate_transaction(block_number: &T::BlockNumber) -> TransactionValidity {

		// Now let's check if the transaction has any chance to succeed.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if &next_unsigned_at > block_number {
			return InvalidTransaction::Stale.into();
		}
		// Let's make sure to reject transactions from the future.
		let current_block = <system::Pallet<T>>::block_number();
		if &current_block < block_number {
			return InvalidTransaction::Future.into();
		}
		ValidTransaction::with_tag_prefix("Kylin OCW")
			.priority(T::UnsignedPriority::get())
			.longevity(5)
			.propagate(true)
			.build()
	
	}
}
}

