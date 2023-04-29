#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use pallet_contracts::Pallet as PalletContracts;
use pallet_xcm::Pallet as PalletXcm;
use sp_core::H160;
use sp_runtime::traits::{AccountIdConversion, Zero};
use sp_std::prelude::*;
use xcm::prelude::*;

pub type MethodSelector = [u8; 4];

pub mod chain_extension;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::Config as SysConfig;
    use pallet_xcm::ensure_response;
    pub use xcm_ce_primitives::{QueryConfig, QueryType};

    // Response info
    #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct ResponseInfo<AccountId> {
        pub query_id: QueryId,
        pub query_type: QueryType<AccountId>,
        pub response: Response,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_xcm::Config + pallet_contracts::Config {
        /// The overarching event type.
        type RuntimeEvent: IsType<<Self as frame_system::Config>::RuntimeEvent> + From<Event<Self>>;

        /// The overarching call type.
        type RuntimeCall: Parameter
            + From<Call<Self>>
            + IsType<<Self as pallet_xcm::Config>::RuntimeCall>;

        /// The overaching origin type
        type RuntimeOrigin: Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>
            + IsType<<Self as frame_system::Config>::RuntimeOrigin>;

        /// Query Handler for creating quries and handling response
        type CallbackHandler: OnCallback<
            AccountId = Self::AccountId,
            BlockNumber = Self::BlockNumber,
        >;

        /// Required origin for sending registering new queries. If successful, it resolves to `MultiLocation`
        /// which exists as an interior location within this chain's XCM context.
        type RegisterQueryOrigin: EnsureOrigin<
            <Self as SysConfig>::RuntimeOrigin,
            Success = MultiLocation,
        >;

        /// Max weight for callback
        #[pallet::constant]
        type MaxCallbackWeight: Get<Weight>;

        /// Relay network id
        #[pallet::constant]
        type Network: Get<Option<NetworkId>>;
    }

    /// Mapping of ongoing queries and thier type
    #[pallet::storage]
    #[pallet::getter(fn callback_query)]
    pub(super) type CallbackQueries<T: Config> =
        StorageMap<_, Blake2_128Concat, QueryId, QueryType<T::AccountId>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // successfully handled callback
        CallbackSuccess(QueryType<T::AccountId>),
        // new query registered
        QueryPrepared {
            query_type: QueryType<T::AccountId>,
            query_id: QueryId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        // CE Errors
        XcmVersionNotSupported,
        CannotWeigh,
        InvalidCommand,
        PreparationMissing,
        ExecutionFailed,
        SendValidateFailed,
        SendFailed,
        /// The version of the Versioned value used is not able to be interpreted.
        BadVersion,
        /// Origin not allow for registering queries
        InvalidOrigin,
        /// Query not found in storage
        UnexpectedQueryResponse,
        /// Does not support the given query type
        NotSupported,
        /// Callback out of gas
        /// TODO: use it
        OutOfGas,
        /// WASM Contract reverted
        WASMContractReverted,
        /// EVM Contract reverted
        EVMContractReverted,
        /// callback failed due to unkown reasons
        /// TODO: split this error into known errors
        CallbackFailed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Dispatch for recieving callback from pallet_xcm's notify
        /// and handle their routing
        /// TODO: Weights,
        ///       (max callback weight) + 1 DB read + 1 event + some extra (from benchmarking)
        #[pallet::call_index(0)]
        #[pallet::weight(T::MaxCallbackWeight::get())]
        pub fn on_callback_recieved(
            origin: OriginFor<T>,
            query_id: QueryId,
            response: Response,
        ) -> DispatchResult {
            // ensure the origin is a response
            let responder = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;
            // fetch the query
            let query_type =
                CallbackQueries::<T>::get(query_id).ok_or(Error::<T>::UnexpectedQueryResponse)?;
            // handle the response routing
            // TODO: in case of error, maybe save the response for manual
            // polling as fallback. This will require taking into weight of storing
            // response in storage in the weights of `prepare_new_query` dispatch
            T::CallbackHandler::on_callback(
                responder,
                ResponseInfo {
                    query_id,
                    query_type: query_type.clone(),
                    response,
                },
            )
            .map_err(|e| e.into())?;

            // remove query from storage
            CallbackQueries::<T>::remove(query_id);

            // deposit success event
            Self::deposit_event(Event::<T>::CallbackSuccess(query_type));
            Ok(())
        }

        /// Register a new query
        /// TODO: Weights,
        ///       (1 DB read + 3 DB write + 1 event + some extra (need benchmarking))
        ///       Weight for this does not take callback weights into account. That should be
        ///       done via XcmWeigher using WeightBounds, where all query instructions's
        ///       `QueryResponseInfo` `max_weight` is taken into account.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(1_000_000, 1_000_000))]
        pub fn prepare_new_query(
            origin: OriginFor<T>,
            config: QueryConfig<T::AccountId, T::BlockNumber>,
            dest: Box<VersionedMultiLocation>,
        ) -> DispatchResult {
            let origin_location = T::RegisterQueryOrigin::ensure_origin(origin)?;
            let interior: Junctions = origin_location
                .try_into()
                .map_err(|_| Error::<T>::InvalidOrigin)?;
            let query_type = config.query_type.clone();
            let dest = MultiLocation::try_from(*dest).map_err(|()| Error::<T>::BadVersion)?;

            // register query
            let query_id = Self::new_query(config, interior, dest)?;
            Self::deposit_event(Event::<T>::QueryPrepared {
                query_type,
                query_id,
            });
            Ok(())
        }
    }
}

