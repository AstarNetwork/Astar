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

use super::{Pallet as Migration, *};

use frame_benchmarking::{account as benchmark_account, v2::*};
use astar_primitives::{dapp_staking::SmartContractHandle, Balance};
use pallet_dapp_staking_v3::StakeAmount;

fn smart_contract<T: Config>(idx: u8) -> T::SmartContract {
    let address: T::AccountId = benchmark_account("smart_contract", idx.into(), 456);
    T::SmartContract::wasm(address)
}

pub(super) fn initial_config<T: Config>() {
    for idx in 0..10 {
        let account: T::AccountId = benchmark_account("developer", idx.into(), 123);
        let smart_contract = smart_contract::<T>(idx);

        v5::StakerInfo::<T>::insert(
            &account,
            &smart_contract,
            v5::SingularStakingInfo {
                staked: StakeAmount {
                    voting: 123 * (idx as Balance + 1),
                    build_and_earn: 345 * (idx as Balance + 1),
                    era: 1,
                    period: 2,
                },
                loyal_staker: true,
            },
        );
    }
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn translate_staking_info_success() {
        initial_config::<T>();

        #[block]
        {
            assert!(Migration::<T>::translate_staking_info(None).is_ok());
        }
    }

    #[benchmark]
    fn translate_staking_info_success_noop() {
        #[block]
        {
            assert!(Migration::<T>::translate_staking_info(None).is_err());
        }
    }
}
