//! The Substrate runtime. This can be compiled with ``#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use frame_support::{
    ensure,
    construct_runtime, match_type,  log, parameter_types,
    traits::{Contains,Filter, All},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_PER_SECOND},
        DispatchClass,IdentityFee, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
    },
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_transaction_payment::{
    FeeDetails, Multiplier, RuntimeDispatchInfo, TargetedFeeAdjustment,
};
use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
    create_runtime_str, generic,impl_opaque_keys,
    traits::{AccountIdLookup,Extrinsic as ExtrinsicT, BlakeTwo256, Block as BlockT, ConvertInto, Verify},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, Perbill, Perquintill,
};
use sp_std::{marker::PhantomData, prelude::*};

#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use polkadot_parachain::primitives::Sibling;
// XCM support
use xcm::v0::{MultiAsset, Junction::*, MultiLocation, MultiLocation::*, Xcm,NetworkId};
use xcm_builder::{
    AccountId32Aliases, CurrencyAdapter, LocationInverter, ParentIsDefault, RelayChainAsNative,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SovereignSignedViaLocation, EnsureXcmOrigin,  ParentAsSuperuser,
     TakeWeightCredit, FixedWeightBounds, IsConcrete, NativeAsset,
    UsingComponents,
};
use xcm_executor::{Config,traits::ShouldExecute, XcmExecutor};

use crate::sp_api_hidden_includes_IMPL_RUNTIME_APIS::sp_api::Encode;

pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Kylin module support
pub use kylin_oracle;

/// Constant values used within the runtime.
pub const MILLISDN: Balance = 1_000_000_000_000_000;
pub const SDN: Balance = 1_000 * MILLISDN;

#[derive(codec::Encode, codec::Decode)]
pub enum XCMPMessage<XAccountId, XBalance> {
    /// Transfer tokens to the given account from the Parachain account.
    TransferToken(XAccountId, XBalance),
}



// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
/// Wasm binary unwrapped. If built with `BUILD_DUMMY_WASM_BINARY`, the function panics.
pub fn wasm_binary_unwrap() -> &'static [u8] {
    WASM_BINARY.expect(
        "Development wasm binary is not available. This means the client is \
                        built with `BUILD_DUMMY_WASM_BINARY` flag and it is only usable for \
                        production chains. Please rebuild with the flag disabled.",
    )
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// Runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("shidenK"),
    impl_name: create_runtime_str!("shidenK"),
    authoring_version: 1,
    spec_version: 3,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 0.5 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}




parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = VERSION;
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub SS58Prefix: u8 = 5;
}

pub struct BaseFilter;
impl Filter<Call> for BaseFilter {
    fn filter(call: &Call) -> bool {
        match call {
            // These modules are not allowed to be called by transactions:
            Call::Balances(_) => false,
            // Other modules should works:
            _ => true,
        }
    }
}

impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Runtime version.
    type Version = Version;
    /// Converts a module to an index of this module in the runtime.
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = BaseFilter;
    type SystemWeightInfo = ();
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type SS58Prefix = SS58Prefix;
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

parameter_types! {
    pub const BasicDeposit: Balance = 10 * SDN;       // 258 bytes on-chain
    pub const FieldDeposit: Balance = 25 * MILLISDN;  // 66 bytes on-chain
    pub const SubAccountDeposit: Balance = 2 * SDN;   // 53 bytes on-chain
    pub const MaxSubAccounts: u32 = 100;
    pub const MaxAdditionalFields: u32 = 100;
    pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = ();
    type ForceOrigin = frame_system::EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type RegistrarOrigin = frame_system::EnsureRoot<<Self as frame_system::Config>::AccountId>;
    type WeightInfo = ();
}

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type WeightInfo = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type Event = Event;
    type OnValidationData = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = XcmpQueue;
    type DmpMessageHandler = DmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
	pub const RocLocation: MultiLocation = X1(Parent);
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = X1(Parachain(ParachainInfo::parachain_id().into()));
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsDefault<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RococoNetwork, AccountId>,
);
/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// bias the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, Origin>,
    // Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, Origin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<Origin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RococoNetwork, Origin>,
);

match_type! {
    pub type JustTheParent: impl Contains<MultiLocation> = { X1(Parent) };
}

parameter_types! {
    // One XCM operation is 1_000_000 weight - almost certainly a conservative estimate.
    pub UnitWeightCost: Weight = 1_000_000;
}

pub type Barrier = (
    TakeWeightCredit,
    AllowAnyPaidExecutionFrom<All<MultiLocation>>,
    // AllowUnpaidExecutionFrom<IsInVec<AllowUnpaidFrom>>,	// <- Parent gets free execution
);


