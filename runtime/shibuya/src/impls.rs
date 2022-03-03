use frame_support::traits::{
    fungibles::{self, Balanced, CreditOf},
    Contains, Get,
};
use pallet_asset_tx_payment::HandleCredit;
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;
use xcm::latest::{AssetId, Fungibility::Fungible, MultiAsset, MultiLocation};
use xcm_executor::traits::FilterAssetLocation;

/// Type alias to conveniently refer to `frame_system`'s `Config::AccountId`.
pub type AccountIdOf<R> = <R as frame_system::Config>::AccountId;

/// A `HandleCredit` implementation that naively transfers the fees to the block author.
/// Will drop and burn the assets in case the transfer fails.
pub struct AssetsToBlockAuthor<R>(PhantomData<R>);
impl<R> HandleCredit<AccountIdOf<R>, pallet_assets::Pallet<R>> for AssetsToBlockAuthor<R>
where
    R: pallet_authorship::Config + pallet_assets::Config,
    AccountIdOf<R>:
        From<polkadot_primitives::v1::AccountId> + Into<polkadot_primitives::v1::AccountId>,
{
    fn handle_credit(credit: CreditOf<AccountIdOf<R>, pallet_assets::Pallet<R>>) {
        let author = pallet_authorship::Pallet::<R>::author();
        // In case of error: Will drop the result triggering the `OnDrop` of the imbalance.
        let _ = pallet_assets::Pallet::<R>::resolve(&author, credit);
    }
}

/// Allow checking in assets that have issuance > 0.
pub struct NonZeroIssuance<AccountId, Assets>(PhantomData<(AccountId, Assets)>);
impl<AccountId, Assets> Contains<<Assets as fungibles::Inspect<AccountId>>::AssetId>
    for NonZeroIssuance<AccountId, Assets>
where
    Assets: fungibles::Inspect<AccountId>,
{
    fn contains(id: &<Assets as fungibles::Inspect<AccountId>>::AssetId) -> bool {
        !Assets::total_issuance(*id).is_zero()
    }
}

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
