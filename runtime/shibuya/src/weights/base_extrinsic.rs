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

// This is just a dummy file.

use sp_core::parameter_types;
use sp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

// parameter_types! {
//     /// Time to execute a NO-OP extrinsic, for example `System::remark`.
//     /// Calculated by multiplying the *Average* with `1.0` and adding `0`.
//     ///
//     /// Stats nanoseconds:
//     ///   Min, Max: 106_559, 107_788
//     ///   Average:  107_074
//     ///   Median:   107_067
//     ///   Std-Dev:  242.67
//     ///
//     /// Percentiles nanoseconds:
//     ///   99th: 107_675
//     ///   95th: 107_513
//     ///   75th: 107_225
//     pub const ExtrinsicBaseWeight: Weight =
//         Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(107_074), 0);
// }

parameter_types! {
    /// Time to execute a NO-OP extrinsic, for example `System::remark`.
    /// Calculated by multiplying the *Average* with `1.0` and adding `0`.
    ///
    /// Stats nanoseconds:
    ///   Min, Max: 87_894, 99_268
    ///   Average:  90_705
    ///   Median:   90_325
    ///   Std-Dev:  2344.95
    ///
    /// Percentiles nanoseconds:
    ///   99th: 99_232
    ///   95th: 96_405
    ///   75th: 91_204
    pub const ExtrinsicBaseWeight: Weight =
        Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(90_705), 0);
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