/// Handle the incoming xcm notify callback from ResponseHandler (pallet_xcm)
pub trait OnCallback {
    // error type, that can be converted to dispatch error
    type Error: Into<DispatchError>;
    // account id type
    type AccountId;
    // blocknumber type
    type BlockNumber;

    // TODO: Query type itself should be generic like
    //
    // type QueryType: Member + Parameter + MaybeSerializeDeserialize + MaxEncodedLen + Convert<Self, Weight>
    // type CallbackHandler: OnResponse<QueryType = T::QueryType>
    //
    // #[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen)]
    // enum MyQueryType {}
    //
    // impl Convert<Self, Weight> for MyQueryType {}

    /// Check whether query type is supported or not
    fn can_handle(query_type: &QueryType<Self::AccountId>) -> bool;

    /// handle the xcm response
    fn on_callback(
        responder: impl Into<MultiLocation>,
        response_info: ResponseInfo<Self::AccountId>,
    ) -> Result<Weight, Self::Error>;
}

impl<T: Config> OnCallback for Pallet<T> {
    type AccountId = T::AccountId;
    type BlockNumber = T::BlockNumber;
    type Error = Error<T>;

    fn can_handle(query_type: &QueryType<Self::AccountId>) -> bool {
        match query_type {
            QueryType::NoCallback => true,
            QueryType::WASMContractCallback { .. } => true,
            // TODO: add support for evm contracts
            QueryType::EVMContractCallback { .. } => false,
        }
    }

    fn on_callback(
        responder: impl Into<MultiLocation>,
        response_info: ResponseInfo<Self::AccountId>,
    ) -> Result<Weight, Self::Error> {
        let ResponseInfo {
            query_id,
            query_type,
            response,
        } = response_info;

        match query_type {
            QueryType::NoCallback => {
                // TODO: Nothing to do, maybe error?
                Ok(Weight::zero())
            }
            QueryType::WASMContractCallback {
                contract_id,
                selector,
            } => Self::call_wasm_contract_method(
                contract_id,
                selector,
                query_id,
                responder.into(),
                response,
            ),
            QueryType::EVMContractCallback {
                contract_id,
                selector,
            } => Self::call_evm_contract_method(
                contract_id,
                selector,
                query_id,
                responder.into(),
                response,
            ),
        }
    }
}

impl<T: Config> Pallet<T> {
    /// The account ID of the pallet.
    pub fn account_id() -> T::AccountId {
        const ID: PalletId = PalletId(*b"py/xcmnt");
        AccountIdConversion::<T::AccountId>::into_account_truncating(&ID)
    }

    /// Register new query originating from querier to dest
    pub fn new_query(
        QueryConfig {
            query_type,
            timeout,
        }: QueryConfig<T::AccountId, T::BlockNumber>,
        querier: impl Into<Junctions>,
        dest: impl Into<MultiLocation>,
    ) -> Result<QueryId, Error<T>> {
        let querier = querier.into();

        // check if with callback handler
        if !(T::CallbackHandler::can_handle(&query_type)) {
            return Err(Error::NotSupported);
        }

        Ok(match query_type.clone() {
            QueryType::NoCallback => PalletXcm::<T>::new_query(dest, timeout, querier),
            QueryType::WASMContractCallback { .. } | QueryType::EVMContractCallback { .. } => {
                let call: <T as Config>::RuntimeCall = Call::on_callback_recieved {
                    query_id: 0,
                    response: Response::Null,
                }
                .into();
                let id = PalletXcm::<T>::new_notify_query(dest, call, timeout, querier);
                CallbackQueries::<T>::insert(id, query_type);
                id
            }
        })
    }

    fn call_wasm_contract_method(
        contract_id: T::AccountId,
        selector: [u8; 4],
        query_id: QueryId,
        responder: MultiLocation,
        response: Response,
    ) -> Result<Weight, Error<T>> {
        // TODO: Use responder to derieve a origin account id
        let outcome = PalletContracts::<T>::bare_call(
            Self::account_id(),
            contract_id,
            Zero::zero(),
            T::MaxCallbackWeight::get(),
            None,
            [selector.to_vec(), (query_id, responder, response).encode()].concat(),
            // TODO: should not be true
            true,
            pallet_contracts::Determinism::Deterministic,
        );

        let retval = outcome.result.map_err(|_| Error::CallbackFailed)?;
        if retval.did_revert() {
            Err(Error::WASMContractReverted)
        } else {
            Ok(outcome.gas_consumed)
        }
    }

    fn call_evm_contract_method(
        _contract_id: H160,
        _selector: [u8; 4],
        _query_id: QueryId,
        _responder: MultiLocation,
        _response: Response,
    ) -> Result<Weight, Error<T>> {
        Ok(Weight::zero())
    }
}
