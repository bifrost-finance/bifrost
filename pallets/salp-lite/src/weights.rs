
//! Autogenerated weights for bifrost_salp_lite
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-05-18, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `VM-16-3-ubuntu`, CPU: `Intel(R) Xeon(R) Platinum 8374C CPU @ 2.70GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("bifrost-kusama-local"), DB CACHE: 1024

// Executed Command:
// target/release/bifrost
// benchmark
// pallet
// --chain=bifrost-kusama-local
// --steps=50
// --repeat=20
// --pallet=bifrost_salp_lite
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./runtime/bifrost-kusama/src/weights/bifrost_salp_lite.rs
// --template=./frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for bifrost_salp_lite.
pub trait WeightInfo {
	fn redeem() -> Weight;
	fn refund() -> Weight;
}

/// Weights for bifrost_salp_lite using the Substrate node and recommended hardware.
pub struct BifrostWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for BifrostWeight<T> {
	/// Storage: SalpLite Funds (r:1 w:1)
	/// Proof Skipped: SalpLite Funds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SalpLite RedeemPool (r:1 w:1)
	/// Proof Skipped: SalpLite RedeemPool (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens TotalIssuance (r:2 w:2)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1979`
		//  Estimated: `21594`
		// Minimum execution time: 103_169_000 picoseconds.
		Weight::from_parts(105_168_000, 21594)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}
	/// Storage: SalpLite FailedFundsToRefund (r:1 w:0)
	/// Proof Skipped: SalpLite FailedFundsToRefund (max_values: None, max_size: None, mode: Measured)
	/// Storage: SalpLite Funds (r:1 w:1)
	/// Proof Skipped: SalpLite Funds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SalpLite RedeemPool (r:1 w:1)
	/// Proof Skipped: SalpLite RedeemPool (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens TotalIssuance (r:2 w:2)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn refund() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1998`
		//  Estimated: `26124`
		// Minimum execution time: 110_331_000 picoseconds.
		Weight::from_parts(113_989_000, 26124)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: SalpLite Funds (r:1 w:1)
	/// Proof Skipped: SalpLite Funds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SalpLite RedeemPool (r:1 w:1)
	/// Proof Skipped: SalpLite RedeemPool (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens TotalIssuance (r:2 w:2)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1979`
		//  Estimated: `21594`
		// Minimum execution time: 103_169_000 picoseconds.
		Weight::from_parts(105_168_000, 21594)
			.saturating_add(RocksDbWeight::get().reads(7_u64))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
	}
	/// Storage: SalpLite FailedFundsToRefund (r:1 w:0)
	/// Proof Skipped: SalpLite FailedFundsToRefund (max_values: None, max_size: None, mode: Measured)
	/// Storage: SalpLite Funds (r:1 w:1)
	/// Proof Skipped: SalpLite Funds (max_values: None, max_size: None, mode: Measured)
	/// Storage: SalpLite RedeemPool (r:1 w:1)
	/// Proof Skipped: SalpLite RedeemPool (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens TotalIssuance (r:2 w:2)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn refund() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1998`
		//  Estimated: `26124`
		// Minimum execution time: 110_331_000 picoseconds.
		Weight::from_parts(113_989_000, 26124)
			.saturating_add(RocksDbWeight::get().reads(8_u64))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
	}
}
