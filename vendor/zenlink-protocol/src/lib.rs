// Copyright 2020-2021 Zenlink
// Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use pallet::*;

use codec::{Decode, Encode};
use frame_support::{
	inherent::Vec,
	pallet_prelude::*,
	sp_runtime::SaturatedConversion,
	traits::{Currency, ExistenceRequirement, ExistenceRequirement::KeepAlive, Get, WithdrawReasons},
	PalletId, RuntimeDebug,
};
use sp_core::U256;
use sp_runtime::traits::{AccountIdConversion, Hash, IntegerSquareRoot, One, StaticLookup, Zero};
use sp_std::{convert::TryInto, marker::PhantomData, prelude::*};

// -------xcm--------
pub use cumulus_primitives_core::ParaId;

use xcm::v0::{
	Error as XcmError, ExecuteXcm, Junction, MultiAsset, MultiLocation, Order, Outcome, Result as XcmResult, Xcm,
};

use xcm_executor::{
	traits::{Convert, FilterAssetLocation, TransactAsset},
	Assets,
};
// -------xcm--------

mod fee;
mod foreign;
mod liquidity;
mod multiassets;
mod primitives;
mod rpc;
mod swap;
mod traits;
mod transactor;
mod transfer;

pub use multiassets::{MultiAssetsHandler, ZenlinkMultiAssets};
pub use primitives::{AssetBalance, AssetId, LIQUIDITY, LOCAL, NATIVE, RESERVED};
pub use rpc::PairInfo;
pub use traits::{ExportZenlink, LocalAssetHandler, OtherAssetHandler};
pub use transactor::{TransactorAdaptor, TrustedParas};

