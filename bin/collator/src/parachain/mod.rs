// This file is part of Astar.

// Copyright (C) Stake Technologies Pte.Ltd.
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

//! Support for Astar ecosystem parachains.

/// Shell to Aura consensus upgrades.
mod shell_upgrade;

/// Parachain specified service.
pub mod service;

/// Parachain specs.
pub mod chain_spec;

pub mod fake_runtime_api;

pub use service::{build_import_queue, new_partial, start_node, HostFunctions};

pub(crate) use shell_upgrade::{
    AuraConsensusDataProviderFallback, PendingCrateInherentDataProvider,
};
