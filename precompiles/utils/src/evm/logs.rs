// This file is part of Astar.

// Copyright 2019-2022 PureStake Inc.
// Copyright (C) 2022-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later
//
// This file is part of Utils package, originally developed by Purestake Inc.
// Utils package used in Astar Network in terms of GPLv3.
//
// Utils is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Utils is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Utils.  If not, see <http://www.gnu.org/licenses/>.

use {
    crate::EvmResult,
    pallet_evm::{Log, PrecompileHandle},
    sp_core::{H160, H256},
    sp_std::{vec, vec::Vec},
};

/// Create a 0-topic log.
#[must_use]
pub fn log0(address: impl Into<H160>, data: impl Into<Vec<u8>>) -> Log {
    Log {
        address: address.into(),
        topics: vec![],
        data: data.into(),
    }
}

/// Create a 1-topic log.
#[must_use]
pub fn log1(address: impl Into<H160>, topic0: impl Into<H256>, data: impl Into<Vec<u8>>) -> Log {
    Log {
        address: address.into(),
        topics: vec![topic0.into()],
        data: data.into(),
    }
}

/// Create a 2-topics log.
#[must_use]
pub fn log2(
    address: impl Into<H160>,
    topic0: impl Into<H256>,
    topic1: impl Into<H256>,
    data: impl Into<Vec<u8>>,
) -> Log {
    Log {
        address: address.into(),
        topics: vec![topic0.into(), topic1.into()],
        data: data.into(),
    }
}

/// Create a 3-topics log.
#[must_use]
pub fn log3(
    address: impl Into<H160>,
    topic0: impl Into<H256>,
    topic1: impl Into<H256>,
    topic2: impl Into<H256>,
    data: impl Into<Vec<u8>>,
) -> Log {
    Log {
        address: address.into(),
        topics: vec![topic0.into(), topic1.into(), topic2.into()],
        data: data.into(),
    }
}

/// Create a 4-topics log.
#[must_use]
pub fn log4(
    address: impl Into<H160>,
    topic0: impl Into<H256>,
    topic1: impl Into<H256>,
    topic2: impl Into<H256>,
    topic3: impl Into<H256>,
    data: impl Into<Vec<u8>>,
) -> Log {
    Log {
        address: address.into(),
        topics: vec![topic0.into(), topic1.into(), topic2.into(), topic3.into()],
        data: data.into(),
    }
}

/// Extension trait allowing to record logs into a PrecompileHandle.
pub trait LogExt {
    fn record(self, handle: &mut impl PrecompileHandle) -> EvmResult;

    fn compute_cost(&self) -> EvmResult<u64>;
}

impl LogExt for Log {
    fn record(self, handle: &mut impl PrecompileHandle) -> EvmResult {
        handle.log(self.address, self.topics, self.data)?;
        Ok(())
    }

    fn compute_cost(&self) -> EvmResult<u64> {
        crate::evm::costs::log_costs(self.topics.len(), self.data.len())
    }
}