const LOG_TARGET: &str = "zenlink_protocol";
pub fn make_x2_location(para_id: u32) -> MultiLocation {
	MultiLocation::X2(Junction::Parent, Junction::Parachain(para_id))
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::DispatchResult;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The assets interface beyond native currency and other assets.
		type MultiAssetsHandler: MultiAssetsHandler<Self::AccountId>;
		/// This pallet id.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// XCM

		/// The set of parachains which the xcm can reach.
		type TargetChains: Get<Vec<(MultiLocation, u128)>>;
		/// This parachain id.
		type SelfParaId: Get<u32>;
		/// Something to execute an XCM message.
		type XcmExecutor: ExecuteXcm<Self::Call>;
		/// AccountId to be used in XCM as a corresponding AccountId32
		/// and convert from MultiLocation in XCM
		type Conversion: Convert<MultiLocation, Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Foreign foreign storage
	#[pallet::storage]
	#[pallet::getter(fn foreign_ledger)]
	/// The number of units of assets held by any given account.
	pub type ForeignLedger<T: Config> =
		StorageMap<_, Blake2_128Concat, (AssetId, T::AccountId), AssetBalance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn foreign_meta)]
	/// TWOX-NOTE: `AssetId` is trusted, so this is safe.
	pub type ForeignMeta<T: Config> = StorageMap<_, Twox64Concat, AssetId, AssetBalance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn foreign_list)]
	pub type ForeignList<T: Config> = StorageValue<_, Vec<AssetId>, ValueQuery>;

	/// Swap liquidity storage
	#[pallet::storage]
	#[pallet::getter(fn lp_metadata)]
	/// TWOX-NOTE: `AssetId` is trusted, so this is safe.
	/// (AssetId, AssetId) -> (PairAccountId, TotalSupply)
	pub type LiquidityMeta<T: Config> = StorageMap<_, Twox64Concat, (AssetId, AssetId), (T::AccountId, AssetBalance)>;

	#[pallet::storage]
	#[pallet::getter(fn lp_ledger)]
	/// ((AssetId, AssetId), AccountId) -> AssetBalance
	pub type LiquidityLedger<T: Config> =
		StorageMap<_, Blake2_128Concat, ((AssetId, AssetId), T::AccountId), AssetBalance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn lp_pairs)]
	pub type LiquidityPairs<T: Config> = StorageValue<_, Vec<(AssetId, AssetId)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn k_last)]
	/// Refer: https://github.com/Uniswap/uniswap-v2-core/blob/master/contracts/UniswapV2Pair.sol#L88
	/// Last unliquidated protocol fee;
	pub type KLast<T: Config> = StorageMap<_, Twox64Concat, (AssetId, AssetId), AssetBalance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn fee_meta)]
	/// (fee_admin, Option<fee_receiver>, fee_point)
	pub(super) type FeeMeta<T: Config> = StorageValue<_, (T::AccountId, Option<T::AccountId>, u8), ValueQuery>;

	#[pallet::genesis_config]
	/// Refer: https://github.com/Uniswap/uniswap-v2-core/blob/master/contracts/UniswapV2Pair.sol#L88
	pub struct GenesisConfig<T: Config> {
		/// The admin of the protocol fee.
		pub fee_admin: T::AccountId,
		/// The receiver of the protocol fee.
		pub fee_receiver: Option<T::AccountId>,
		/// The fee point which integer between [0,30]
		/// 0 means no protocol fee.
		/// 30 means 0.3% * 100% = 0.0030.
		/// default is 5 and means 0.3% * 1 / 6 = 0.0005.
		pub fee_point: u8,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				fee_admin: Default::default(),
				fee_receiver: None,
				fee_point: 5,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<FeeMeta<T>>::put((&self.fee_admin, &self.fee_receiver, &self.fee_point));
		}
	}

	#[cfg(feature = "std")]
	impl<T: Config> GenesisConfig<T> {
		/// Direct implementation of `GenesisBuild::build_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn build_storage(&self) -> Result<sp_runtime::Storage, String> {
			<Self as GenesisBuild<T>>::build_storage(self)
		}

		/// Direct implementation of `GenesisBuild::assimilate_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn assimilate_storage(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
			<Self as GenesisBuild<T>>::assimilate_storage(self, storage)
		}
	}

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Foreign Asset

		/// Some assets were transferred. \[asset_id, owner, target, amount\]
		Transferred(AssetId, T::AccountId, T::AccountId, AssetBalance),
		/// Some assets were burned. \[asset_id, owner, amount\]
		Burned(AssetId, T::AccountId, AssetBalance),
		/// Some assets were minted. \[asset_id, owner, amount\]
		Minted(AssetId, T::AccountId, AssetBalance),

		/// Swap

		/// Create a trading pair. \[creator, asset_0, asset_1\]
		PairCreated(T::AccountId, AssetId, AssetId),
		/// Add liquidity. \[owner, asset_0, asset_1, add_balance_0, add_balance_1,
		/// mint_balance_lp\]
		LiquidityAdded(T::AccountId, AssetId, AssetId, AssetBalance, AssetBalance, AssetBalance),
		/// Remove liquidity. \[owner, recipient, asset_0, asset_1, rm_balance_0, rm_balance_1,
		/// burn_balance_lp\]
		LiquidityRemoved(
			T::AccountId,
			T::AccountId,
			AssetId,
			AssetId,
			AssetBalance,
			AssetBalance,
			AssetBalance,
		),
		/// Transact in trading \[owner, recipient, swap_path, balance_in, balance_out\]
		AssetSwap(T::AccountId, T::AccountId, Vec<AssetId>, AssetBalance, AssetBalance),

		/// Transfer by xcm

		/// Transferred to parachain. \[asset_id, src, para_id, dest, amount, used_weight\]
		TransferredToParachain(AssetId, T::AccountId, ParaId, T::AccountId, AssetBalance, Weight),
	}
	#[pallet::error]
	pub enum Error<T> {
		/// Require the admin who can reset the admin and receiver of the protocol fee.
		RequireProtocolAdmin,
		/// Invalid fee_point
		InvalidFeePoint,
		/// Unsupported AssetId by this ZenlinkProtocol Version.
		UnsupportedAssetType,
		/// Account balance must be greater than or equal to the transfer amount.
		InsufficientAssetBalance,
		/// Account native currency balance must be greater than ExistentialDeposit.
		NativeBalanceTooLow,
		/// Trading pair can't be created.
		DeniedCreatePair,
		/// Trading pair already exists.
		PairAlreadyExists,
		/// Trading pair does not exist.
		PairNotExists,
		/// Asset does not exist.
		AssetNotExists,
		/// Liquidity is not enough.
		InsufficientLiquidity,
		/// Trading pair does have enough foreign.
		InsufficientPairReserve,
		/// Get target amount is less than exception.
		InsufficientTargetAmount,
		/// Sold amount is more than exception.
		ExcessiveSoldAmount,
		/// Can't find pair though trading path.
		InvalidPath,
		/// Incorrect foreign amount range.
		IncorrectAssetAmountRange,
		/// Overflow.
		Overflow,
		/// Transaction block number is larger than the end block number.
		Deadline,
		/// Location given was invalid or unsupported.
		AccountIdBadLocation,
		/// XCM execution failed.
		ExecutionFailed,
		/// Transfer to self by XCM message.
		DeniedTransferToSelf,
		/// Not in ZenlinkRegistedParaChains.
		TargetChainNotRegistered,
		/// Can't pass the K value check
		InvariantCheckFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the new admin of the protocol fee.
		///
		/// # Arguments
		///
		/// - `new_admin`: The new admin account.
		#[pallet::weight(1_000_000)]
		pub fn set_fee_admin(origin: OriginFor<T>, new_admin: <T::Lookup as StaticLookup>::Source) -> DispatchResult {
			if let Ok(frame_system::RawOrigin::Root) = origin.clone().into() {
				// do nothing
			} else {
				let origin = ensure_signed(origin)?;
				ensure!(origin == Self::fee_meta().0, Error::<T>::RequireProtocolAdmin);
			}

			let new_admin = T::Lookup::lookup(new_admin)?;

			FeeMeta::<T>::mutate(|fee_meta| (*fee_meta).0 = new_admin);

			Ok(())
		}

		/// Set the new receiver of the protocol fee.
		///
		/// # Arguments
		///
		/// - `send_to`:
		/// (1) Some(receiver): it turn on the protocol fee and the new receiver account.
		/// (2) None: it turn off the protocol fee.
		#[pallet::weight(1_000_000)]
		pub fn set_fee_receiver(
			origin: OriginFor<T>,
			send_to: Option<<T::Lookup as StaticLookup>::Source>,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(origin == Self::fee_meta().0, Error::<T>::RequireProtocolAdmin);

			let receiver = match send_to {
				Some(r) => {
					let account = T::Lookup::lookup(r)?;
					Some(account)
				}
				None => None,
			};

			FeeMeta::<T>::mutate(|fee_meta| (*fee_meta).1 = receiver);

			Ok(())
		}

		/// Set the protocol fee point.
		///
		/// # Arguments
		///
		/// - `fee_point`:
		/// The fee_point which integer between [0,30]
		/// 0 means no protocol fee.
		/// 30 means 0.3% * 100% = 0.0030.
		/// default is 5 and means 0.3% * 1 / 6 = 0.0005.
		#[pallet::weight(1_000_000)]
		pub fn set_fee_point(origin: OriginFor<T>, fee_point: u8) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			ensure!(origin == Self::fee_meta().0, Error::<T>::RequireProtocolAdmin);
			ensure!(fee_point <= 30, Error::<T>::InvalidFeePoint);

			FeeMeta::<T>::mutate(|fee_meta| (*fee_meta).2 = fee_point);

			Ok(())
		}

		/// Move some assets from one holder to another.
		///
		/// # Arguments
		///
		/// - `asset_id`: The foreign id.
		/// - `target`: The receiver of the foreign.
		/// - `amount`: The amount of the foreign to transfer.
		#[pallet::weight(1_000_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: AssetId,
			recipient: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] amount: AssetBalance,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			let target = T::Lookup::lookup(recipient)?;
			let balance = T::MultiAssetsHandler::balance_of(asset_id, &origin);
			ensure!(balance >= amount, Error::<T>::InsufficientAssetBalance);

			T::MultiAssetsHandler::transfer(asset_id, &origin, &target, amount)?;

			Ok(())
		}

		/// Transfer zenlink assets to a sibling parachain.
		///
		/// Zenlink assets can be either native or foreign to the sending parachain.
		///
		/// # Arguments
		///
		/// - `asset_id`: Global identifier for a zenlink foreign
		/// - `para_id`: Destination parachain
		/// - `account`: Destination account
		/// - `amount`: Amount to transfer
		#[pallet::weight(max_weight.saturating_add(100_000_000u64))]
		#[frame_support::transactional]
		pub fn transfer_to_parachain(
			origin: OriginFor<T>,
			asset_id: AssetId,
			para_id: ParaId,
			recipient: T::AccountId,
			#[pallet::compact] amount: AssetBalance,
			max_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			let balance = T::MultiAssetsHandler::balance_of(asset_id, &who);
			let checked = Self::check_existential_deposit(asset_id, amount);
			ensure!(asset_id.is_support(), Error::<T>::UnsupportedAssetType);
			ensure!(para_id != T::SelfParaId::get().into(), Error::<T>::DeniedTransferToSelf);
			ensure!(checked.is_some(), Error::<T>::TargetChainNotRegistered);
			ensure!(Some(true) == checked, Error::<T>::NativeBalanceTooLow);
			ensure!(balance >= amount, Error::<T>::InsufficientAssetBalance);

			let xcm_target = T::Conversion::reverse(recipient.clone()).map_err(|_| Error::<T>::AccountIdBadLocation)?;

			let xcm = Self::make_xcm_transfer_to_parachain(&asset_id, para_id, xcm_target, amount)
				.map_err(|_| Error::<T>::AssetNotExists)?;

			let xcm_origin = T::Conversion::reverse(who.clone()).map_err(|_| Error::<T>::AccountIdBadLocation)?;

			log::info! {
				target: LOG_TARGET,
				"transfer_to_parachain xcm = {:?}",
				xcm
			}

			let out_come = T::XcmExecutor::execute_xcm(xcm_origin, xcm, max_weight);
			match out_come {
				Outcome::Complete(weight) => {
					Self::deposit_event(Event::<T>::TransferredToParachain(
						asset_id, who, para_id, recipient, amount, weight,
					));

					Ok(())
				}
				Outcome::Incomplete(weight, err) => {
					log::info! {
						target: LOG_TARGET,
						"transfer_to_parachain is rollback: xcm outcome Incomplete, weight = {:?}, err = {:?}",
						weight, err
					}

					Err(Error::<T>::ExecutionFailed.into())
				}

				Outcome::Error(err) => {
					log::info! {
						target: LOG_TARGET,
						"transfer_to_parachain is rollback: xcm outcome Error, err = {:?}",
						err
					}

					Err(Error::<T>::ExecutionFailed.into())
				}
			}
		}

		/// Create pair by two assets.
		///
		/// The order of foreign dot effect result.
		///
		/// # Arguments
		///
		/// - `asset_0`: Asset which make up Pair
		/// - `asset_1`: Asset which make up Pair
		#[pallet::weight(1_000_000)]
		pub fn create_pair(origin: OriginFor<T>, asset_0: AssetId, asset_1: AssetId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				asset_0.is_support() && asset_1.is_support(),
				Error::<T>::UnsupportedAssetType
			);
			ensure!(asset_0 != asset_1, Error::<T>::DeniedCreatePair);
			ensure!(T::MultiAssetsHandler::is_exists(asset_0), Error::<T>::AssetNotExists);
			ensure!(T::MultiAssetsHandler::is_exists(asset_1), Error::<T>::AssetNotExists);

			let (asset_0, asset_1) = Self::sort_asset_id(asset_0, asset_1);

			let pair_account = Self::pair_account_id(asset_0, asset_1);

			LiquidityMeta::<T>::try_mutate((asset_0, asset_1), |meta| {
				if meta.is_none() {
					*meta = Some((pair_account, Default::default()));

					Self::mutate_lp_pairs(asset_0, asset_1);

					Self::deposit_event(Event::PairCreated(who, asset_0, asset_1));

					Ok(())
				} else {
					Err(Error::<T>::PairAlreadyExists.into())
				}
			})
		}

		/// Provide liquidity to a pair.
		///
		/// The order of foreign dot effect result.
		///
		/// # Arguments
		///
		/// - `asset_0`: Asset which make up pair
		/// - `asset_1`: Asset which make up pair
		/// - `amount_0_desired`: Maximum amount of asset_0 added to the pair
		/// - `amount_1_desired`: Maximum amount of asset_1 added to the pair
		/// - `amount_0_min`: Minimum amount of asset_0 added to the pair
		/// - `amount_1_min`: Minimum amount of asset_1 added to the pair
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(1_000_000)]
		#[frame_support::transactional]
		#[allow(clippy::too_many_arguments)]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			asset_0: AssetId,
			asset_1: AssetId,
			#[pallet::compact] amount_0_desired: AssetBalance,
			#[pallet::compact] amount_1_desired: AssetBalance,
			#[pallet::compact] amount_0_min: AssetBalance,
			#[pallet::compact] amount_1_min: AssetBalance,
			#[pallet::compact] deadline: T::BlockNumber,
		) -> DispatchResult {
			ensure!(
				asset_0.is_support() && asset_1.is_support(),
				Error::<T>::UnsupportedAssetType
			);
			let who = ensure_signed(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_add_liquidity(
				&who,
				asset_0,
				asset_1,
				amount_0_desired,
				amount_1_desired,
				amount_0_min,
				amount_1_min,
			)
		}

		/// Extract liquidity.
		///
		/// The order of foreign dot effect result.
		///
		/// # Arguments
		///
		/// - `asset_0`: Asset which make up pair
		/// - `asset_1`: Asset which make up pair
		/// - `amount_asset_0_min`: Minimum amount of asset_0 to exact
		/// - `amount_asset_1_min`: Minimum amount of asset_1 to exact
		/// - `recipient`: Account that accepts withdrawal of assets
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(1_000_000)]
		#[frame_support::transactional]
		#[allow(clippy::too_many_arguments)]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			asset_0: AssetId,
			asset_1: AssetId,
			#[pallet::compact] liquidity: AssetBalance,
			#[pallet::compact] amount_0_min: AssetBalance,
			#[pallet::compact] amount_1_min: AssetBalance,
			recipient: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] deadline: T::BlockNumber,
		) -> DispatchResult {
			ensure!(
				asset_0.is_support() && asset_1.is_support(),
				Error::<T>::UnsupportedAssetType
			);
			let who = ensure_signed(origin)?;
			let recipient = T::Lookup::lookup(recipient)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_remove_liquidity(
				&who,
				asset_0,
				asset_1,
				liquidity,
				amount_0_min,
				amount_1_min,
				&recipient,
			)
		}

		/// Sell amount of foreign by path.
		///
		/// # Arguments
		///
		/// - `amount_in`: Amount of the foreign will be sold
		/// - `amount_out_min`: Minimum amount of target foreign
		/// - `path`: path can convert to pairs.
		/// - `recipient`: Account that receive the target foreign
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(1_000_000)]
		#[frame_support::transactional]
		pub fn swap_exact_assets_for_assets(
			origin: OriginFor<T>,
			#[pallet::compact] amount_in: AssetBalance,
			#[pallet::compact] amount_out_min: AssetBalance,
			path: Vec<AssetId>,
			recipient: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] deadline: T::BlockNumber,
		) -> DispatchResult {
			ensure!(path.iter().all(|id| id.is_support()), Error::<T>::UnsupportedAssetType);
			let who = ensure_signed(origin)?;
			let recipient = T::Lookup::lookup(recipient)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_swap_exact_assets_for_assets(&who, amount_in, amount_out_min, &path, &recipient)
		}

		/// Buy amount of foreign by path.
		///
		/// # Arguments
		///
		/// - `amount_out`: Amount of the foreign will be bought
		/// - `amount_in_max`: Maximum amount of sold foreign
		/// - `path`: path can convert to pairs.
		/// - `recipient`: Account that receive the target foreign
		/// - `deadline`: Height of the cutoff block of this transaction
		#[pallet::weight(1_000_000)]
		#[frame_support::transactional]
		pub fn swap_assets_for_exact_assets(
			origin: OriginFor<T>,
			#[pallet::compact] amount_out: AssetBalance,
			#[pallet::compact] amount_in_max: AssetBalance,
			path: Vec<AssetId>,
			recipient: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] deadline: T::BlockNumber,
		) -> DispatchResult {
			ensure!(path.iter().all(|id| id.is_support()), Error::<T>::UnsupportedAssetType);
			let who = ensure_signed(origin)?;
			let recipient = T::Lookup::lookup(recipient)?;
			let now = frame_system::Pallet::<T>::block_number();
			ensure!(deadline > now, Error::<T>::Deadline);

			Self::inner_swap_assets_for_exact_assets(&who, amount_out, amount_in_max, &path, &recipient)
		}
	}
}
