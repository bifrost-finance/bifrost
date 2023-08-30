//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn transfer_non_native_currency() -> Weight {
		Weight::from_parts(172_011_000, 0)
			.saturating_add(DbWeight::get().reads(5 as u64))
			.saturating_add(DbWeight::get().writes(2 as u64))
	}
	fn transfer_native_currency() -> Weight {
		Weight::from_parts(43_023_000, 0)
	}
	fn update_balance_non_native_currency() -> Weight {
		Weight::from_parts(137_440_000, 0)
			.saturating_add(DbWeight::get().reads(5 as u64))
			.saturating_add(DbWeight::get().writes(2 as u64))
	}
	fn update_balance_native_currency_creating() -> Weight {
		Weight::from_parts(64_432_000, 0)
	}
	fn update_balance_native_currency_killing() -> Weight {
		Weight::from_parts(62_595_000, 0)
	}
}
