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

//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-07-21 (Y/M/D)
//! HOSTNAME: `gh-runner-01-ovh`, CPU: `Intel(R) Xeon(R) E-2236 CPU @ 3.40GHz`
//!
//! SHORT-NAME: `extrinsic`, LONG-NAME: `ExtrinsicBase`, RUNTIME: `shibuya`
//! WARMUPS: `10`, REPEAT: `50`
//! WEIGHT-PATH: `./benchmark-results/shibuya`
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   frame-omni-bencher
//   v1
//   benchmark
//   overhead
//   --runtime=./target/release/wbuild/shibuya-runtime/shibuya_runtime.compact.compressed.wasm
//   --repeat=50
//   --weight-path=./benchmark-results/shibuya

use sp_core::parameter_types;
use sp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

parameter_types! {
    /// Weight of executing a NO-OP extrinsic, for example `System::remark`.
    /// Calculated by multiplying the *Average* with `1.0` and adding `0`.
    ///
    /// Stats nanoseconds:
    ///   Min, Max: 108_407, 110_306
    ///   Average:  109_119
    ///   Median:   109_001
    ///   Std-Dev:  361.43
    ///
    /// Percentiles nanoseconds:
    ///   99th: 110_306
    ///   95th: 109_704
    ///   75th: 109_290
    pub const ExtrinsicBaseWeight: Weight =
        Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(109_119), 166);
}

#[cfg(test)]
mod test_weights {
    use sp_weights::constants;

    /// Checks that the weight exists and is sane.
    // NOTE: If this test fails but you are sure that the generated values are fine,
    // you can delete it.
    #[test]
    fn sane() {
        let w = super::ExtrinsicBaseWeight::get();

        // At least 10 µs.
        assert!(
            w.ref_time() >= 10u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
            "Weight should be at least 10 µs."
        );
        // At most 1 ms.
        assert!(
            w.ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
            "Weight should be at most 1 ms."
        );
    }
}
