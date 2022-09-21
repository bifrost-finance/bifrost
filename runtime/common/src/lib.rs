// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
use bifrost_asset_registry::{AssetIdMaps, Config};
use frame_support::{
	parameter_types, sp_runtime::traits::BlockNumberProvider, traits::EitherOfDiverse,
};
use frame_system::EnsureRoot;
use node_primitives::{AccountId, Balance, BlockNumber, CurrencyId, CurrencyIdMapping, TokenInfo};
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{FixedPointNumber, Perquintill};

pub mod constants;

#[cfg(test)]
mod tests;

pub struct RelaychainBlockNumberProvider<T>(sp_std::marker::PhantomData<T>);

impl<T: cumulus_pallet_parachain_system::Config> BlockNumberProvider
	for RelaychainBlockNumberProvider<T>
{
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		cumulus_pallet_parachain_system::Pallet::<T>::validation_data()
			.map(|d| d.relay_parent_number)
			.unwrap_or_default()
	}
}

parameter_types! {
	/// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
	/// than this will decrease the weight and more will increase.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
	/// change the fees more rapidly.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(3, 100_000);
	/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
	/// that combined with `AdjustmentVariable`, we can recover from the minimum.
	/// See `multiplier_can_grow_from_zero`.
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000u128);
}

pub type SlowAdjustingFeeUpdate<R> =
	TargetedFeeAdjustment<R, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;

pub type CouncilCollective = pallet_collective::Instance1;

pub type TechnicalCollective = pallet_collective::Instance2;

pub type MoreThanHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

// Technical Committee Council
pub type EnsureRootOrAllTechnicalCommittee = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
>;

pub fn dollar<T: Config>(currency_id: CurrencyId) -> Balance {
	let decimals = currency_id
		.decimals()
		.unwrap_or(
			AssetIdMaps::<T>::get_currency_metadata(currency_id)
				.map_or(12, |metatata| metatata.decimals.into()),
		)
		.into();

	10u128.saturating_pow(decimals)
}

pub fn milli<T: Config>(currency_id: CurrencyId) -> Balance {
	dollar::<T>(currency_id) / 1000
}

pub fn micro<T: Config>(currency_id: CurrencyId) -> Balance {
	milli::<T>(currency_id) / 1000
}

pub fn cent<T: Config>(currency_id: CurrencyId) -> Balance {
	dollar::<T>(currency_id) / 100
}

pub fn millicent<T: Config>(currency_id: CurrencyId) -> Balance {
	cent::<T>(currency_id) / 1000
}

pub fn microcent<T: Config>(currency_id: CurrencyId) -> Balance {
	millicent::<T>(currency_id) / 1000
}

pub struct RelayChainBlockNumberProvider<T>(sp_std::marker::PhantomData<T>);

impl<T: cumulus_pallet_parachain_system::Config> BlockNumberProvider
	for RelayChainBlockNumberProvider<T>
{
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		cumulus_pallet_parachain_system::Pallet::<T>::validation_data()
			.map(|d| d.relay_parent_number)
			.unwrap_or_default()
	}
}

#[macro_export]
macro_rules! prod_or_test {
	($prod:expr, $test:expr) => {
		if cfg!(feature = "fast-runtime") {
			$test
		} else {
			$prod
		}
	};
	($prod:expr, $test:expr, $env:expr) => {
		if cfg!(feature = "fast-runtime") {
			core::option_env!($env).map(|s| s.parse().ok()).flatten().unwrap_or($test)
		} else {
			$prod
		}
	};
}
