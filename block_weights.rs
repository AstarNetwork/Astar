
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-01-19 (Y/M/D)
//! HOSTNAME: `Shunsukes-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//!
//! SHORT-NAME: `block`, LONG-NAME: `BlockExecution`, RUNTIME: `Development`
//! WARMUPS: `10`, REPEAT: `100`
//! WEIGHT-PATH: ``
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   ./target/debug/astar-collator
//   benchmark
//   overhead
//   --dev

use sp_core::parameter_types;
use sp_weights::{constants::WEIGHT_PER_NANOS, Weight};

parameter_types! {
	/// Time to execute an empty block.
	/// Calculated by multiplying the *Average* with `1.0` and adding `0`.
	///
	/// Stats nanoseconds:
	///   Min, Max: 5_522_958, 5_837_375
	///   Average:  5_557_704
	///   Median:   5_540_750
	///   Std-Dev:  54776.34
	///
	/// Percentiles nanoseconds:
	///   99th: 5_820_583
	///   95th: 5_678_625
	///   75th: 5_559_041
	pub const BlockExecutionWeight: Weight = WEIGHT_PER_NANOS.saturating_mul(5_557_704);
}

#[cfg(test)]
mod test_weights {
	use sp_weights::constants;

	/// Checks that the weight exists and is sane.
	// NOTE: If this test fails but you are sure that the generated values are fine,
	// you can delete it.
	#[test]
	fn sane() {
		let w = super::BlockExecutionWeight::get();

		// At least 100 µs.
		assert!(
			w.ref_time() >= 100u64 * constants::WEIGHT_PER_MICROS.ref_time(),
			"Weight should be at least 100 µs."
		);
		// At most 50 ms.
		assert!(
			w.ref_time() <= 50u64 * constants::WEIGHT_PER_MILLIS.ref_time(),
			"Weight should be at most 50 ms."
		);
	}
}
