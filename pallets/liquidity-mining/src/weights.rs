
//! Autogenerated weights for bifrost_liquidity_mining
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
// --pallet=bifrost_liquidity_mining
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./runtime/bifrost-kusama/src/weights/bifrost_liquidity_mining.rs
// --template=./frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for bifrost_liquidity_mining.
pub trait WeightInfo {
	fn charge() -> Weight;
	fn deposit() -> Weight;
	fn redeem() -> Weight;
	fn redeem_all() -> Weight;
	fn volunteer_to_redeem() -> Weight;
	fn claim() -> Weight;
	fn unlock() -> Weight;
	fn cancel_unlock() -> Weight;
}

/// Weights for bifrost_liquidity_mining using the Substrate node and recommended hardware.
pub struct BifrostWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for BifrostWeight<T> {
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining ChargedPoolIds (r:1 w:1)
	/// Proof Skipped: LiquidityMining ChargedPoolIds (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn charge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2124`
		//  Estimated: `24700`
		// Minimum execution time: 133_752_000 picoseconds.
		Weight::from_parts(137_628_000, 24700)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:4 w:4)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn deposit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2317`
		//  Estimated: `32638`
		// Minimum execution time: 139_180_000 picoseconds.
		Weight::from_parts(142_300_000, 32638)
			.saturating_add(T::DbWeight::get().reads(10_u64))
			.saturating_add(T::DbWeight::get().writes(7_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:7 w:6)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2824`
		//  Estimated: `45048`
		// Minimum execution time: 241_295_000 picoseconds.
		Weight::from_parts(245_871_000, 45048)
			.saturating_add(T::DbWeight::get().reads(14_u64))
			.saturating_add(T::DbWeight::get().writes(9_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:7 w:6)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn redeem_all() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2824`
		//  Estimated: `45048`
		// Minimum execution time: 237_953_000 picoseconds.
		Weight::from_parts(246_523_000, 45048)
			.saturating_add(T::DbWeight::get().reads(14_u64))
			.saturating_add(T::DbWeight::get().writes(9_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:2 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:7 w:6)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn volunteer_to_redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2824`
		//  Estimated: `47523`
		// Minimum execution time: 256_024_000 picoseconds.
		Weight::from_parts(262_125_000, 47523)
			.saturating_add(T::DbWeight::get().reads(15_u64))
			.saturating_add(T::DbWeight::get().writes(9_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn claim() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2548`
		//  Estimated: `28376`
		// Minimum execution time: 136_439_000 picoseconds.
		Weight::from_parts(140_657_000, 28376)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:5 w:4)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn unlock() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2655`
		//  Estimated: `39186`
		// Minimum execution time: 150_144_000 picoseconds.
		Weight::from_parts(153_822_000, 39186)
			.saturating_add(T::DbWeight::get().reads(12_u64))
			.saturating_add(T::DbWeight::get().writes(7_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn cancel_unlock() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1204`
		//  Estimated: `12736`
		// Minimum execution time: 47_198_000 picoseconds.
		Weight::from_parts(49_930_000, 12736)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining ChargedPoolIds (r:1 w:1)
	/// Proof Skipped: LiquidityMining ChargedPoolIds (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn charge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2124`
		//  Estimated: `24700`
		// Minimum execution time: 133_752_000 picoseconds.
		Weight::from_parts(137_628_000, 24700)
			.saturating_add(RocksDbWeight::get().reads(8_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:4 w:4)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn deposit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2317`
		//  Estimated: `32638`
		// Minimum execution time: 139_180_000 picoseconds.
		Weight::from_parts(142_300_000, 32638)
			.saturating_add(RocksDbWeight::get().reads(10_u64))
			.saturating_add(RocksDbWeight::get().writes(7_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:7 w:6)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2824`
		//  Estimated: `45048`
		// Minimum execution time: 241_295_000 picoseconds.
		Weight::from_parts(245_871_000, 45048)
			.saturating_add(RocksDbWeight::get().reads(14_u64))
			.saturating_add(RocksDbWeight::get().writes(9_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:7 w:6)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn redeem_all() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2824`
		//  Estimated: `45048`
		// Minimum execution time: 237_953_000 picoseconds.
		Weight::from_parts(246_523_000, 45048)
			.saturating_add(RocksDbWeight::get().reads(14_u64))
			.saturating_add(RocksDbWeight::get().writes(9_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:2 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:7 w:6)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn volunteer_to_redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2824`
		//  Estimated: `47523`
		// Minimum execution time: 256_024_000 picoseconds.
		Weight::from_parts(262_125_000, 47523)
			.saturating_add(RocksDbWeight::get().reads(15_u64))
			.saturating_add(RocksDbWeight::get().writes(9_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn claim() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2548`
		//  Estimated: `28376`
		// Minimum execution time: 136_439_000 picoseconds.
		Weight::from_parts(140_657_000, 28376)
			.saturating_add(RocksDbWeight::get().reads(8_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:5 w:4)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:2 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: System Account (r:2 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn unlock() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2655`
		//  Estimated: `39186`
		// Minimum execution time: 150_144_000 picoseconds.
		Weight::from_parts(153_822_000, 39186)
			.saturating_add(RocksDbWeight::get().reads(12_u64))
			.saturating_add(RocksDbWeight::get().writes(7_u64))
	}
	/// Storage: LiquidityMining PalletVersion (r:1 w:0)
	/// Proof Skipped: LiquidityMining PalletVersion (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalPoolInfosV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalPoolInfosV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: LiquidityMining TotalDepositDataV200 (r:1 w:1)
	/// Proof Skipped: LiquidityMining TotalDepositDataV200 (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn cancel_unlock() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1204`
		//  Estimated: `12736`
		// Minimum execution time: 47_198_000 picoseconds.
		Weight::from_parts(49_930_000, 12736)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
}
