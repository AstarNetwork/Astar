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

use sp_core_hashing::keccak_256;

#[test]
fn test_keccak256() {
    assert_eq!(
        &precompile_utils_macro_v2::keccak256!(""),
        keccak_256(b"").as_slice(),
    );
    assert_eq!(
        &precompile_utils_macro_v2::keccak256!("toto()"),
        keccak_256(b"toto()").as_slice(),
    );
    assert_ne!(
        &precompile_utils_macro_v2::keccak256!("toto()"),
        keccak_256(b"tata()").as_slice(),
    );
}

#[test]
#[ignore]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/**/*.rs");
    t.pass("tests/pass/**/*.rs");
}

// Cargo expand is not supported on stable rust
#[test]
#[ignore]
fn expand() {
    // Use `expand` to update the expansions
    // Replace it with `expand_without_refresh` afterward so that
    // CI checks the expension don't change

    // macrotest::expand("tests/expand/**/*.rs");
    macrotest::expand_without_refresh("tests/expand/**/*.rs");
}
