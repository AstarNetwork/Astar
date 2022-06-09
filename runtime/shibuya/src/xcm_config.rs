use super::{
    AccountId, AssetId, Assets, Balance, Balances, Call, DealWithFees, Event, Origin,
    ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, WeightToFee, XcAssetConfig, XcmpQueue,
    MAXIMUM_BLOCK_WEIGHT,
};
use frame_support::{
    match_types, parameter_types,
    traits::{Everything, Nothing, PalletInfoAccess},
    weights::{constants::WEIGHT_PER_SECOND, Weight},
};
use sp_runtime::traits::Bounded;
use sp_std::{borrow::Borrow, marker::PhantomData};

// Polkadot imports
use pallet_xc_asset_config::{ExecutionPaymentRate, XcAssetLocation};
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, ConvertedConcreteAssetId,
    CurrencyAdapter, EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, IsConcrete,
    LocationInverter, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue, TakeWeightCredit,
    UsingComponents,
};
use xcm_executor::{
    traits::{FilterAssetLocation, JustTry, WeightTrader},
    Config, XcmExecutor,
};

parameter_types! {
    pub const RelayLocation: MultiLocation = MultiLocation::parent();
    pub RelayNetwork: NetworkId = NetworkId::Any;
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
    pub const Local: MultiLocation = Here.into();
    pub AssetsPalletLocation: MultiLocation =
        PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
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

pub struct AssetLocationIdConverter<AssetId, AssetMapper>(
    sp_std::marker::PhantomData<(AssetId, AssetMapper)>,
);
impl<AssetId, AssetMapper> xcm_executor::traits::Convert<MultiLocation, AssetId>
    for AssetLocationIdConverter<AssetId, AssetMapper>
where
    AssetId: Clone + Eq + Bounded,
    AssetMapper: XcAssetLocation<AssetId>,
{
    fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
        if let Some(asset_id) = AssetMapper::get_asset_id(location.borrow().clone()) {
            Ok(asset_id)
        } else {
            Err(())
        }
    }

    fn reverse_ref(id: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
        if let Some(multilocation) = AssetMapper::get_xc_asset_location(id.borrow().clone()) {
            Ok(multilocation)
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
    ConvertedConcreteAssetId<
        AssetId,
        Balance,
        AssetLocationIdConverter<AssetId, XcAssetConfig>,
        JustTry,
    >,
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

/// Accepts any asset for dev purposes.
/// TODO: replace to AssetFrom
pub struct AnyAsset;
impl FilterAssetLocation for AnyAsset {
    fn filter_asset_location(_asset: &MultiAsset, _origin: &MultiLocation) -> bool {
        true
    }
}

/*
/// Asset filter that allows all assets from a certain location.
pub struct AssetsFrom<T>(PhantomData<T>);
impl<T: Get<MultiLocation>> FilterAssetLocation for AssetsFrom<T> {
    fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
        let loc = T::get();
        &loc == origin
            && matches!(asset, MultiAsset { id: AssetId::Concrete(asset_loc), fun: Fungible(_a) }
                if asset_loc.match_and_split(&loc).is_some())
    }
}
*/

pub struct FixedRateOfForeignAsset<T: ExecutionPaymentRate, R: TakeRevenue> {
    /// Total used weight
    weight: Weight,
    /// Total consumed assets
    consumed: u128,
    /// Asset Id (as MultiLocation) and units per second for payment
    asset_location_and_units_per_second: Option<(MultiLocation, u128)>,
    _pd: PhantomData<(T, R)>,
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> WeightTrader for FixedRateOfForeignAsset<T, R> {
    fn new() -> Self {
        Self {
            weight: 0,
            consumed: 0,
            asset_location_and_units_per_second: None,
            _pd: PhantomData,
        }
    }

    fn buy_weight(
        &mut self,
        weight: Weight,
        payment: xcm_executor::Assets,
    ) -> Result<xcm_executor::Assets, XcmError> {
        log::trace!(
            target: "xcm::weight",
            "FixedRateOfForeignAsset::buy_weight weight: {:?}, payment: {:?}",
            weight, payment,
        );

        // Atm in pallet, we only support one asset so this should work
        let payment_asset = payment
            .fungible_assets_iter()
            .next()
            .ok_or(XcmError::TooExpensive)?;

        match payment_asset {
            MultiAsset {
                id: xcm::latest::AssetId::Concrete(asset_location),
                fun: Fungibility::Fungible(_),
            } => {
                if let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) {
                    let amount = units_per_second * (weight as u128) / (WEIGHT_PER_SECOND as u128);
                    if amount == 0 {
                        return Ok(payment);
                    }

                    let unused = payment
                        .checked_sub((asset_location.clone(), amount).into())
                        .map_err(|_| XcmError::TooExpensive)?;

                    self.weight = self.weight.saturating_add(weight);

                    // If there are multiple calls to `BuyExecution` but with different assets, we need to be able to handle that.
                    // Current primitive implementation will just keep total track of consumed asset for the FIRST consumed asset.
                    // Others will just be ignored when refund is concerned.
                    if let Some((old_asset_location, _)) =
                        self.asset_location_and_units_per_second.clone()
                    {
                        if old_asset_location == asset_location {
                            self.consumed = self.consumed.saturating_add(amount);
                        }
                    } else {
                        self.consumed = self.consumed.saturating_add(amount);
                        self.asset_location_and_units_per_second =
                            Some((asset_location, units_per_second));
                    }

                    return Ok(unused);
                } else {
                    return Err(XcmError::TooExpensive);
                }
            }
            _ => Err(XcmError::TooExpensive),
        }
    }

    fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
        log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

        if let Some((asset_location, units_per_second)) =
            self.asset_location_and_units_per_second.clone()
        {
            let weight = weight.min(self.weight);
            let amount = units_per_second * (weight as u128) / (WEIGHT_PER_SECOND as u128);

            self.weight = self.weight.saturating_sub(weight);
            self.consumed = self.consumed.saturating_sub(amount);

            if amount > 0 {
                Some((asset_location, amount).into())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> Drop for FixedRateOfForeignAsset<T, R> {
    fn drop(&mut self) {
        if let Some((asset_location, _)) = self.asset_location_and_units_per_second.clone() {
            if self.consumed > 0 {
                R::take_revenue((asset_location, self.consumed).into());
            }
        }
    }
}

pub struct XcmConfig;
impl Config for XcmConfig {
    type Call = Call;
    type XcmSender = XcmRouter;
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = AnyAsset;
    type IsTeleporter = ();
    type LocationInverter = LocationInverter<Ancestry>;
    type Barrier = XcmBarrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
    type Trader = (
        UsingComponents<WeightToFee, AnchoringSelfReserve, AccountId, Balances, DealWithFees>,
        FixedRateOfForeignAsset<XcAssetConfig, ()>,
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
    use frame_support::assert_ok;
    use sp_runtime::traits::Zero;
    use xcm_executor::traits::Convert;

    // Primitive, perhaps I improve it later
    const PARENT: MultiLocation = MultiLocation::parent();
    const PARACHAIN: MultiLocation = MultiLocation {
        parents: 1,
        interior: Junctions::X1(Parachain(10)),
    };
    const GENERAL_INDEX: MultiLocation = MultiLocation {
        parents: 2,
        interior: Junctions::X1(GeneralIndex(20)),
    };
    const RELAY_ASSET: AssetId = AssetId::max_value();

    /// Helper struct used for testing `AssetLocationIdConverter`
    struct AssetLocationMapper;
    impl XcAssetLocation<AssetId> for AssetLocationMapper {
        fn get_xc_asset_location(asset_id: AssetId) -> Option<MultiLocation> {
            match asset_id {
                RELAY_ASSET => Some(PARENT),
                20 => Some(PARACHAIN),
                30 => Some(GENERAL_INDEX),
                _ => None,
            }
        }

        fn get_asset_id(asset_location: MultiLocation) -> Option<AssetId> {
            match asset_location {
                a if a == PARENT => Some(RELAY_ASSET),
                a if a == PARACHAIN => Some(20),
                a if a == GENERAL_INDEX => Some(30),
                _ => None,
            }
        }
    }

    /// Helper struct used for testing `FixedRateOfForeignAsset`
    struct ExecutionPayment;
    impl ExecutionPaymentRate for ExecutionPayment {
        fn get_units_per_second(asset_location: MultiLocation) -> Option<u128> {
            match asset_location {
                a if a == PARENT => Some(1_000_000),
                a if a == PARACHAIN => Some(2_000_000),
                a if a == GENERAL_INDEX => Some(3_000_000),
                _ => None,
            }
        }
    }

    /// Execution fee for the specified weight, using provided `units_per_second`
    fn execution_fee(weight: Weight, units_per_second: u128) -> u128 {
        units_per_second * (weight as u128) / (WEIGHT_PER_SECOND as u128)
    }

    #[test]
    fn asset_location_to_id() {
        // Test cases where the MultiLocation is valid
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(PARENT),
            Ok(u128::max_value())
        );
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(PARACHAIN),
            Ok(20)
        );
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(GENERAL_INDEX),
            Ok(30)
        );

        // Test case where MultiLocation isn't supported
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::convert_ref(
                MultiLocation::here()
            ),
            Err(())
        );
    }

    #[test]
    fn asset_id_to_location() {
        // Test cases where the AssetId is valid
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(u128::max_value()),
            Ok(PARENT)
        );
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(20),
            Ok(PARACHAIN)
        );
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(30),
            Ok(GENERAL_INDEX)
        );

        // Test case where the AssetId isn't supported
        assert_eq!(
            AssetLocationIdConverter::<AssetId, AssetLocationMapper>::reverse_ref(0),
            Err(())
        );
    }

    #[test]
    fn fixed_rate_of_foreign_asset_buy_is_ok() {
        let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

        // The amount we have designated for payment (doesn't mean it will be used though)
        let total_payment = 10_000;
        let payment_multi_asset = MultiAsset {
            id: xcm::latest::AssetId::Concrete(PARENT),
            fun: Fungibility::Fungible(total_payment),
        };
        let weight: Weight = 1_000_000_000;

        // Calculate the expected execution fee for the execution weight
        let expected_execution_fee = execution_fee(
            weight,
            ExecutionPayment::get_units_per_second(PARENT).unwrap(),
        );
        assert!(expected_execution_fee > 0); // sanity check

        // 1. Buy weight and expect it to be successful
        let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into());
        if let Ok(assets) = result {
            // We expect only one unused payment asset and specific amount
            assert_eq!(assets.len(), 1);
            assert_ok!(assets.ensure_contains(
                &MultiAsset::from((PARENT, total_payment - expected_execution_fee)).into()
            ));

            assert_eq!(fixed_rate_trader.consumed, expected_execution_fee);
            assert_eq!(fixed_rate_trader.weight, weight);
            assert_eq!(
                fixed_rate_trader.asset_location_and_units_per_second,
                Some((
                    PARENT,
                    ExecutionPayment::get_units_per_second(PARENT).unwrap()
                ))
            );
        } else {
            panic!("Should have been `Ok` wrapped Assets!");
        }

        // 2. Buy more weight, using the same trader and asset type. Verify it works as expected.
        let (old_weight, old_consumed) = (fixed_rate_trader.weight, fixed_rate_trader.consumed);

        let weight: Weight = 3_500_000_000;
        let expected_execution_fee = execution_fee(
            weight,
            ExecutionPayment::get_units_per_second(PARENT).unwrap(),
        );
        assert!(expected_execution_fee > 0); // sanity check

        let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into());
        if let Ok(assets) = result {
            // We expect only one unused payment asset and specific amount
            assert_eq!(assets.len(), 1);
            assert_ok!(assets.ensure_contains(
                &MultiAsset::from((PARENT, total_payment - expected_execution_fee)).into()
            ));

            assert_eq!(
                fixed_rate_trader.consumed,
                expected_execution_fee + old_consumed
            );
            assert_eq!(fixed_rate_trader.weight, weight + old_weight);
            assert_eq!(
                fixed_rate_trader.asset_location_and_units_per_second,
                Some((
                    PARENT,
                    ExecutionPayment::get_units_per_second(PARENT).unwrap()
                ))
            );
        } else {
            panic!("Should have been `Ok` wrapped Assets!");
        }

        // 3. Buy even more weight, but use a different type of asset now while reusing the old trader instance.
        let (old_weight, old_consumed) = (fixed_rate_trader.weight, fixed_rate_trader.consumed);

        // Note that the concrete asset type differs now from previous buys
        let total_payment = 20_000;
        let payment_multi_asset = MultiAsset {
            id: xcm::latest::AssetId::Concrete(PARACHAIN),
            fun: Fungibility::Fungible(total_payment),
        };

        let weight: Weight = 1_750_000_000;
        let expected_execution_fee = execution_fee(
            weight,
            ExecutionPayment::get_units_per_second(PARACHAIN).unwrap(),
        );
        assert!(expected_execution_fee > 0); // sanity check

        let result = fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into());
        if let Ok(assets) = result {
            // We expect only one unused payment asset and specific amount
            assert_eq!(assets.len(), 1);
            assert_ok!(assets.ensure_contains(
                &MultiAsset::from((PARACHAIN, total_payment - expected_execution_fee)).into()
            ));

            assert_eq!(fixed_rate_trader.weight, weight + old_weight);
            // We don't expect this to change since trader already contains data about previous asset type.
            // Current rule is not to update in this case.
            assert_eq!(fixed_rate_trader.consumed, old_consumed);
            assert_eq!(
                fixed_rate_trader.asset_location_and_units_per_second,
                Some((
                    PARENT,
                    ExecutionPayment::get_units_per_second(PARENT).unwrap()
                ))
            );
        } else {
            panic!("Should have been `Ok` wrapped Assets!");
        }
    }

    #[test]
    fn fixed_rate_of_foreign_asset_buy_execution_fails() {
        let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

        // The amount we have designated for payment (doesn't mean it will be used though)
        let total_payment = 1000;
        let payment_multi_asset = MultiAsset {
            id: xcm::latest::AssetId::Concrete(PARENT),
            fun: Fungibility::Fungible(total_payment),
        };
        let weight: Weight = 3_000_000_000;

        // Calculate the expected execution fee for the execution weight
        let expected_execution_fee = execution_fee(
            weight,
            ExecutionPayment::get_units_per_second(PARENT).unwrap(),
        );
        // sanity check, should be more for UT to make sense
        assert!(expected_execution_fee > total_payment);

        // Expect failure because we lack the required funds
        assert_eq!(
            fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into()),
            Err(XcmError::TooExpensive)
        );

        // Try to pay with unsupported funds, expect failure
        let payment_multi_asset = MultiAsset {
            id: xcm::latest::AssetId::Concrete(MultiLocation::here()),
            fun: Fungibility::Fungible(total_payment),
        };
        assert_eq!(
            fixed_rate_trader.buy_weight(0, payment_multi_asset.clone().into()),
            Err(XcmError::TooExpensive)
        );
    }

    #[test]
    fn fixed_rate_of_foreign_asset_refund_is_ok() {
        let mut fixed_rate_trader = FixedRateOfForeignAsset::<ExecutionPayment, ()>::new();

        // The amount we have designated for payment (doesn't mean it will be used though)
        let total_payment = 10_000;
        let payment_multi_asset = MultiAsset {
            id: xcm::latest::AssetId::Concrete(PARENT),
            fun: Fungibility::Fungible(total_payment),
        };
        let weight: Weight = 1_000_000_000;

        // Calculate the expected execution fee for the execution weight and buy it
        let expected_execution_fee = execution_fee(
            weight,
            ExecutionPayment::get_units_per_second(PARENT).unwrap(),
        );
        assert!(expected_execution_fee > 0); // sanity check
        assert_ok!(fixed_rate_trader.buy_weight(weight, payment_multi_asset.clone().into()));

        // Refund quarter and expect it to pass
        let weight_to_refund = weight / 4;
        let assets_to_refund = expected_execution_fee / 4;
        let (old_weight, old_consumed) = (fixed_rate_trader.weight, fixed_rate_trader.consumed);

        let result = fixed_rate_trader.refund_weight(weight_to_refund);
        if let Some(asset_location) = result {
            assert_eq!(asset_location, (PARENT, assets_to_refund).into());

            assert_eq!(fixed_rate_trader.weight, old_weight - weight_to_refund);
            assert_eq!(fixed_rate_trader.consumed, old_consumed - assets_to_refund);
        }

        // Refund more than remains and expect it to pass (saturated)
        let assets_to_refund = fixed_rate_trader.consumed;

        let result = fixed_rate_trader.refund_weight(weight + 10000);
        if let Some(asset_location) = result {
            assert_eq!(asset_location, (PARENT, assets_to_refund).into());

            assert!(fixed_rate_trader.weight.is_zero());
            assert!(fixed_rate_trader.consumed.is_zero());
        }
    }
}
