// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

use frame_support::log;
use frame_support::{
    log,
    parameter_types,
    traits::nonfungibles::{Inspect, Mutate, Transfer},
    weights::Weight
};
use sp_runtime::DispatchResult;

// Polkadot imports
use polkadot_parachain::primitives::Sibling;
use polkadot_primitives::AccountId;
use xcm::latest::prelude::*;
use xcm::v2::Junctions::X1;
use xcm_builder::{
    AccountId32Aliases, ConvertedConcreteId, NoChecking, NonFungiblesAdapter, ParentIsPreset,
    SiblingParachainConvertsVia,
};
use xcm_executor::traits::JustTry;

type CollectionId = MultiLocation;
type ItemId = AssetInstance;

pub struct NftAdapter;
const SELECTOR_FLIP: [u8; 4] = [0x63, 0x3a, 0xa5, 0x51];

impl Mutate<AccountId> for NftAdapter {
    fn mint_into(collection_ml: &CollectionId, _item: &ItemId, _who: &AccountId) -> DispatchResult {
        log::trace!(target: "runtime", "########### mint_into {:?} {:?} {:?}", collection_ml, _item, _who);
        let contract_id : AccountId = collection_ml.interior().clone().into();

        let call = RuntimeCall::Contracts(pallet_contracts::Call::call {
            dest: contract_id.clone(),
            value: 0,
            gas_limit: Weight::from_parts(100_000_000_000, 1024 * 1024),
            storage_deposit_limit: None,
            data: SELECTOR_FLIP.to_vec(),
        });
        Ok(())
    }

    fn burn(
        _collection: &CollectionId,
        _item: &ItemId,
        _maybe_check_owner: Option<&AccountId>,
    ) -> DispatchResult {
        log::trace!(target: "runtime", "########### burn {:?} {:?} {:?}", _collection, _item, _maybe_check_owner);
        Ok(())
    }
}

impl Transfer<AccountId> for NftAdapter {
    fn transfer(
        _collection: &Self::CollectionId,
        _item: &Self::ItemId,
        _destination: &AccountId,
    ) -> DispatchResult {
        log::trace!(target: "runtime", "########### transfer {:?} {:?} {:?}", _collection, _item, _destination);
        Ok(())
    }
}

impl Inspect<AccountId> for NftAdapter {
    type ItemId = ItemId;
    type CollectionId = CollectionId;

    fn owner(_collection: &Self::CollectionId, _item: &Self::ItemId) -> Option<AccountId> {
        None
    }
}

parameter_types! {
    pub const RelayNetwork: Option<NetworkId> = Some(NetworkId::Rococo);
}

pub type SovereignAccountOf = (
    SiblingParachainConvertsVia<Sibling, AccountId>,
    AccountId32Aliases<RelayNetwork, AccountId>,
    ParentIsPreset<AccountId>,
);

/// Means for transacting non-fungibles assets
pub type NonFungiblesTransactor = NonFungiblesAdapter<
    NftAdapter,
    ConvertedConcreteId<MultiLocation, AssetInstance, JustTry, JustTry>,
    SovereignAccountOf,
    AccountId,
    NoChecking,
    (),
>;
