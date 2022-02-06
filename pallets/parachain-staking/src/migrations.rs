// Copyright 2019-2021 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! # Migrations
#[cfg(feature = "try-runtime")]
use frame_support::traits::OnRuntimeUpgradeHelpersExt;
#[cfg(feature = "try-runtime")]
use frame_support::Twox64Concat;

#[cfg(feature = "try-runtime")]
use crate::pallet::CandidatePool;
#[cfg(feature = "try-runtime")]
use crate::Delegator;
use crate::{
	inflation::{perbill_annual_to_perbill_round, InflationInfo, BLOCKS_PER_YEAR},
	pallet::{CollatorCommission, ParachainBondInfo, TotalSelected},
	BalanceOf, Config, InflationConfig, Pallet, ParachainBondConfig, Range, Round, RoundInfo,
	Staked, Total,
};
extern crate alloc;
#[cfg(feature = "try-runtime")]
use alloc::format;

use frame_support::{
	pallet_prelude::PhantomData,
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
};
use sp_runtime::{traits::AccountIdConversion, Perbill};

/// Migration to purge staking storage bloat for `Points` and `AtStake` storage items
pub struct InitGenesisMigration<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for InitGenesisMigration<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "InitMigration", "init genesis data");
		fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
			perbill_annual_to_perbill_round(
				annual,
				// rounds per year
				BLOCKS_PER_YEAR / 600,
			)
		}
		let annual = Range {
			min: Perbill::from_percent(5),
			ideal: Perbill::from_percent(5),
			max: Perbill::from_percent(5),
		};
		let expected: BalanceOf<T> = BalanceOf::<T>::from(T::PaymentInRound::get());

		let inflation_info: InflationInfo<BalanceOf<T>> = InflationInfo {
			// staking expectations
			expect: Range { min: expected, ideal: expected, max: expected },
			// annual inflation
			annual,
			round: to_round_inflation(annual),
		};
		<InflationConfig<T>>::put(inflation_info);
		let endowment: BalanceOf<T> = BalanceOf::<T>::from(T::MinCollatorStk::get());

		let mut candidate_count = 0u32;

		for &ref candidate in &T::ToMigrateInvulnables::get() {
			candidate_count += 1u32;
			if let Err(error) = <Pallet<T>>::join_candidates(
				T::Origin::from(Some(candidate.clone()).into()),
				endowment,
				candidate_count,
			) {
				log::warn!("Join candidates failed in genesis with error {:?}", error);
			} else {
				candidate_count += 1u32;
			}
		}
		// Set collator commission to default config
		<CollatorCommission<T>>::put(T::DefaultCollatorCommission::get());
		// Set parachain bond config to default config
		<ParachainBondInfo<T>>::put(ParachainBondConfig {
			// must be set soon; if not => due inflation will be sent to collators/delegators
			account: T::PalletId::get().into_account(),
			percent: T::DefaultParachainBondReservePercent::get(),
			payment_in_round: T::PaymentInRound::get(),
		});
		// Set total selected candidates to minimum config
		<TotalSelected<T>>::put(T::MinSelectedCandidates::get());
		// Choose top TotalSelected collator candidates
		<Pallet<T>>::select_top_candidates(1u32);
		// Start Round 1 at Block 0
		let round: RoundInfo<T::BlockNumber> =
			RoundInfo::new(1u32, 0u32.into(), T::DefaultBlocksPerRound::get());
		<Round<T>>::put(round);
		// Snapshot total stake
		<Staked<T>>::insert(1u32, <Total<T>>::get());
		let db_weight = T::DbWeight::get();
		db_weight.reads(5) + db_weight.writes(2) + 250_000_000_000
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		let candidates = <CandidatePool<T>>::get();
		let old_count = candidates.0.len() as u32;
		assert_eq!(old_count, 0);
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let candidates = <CandidatePool<T>>::get();
		let new_count = candidates.0.len() as u32;
		assert_eq!(new_count, 4);
		Ok(())
	}
}
