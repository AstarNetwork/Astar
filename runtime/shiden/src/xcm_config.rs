use super::{
    AccountId, AssetId, Assets, Balance, Balances, Call, DealWithFees, Event, Origin,
    ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, WeightToFee, XcmpQueue,
    MAXIMUM_BLOCK_WEIGHT,
};
use frame_support::{
    match_types, parameter_types,
    traits::{Everything, Nothing, PalletInfoAccess},
    weights::Weight,
};
use sp_std::borrow::Borrow;

// Polkadot imports
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, ConvertedConcreteAssetId,
    CurrencyAdapter, EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, FungiblesAdapter,
    IsConcrete, LocationInverter, NativeAsset, ParentAsSuperuser, ParentIsPreset,
    RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
    SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
    UsingComponents,
};
use xcm_executor::{traits::JustTry, Config, XcmExecutor};

parameter_types! {
    pub const RelayLocation: MultiLocation = MultiLocation::parent();
    pub RelayNetwork: NetworkId = NetworkId::Kusama;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
    pub const Local: MultiLocation = Here.into();
    pub AnchoringSelfReserve: MultiLocation =
        PalletInstance(<Balances as PalletInfoAccess>::index() as u8).into();
    pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<polkadot_parachain::primitives::Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<AnchoringSelfReserve>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    (),
>;

pub struct AsAssetWithRelay;

/// AssetId allocation:
/// [1; 2^32-1]     Custom user assets (permissionless)
/// [2^32; 2^64-1]  Statemine assets (simple map)
/// [2^64; 2^128-1] Ecosystem assets
/// 2^128-1         Relay chain token (KSM)
impl AsAssetWithRelay {
    /// Local Id of the relay chain asset (KSM)
    pub const RELAY_CHAIN_ASSET_ID: AssetId = AssetId::max_value();

    pub const STATEMINE_PARA_ID: u32 = 1000;
    pub const STATEMINE_ASSET_PALLET_INSTANCE: u8 = 50;

    /// Offset value from which Statemine asset Ids start
    pub const STATEMINE_OFFSET: AssetId = (1 << 32);

    /// Max value of Statemine asset id
    pub const MAX_STATEMINE_ASSET_ID: AssetId = (1 << 64) - 1;

    /// `true` if asset is a Statemine asset, `false` otherwise
    pub fn is_statemine_asset(id: AssetId) -> bool {
        id >= Self::STATEMINE_OFFSET && id <= Self::MAX_STATEMINE_ASSET_ID
    }
}

impl xcm_executor::traits::Convert<MultiLocation, AssetId> for AsAssetWithRelay {
    fn convert_ref(id: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
        match id.borrow() {
            // Native relaychain asset
            MultiLocation {
                parents: 1,
                interior: Junctions::Here,
            } => Ok(Self::RELAY_CHAIN_ASSET_ID),
            // Statemine `pallet_assets` assets
            MultiLocation {
                parents: 1,
                interior:
                    X3(
                        Parachain(Self::STATEMINE_PARA_ID),
                        PalletInstance(Self::STATEMINE_ASSET_PALLET_INSTANCE),
                        GeneralIndex(index),
                    ),
            } => index
                .checked_add(Self::STATEMINE_OFFSET)
                .map_or(Err(()), |x| Ok(x)),
            _ => Err(()),
        }
    }

    fn reverse_ref(what: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
        let local_asset_id = *what.borrow();

        if local_asset_id == Self::RELAY_CHAIN_ASSET_ID {
            Ok(MultiLocation::parent())
        } else if Self::is_statemine_asset(local_asset_id) {
            local_asset_id
                .checked_sub(Self::STATEMINE_OFFSET)
                .map_or(Err(()), |x| {
                    Ok(MultiLocation {
                        parents: 1,
                        interior: X3(
                            Parachain(Self::STATEMINE_PARA_ID),
                            PalletInstance(Self::STATEMINE_ASSET_PALLET_INSTANCE),
                            GeneralIndex(x),
                        ),
                    })
                })
        } else {
            Err(())
        }
    }
}

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    ConvertedConcreteAssetId<AssetId, Balance, AsAssetWithRelay, JustTry>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Assets`.
    Nothing,
    // We don't track any teleports of `Assets`.
    CheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (FungiblesTransactor, CurrencyTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, Origin>,
    // Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, Origin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<Origin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    pallet_xcm::XcmPassthrough<Origin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, Origin>,
);

parameter_types! {
    // One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
    pub UnitWeightCost: Weight = 1_000_000_000;
    pub const MaxInstructions: u32 = 100;
    pub KsmPerSecond: (xcm::v1::AssetId, u128) = (MultiLocation::parent().into(), 1_000_000_000);
}

match_types! {
    pub type ParentOrParentsPlurality: impl Contains<MultiLocation> = {
        MultiLocation { parents: 1, interior: Here } |
        MultiLocation { parents: 1, interior: X1(Plurality { .. }) }
    };
}

pub type XcmBarrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    // Parent and its plurality get free execution
    AllowUnpaidExecutionFrom<ParentOrParentsPlurality>,
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    // Subscriptions for version tracking are OK.
    AllowSubscriptionsFrom<Everything>,
);

pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmRouter;
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = XcmBarrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type Trader = (
        FixedRateOfFungible<KsmPerSecond, ()>,
        UsingComponents<WeightToFee, AnchoringSelfReserve, AccountId, Balances, DealWithFees>,
    );
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
}

parameter_types! {
    pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;

    type Event = Event;
    type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type LocationInverter = LocationInverter<Ancestry>;
    type Origin = Origin;
    type Call = Call;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
    type ControllerOrigin = frame_system::EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

#[cfg(test)]
mod test {

    use super::*;
    use xcm_executor::traits::Convert;

    /// Returns `MultiLocation` representing a Statemine asset
    fn statemine_asset_location(index: u32) -> MultiLocation {
        MultiLocation {
            parents: 1,
            interior: Junctions::X3(
                Parachain(1000),
                PalletInstance(50),
                GeneralIndex(index.into()),
            ),
        }
    }

    #[test]
    fn multilocation_to_asset_id() {
        // Relay chain asset
        let relay_chain_native_asset = MultiLocation::parent();
        assert_eq!(
            AsAssetWithRelay::convert_ref(relay_chain_native_asset),
            Ok(u128::max_value())
        );

        // Statemine assets
        let min_statemine_asset = statemine_asset_location(0);
        assert_eq!(
            AsAssetWithRelay::convert_ref(min_statemine_asset),
            Ok(1_u128 << 32)
        );
        let arbitrary_statemine_asset = statemine_asset_location(7 * 13 * 19);
        assert_eq!(
            AsAssetWithRelay::convert_ref(arbitrary_statemine_asset),
            Ok((1_u128 << 32) + 7 * 13 * 19)
        );
        // at the moment, we cannot take advantage of the entire Statemine asset Id range since u32 is used
        let max_statemine_asset = statemine_asset_location(u32::max_value());
        assert_eq!(
            AsAssetWithRelay::convert_ref(max_statemine_asset),
            Ok((1_u128 << 33) - 1)
        );
    }

    #[test]
    fn asset_id_to_multilocation() {
        // Relay chain asset
        assert_eq!(
            AsAssetWithRelay::reverse_ref(u128::max_value()),
            Ok(MultiLocation::parent())
        );

        // Statemine assets
        let min_statemine_asset_id = 0;
        assert_eq!(
            AsAssetWithRelay::reverse_ref(1 << 32),
            Ok(statemine_asset_location(
                min_statemine_asset_id.try_into().unwrap()
            ))
        );
        let max_statemine_asset_id: AssetId = (u32::max_value()).into();
        assert_eq!(
            AsAssetWithRelay::reverse_ref(max_statemine_asset_id + (1 << 32)),
            Ok(statemine_asset_location(
                max_statemine_asset_id.try_into().unwrap()
            ))
        );
    }
}
