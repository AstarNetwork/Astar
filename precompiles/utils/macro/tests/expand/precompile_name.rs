// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
// This file is part of Frontier.
//
// Copyright (c) 2019-2022 Moonsong Labs.
// Copyright (c) 2023 Parity Technologies (UK) Ltd.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// Few mock structs to check the macro.
struct PrecompileAt<T, U, V = ()>(PhantomData<(T, U, V)>);
struct AddressU64<const N: u64>;
struct FooPrecompile<R>(PhantomData<R>);
struct BarPrecompile<R, S>(PhantomData<(R, S)>);
struct MockCheck;

#[precompile_utils_macro::precompile_name_from_address]
type Precompiles = (
	PrecompileAt<AddressU64<1>, FooPrecompile<R>>,
	PrecompileAt<AddressU64<2>, BarPrecompile<R, S>, (MockCheck, MockCheck)>,
);
