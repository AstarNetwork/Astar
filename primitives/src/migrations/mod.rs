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

#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::traits::{OnRuntimeUpgrade, PalletInfoAccess, StorageVersion};
use frame_support::weights::Weight;
use sp_core::Get;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use sp_std::marker::PhantomData;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

pub mod contract_v12;
pub mod contract_v12_fix;
pub mod contract_v14;

/// Force set pallet-contracts version
pub struct ForceContractsVersion<T: pallet_contracts::Config, const V: u16> {
    _phantom: PhantomData<T>,
}

impl<T: pallet_contracts::Config, const V: u16> OnRuntimeUpgrade for ForceContractsVersion<T, V> {
    fn on_runtime_upgrade() -> Weight {
        StorageVersion::new(V).put::<pallet_contracts::Pallet<T>>();
        <T as frame_system::Config>::DbWeight::get().writes(1)
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
        let pallet_version = StorageVersion::get::<pallet_contracts::Pallet<T>>();
        ensure!(pallet_version == V, "Contracts pallet version mismatch");
        Ok(())
    }
}

/// Noop migration that checks for pallet version in try-runtime

pub struct EnsurePalletVersion<P: PalletInfoAccess, const V: u16> {
    _phantam: PhantomData<P>,
}

impl<P: PalletInfoAccess, const V: u16> OnRuntimeUpgrade for EnsurePalletVersion<P, V> {
    #[cfg(feature = "try-runtime")]
    fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
        let pallet_version = StorageVersion::get::<P>();
        ensure!(pallet_version == V, "Pallet version mismatch");
        Ok(())
    }
}
