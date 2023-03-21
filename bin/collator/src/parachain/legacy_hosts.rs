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

//! This file contatins legacy host functions that required for chain sync.

use sp_runtime_interface::runtime_interface;
use sp_std::vec::Vec;

type HostFunctions = (moonbeam_ext::HostFunctions,);

#[runtime_interface]
pub trait MoonbeamExt {
    fn raw_step(&mut self, _data: Vec<u8>) {}

    fn raw_gas(&mut self, _data: Vec<u8>) {}

    fn raw_return_value(&mut self, _data: Vec<u8>) {}

    fn call_list_entry(&mut self, _index: u32, _value: Vec<u8>) {}

    fn call_list_new(&mut self) {}

    fn evm_event(&mut self, _event: Vec<u8>) {}

    fn gasometer_event(&mut self, _event: Vec<u8>) {}

    fn runtime_event(&mut self, _event: Vec<u8>) {}

    fn step_event_filter(&self) -> (bool, bool) {}

    #[version(2)]
    fn call_list_new(&mut self) {}
}