pub struct AllowAnyPaidExecutionFrom<T>(PhantomData<T>);
impl<T: Contains<MultiLocation>> ShouldExecute for AllowAnyPaidExecutionFrom<T> {
	fn should_execute<Call>(
		origin: &MultiLocation,
		_top_level: bool,
		_message: &Xcm<Call>,
		_shallow_weight: Weight,
		_weight_credit: &mut Weight,
	) -> Result<(), ()> {
		ensure!(T::contains(origin), ());
		Ok(())
	}
}


pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmRouter;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = NativeAsset;	// <- should be enough to allow teleportation of ROC
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
    type Trader = UsingComponents<IdentityFee<Balance>, RocLocation, AccountId, Balances, ()>;
    type ResponseHandler = ();	// Don't handle responses for now.
}

impl cumulus_pallet_xcm::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

parameter_types! {
	pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
    pub const GracePeriod: u32 = 5;
    pub const UnsignedInterval: u64 = 128;
    pub const UnsignedPriority: u64 = 1 << 20;
}




parameter_types! {
    pub const ExistentialDeposit: Balance = 1_000_000;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 1 * SDN;
}

impl pallet_vesting::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = ();
}

parameter_types! {
    pub const TransactionByteFee: Balance = MILLISDN / 100;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}


/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - [0, MAXIMUM_BLOCK_WEIGHT]
///   - [Balance::min, Balance::max]
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        // in Shiden, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 mSDN:
        let p = MILLISDN;
        let q = 10 * Balance::from(ExtrinsicBaseWeight::get());
        smallvec::smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = WeightToFee;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}



impl kylin_oracle::Config for Runtime {
    type Event = Event;
    type AuthorityId = kylin_oracle::crypto::TestAuthId;
    type Call = Call;
    type Origin = Origin;
    type XcmSender = XcmRouter;
    type UnsignedPriority = UnsignedPriority;
    type UnixTime = Timestamp;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        public: <Signature as Verify>::Signer,
        account: AccountId,
        nonce: <Runtime as frame_system::Config>::Index,
    ) -> Option<(Call, <UncheckedExtrinsic as ExtrinsicT>::SignaturePayload)> {
        use sp_runtime::traits::StaticLookup;
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;

        let current_block = System::block_number()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let tip = 0;
        let extra: SignedExtra = (
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckMortality::<Runtime>::from(generic::Era::mortal(period, current_block.into())),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra).map_err(|e| {
            log::warn!("Unable to create signed payload: {:?}", e);
        }).ok()?;
        let signature = raw_payload.using_encoded(|payload| {
            C::sign(payload, public)
        })?;
        let (call, extra, _) = raw_payload.deconstruct();
        let address = <Runtime as frame_system::Config>::Lookup::unlookup(account);
        Some((call, (address, signature, extra)))
    }
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = ();

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<RocLocation>,
    // Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports.
    (),
>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
    type Event = Event;
    type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmExecuteFilter = All<(MultiLocation, Xcm<Call>)>;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = All<(MultiLocation, Vec<MultiAsset>)>;
    type XcmReserveTransferFilter = ();
    type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as sp_runtime::traits::Verify>::Signer;
    type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
    where
        Call: From<C>,
{
    type OverarchingCall = Call;
    type Extrinsic = UncheckedExtrinsic;
}


construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>} = 10,
        
        Utility: pallet_utility::{Pallet, Call, Event} = 11,
        Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 12,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} =13,
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned} = 20,
        ParachainInfo: parachain_info::{Pallet, Storage, Config} = 21,

        TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 30,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 31,
        Vesting: pallet_vesting::{Pallet, Call, Storage, Config<T>, Event<T>} = 32,

        // XCM helpers.		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 50,
		// PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin} = 51,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 52,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 53,

        // Kylin Pallets
        KylinOraclePallet: kylin_oracle::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 54,
        
        Aura: pallet_aura::{Pallet, Config<T>},
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Config},

        XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin} = 98,

        Sudo: pallet_sudo::{Pallet, Call, Storage, Event<T>, Config<T>} = 99,

    }
);

/// Balance of an account.
pub type Balance = u128;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
/// Index of a transaction in the chain.
pub type Index = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// An index to a block.
pub type BlockNumber = u32;
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPallets,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }


	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}


    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
        Block,
        Balance,
    > for Runtime {
        fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(_: Option<Vec<u8>>) -> Vec<u8> {
            Vec::new()
        }

        fn decode_session_keys(_: Vec<u8>) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
            Some(Vec::new())
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }
    
        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info()
        }
    }
}

struct CheckInherents;
impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
    fn check_inherents(
        _: &Block,
        _: &cumulus_pallet_parachain_system::RelayChainStateProof,
    ) -> sp_inherents::CheckInherentsResult {
        sp_inherents::CheckInherentsResult::new()
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
    CheckInherents = CheckInherents,
}
