// Copyright 2019-2022 PureStake Inc.
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

#![allow(unused)]

use frame_support::{
	migration::storage_key_iter,
	pallet_prelude::PhantomData,
	traits::{Get, OnRuntimeUpgrade, ReservableCurrency},
	weights::Weight,
	Twox64Concat,
};
#[cfg(feature = "try-runtime")]
use parity_scale_codec::{Decode, Encode};
extern crate alloc;
#[cfg(feature = "try-runtime")]
use alloc::{format, string::ToString};

#[cfg(feature = "try-runtime")]
use scale_info::prelude::string::String;
use sp_runtime::{
	traits::{AccountIdConversion, Saturating, Zero},
	Perbill,
};
use sp_std::{convert::TryInto, vec::Vec};

#[allow(deprecated)]
use crate::types::deprecated::{DelegationChange, Delegator as OldDelegator};
use crate::{
	delegation_requests::{DelegationAction, ScheduledRequest},
	inflation::{perbill_annual_to_perbill_round, InflationInfo, BLOCKS_PER_YEAR},
	pallet::{DelegationScheduledRequests, DelegatorState, Total},
	types::Delegator,
	AccountIdOf, BalanceOf, Bond, BottomDelegations, CandidateInfo, CandidateMetadata,
	CandidatePool, CapacityStatus, CollatorCandidate, CollatorCommission, Config, Delegations,
	Event, InflationConfig, Pallet, ParachainBondConfig, ParachainBondInfo, Points, Range, Round,
	RoundInfo, Staked, TopDelegations, TotalSelected,
};

/// Migration to purge staking storage bloat for `Points` and `AtStake` storage items
pub struct InitGenesisMigration<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for InitGenesisMigration<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "Staking", "init migraion");
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
		let expected: BalanceOf<T> = T::PaymentInRound::get();

		let inflation_info: InflationInfo<BalanceOf<T>> = InflationInfo {
			// staking expectations
			expect: Range { min: expected, ideal: expected, max: expected },
			// annual inflation
			annual,
			round: to_round_inflation(annual),
		};
		<InflationConfig<T>>::put(inflation_info);
		let endowment: BalanceOf<T> = T::InitSeedStk::get();

		let mut candidate_count = 0u32;

		for candidate in &T::ToMigrateInvulnables::get() {
			candidate_count += 1u32;
			if let Err(error) = <Pallet<T>>::join_candidates(
				T::RuntimeOrigin::from(Some(candidate.clone()).into()),
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
			account: T::PalletId::get().into_account_truncating(),
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
		db_weight.reads(5) + db_weight.writes(2) + Weight::from_ref_time(250_000_000_000 as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		log::info!(target: "Staking", "pre-init migraion");
		let candidates = <CandidatePool<T>>::get();
		let old_count = candidates.0.len() as u32;
		assert_eq!(old_count, 0);
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		log::info!(target: "Staking", "post-init migraion");
		let candidates = <CandidatePool<T>>::get();
		let new_count = candidates.0.len();
		assert_eq!(new_count, T::ToMigrateInvulnables::get().len());
		Ok(())
	}
}

/// Migration to move delegator requests towards a delegation, from [DelegatorState] into
/// [DelegationScheduledRequests] storage item.
/// Additionally [DelegatorState] is migrated from [OldDelegator] to [Delegator].
pub struct SplitDelegatorStateIntoDelegationScheduledRequests<T>(PhantomData<T>);
impl<T: Config> SplitDelegatorStateIntoDelegationScheduledRequests<T> {
	const PALLET_PREFIX: &'static [u8] = b"ParachainStaking";
	const DELEGATOR_STATE_PREFIX: &'static [u8] = b"DelegatorState";

	#[allow(deprecated)]
	#[cfg(feature = "try-runtime")]
	fn old_request_to_string(
		delegator: &AccountIdOf<T>,
		request: &crate::deprecated::DelegationRequest<AccountIdOf<T>, BalanceOf<T>>,
	) -> String {
		match request.action {
			DelegationChange::Revoke => {
				format!(
					"delegator({:?})_when({})_Revoke({:?})",
					delegator, request.when_executable, request.amount
				)
			},
			DelegationChange::Decrease => {
				format!(
					"delegator({:?})_when({})_Decrease({:?})",
					delegator, request.when_executable, request.amount
				)
			},
		}
	}

	#[cfg(feature = "try-runtime")]
	fn new_request_to_string(request: &ScheduledRequest<AccountIdOf<T>, BalanceOf<T>>) -> String {
		match request.action {
			DelegationAction::Revoke(v) => {
				format!(
					"delegator({:?})_when({})_Revoke({:?})",
					request.delegator, request.when_executable, v
				)
			},
			DelegationAction::Decrease(v) => {
				format!(
					"delegator({:?})_when({})_Decrease({:?})",
					request.delegator, request.when_executable, v
				)
			},
		}
	}
}

#[allow(deprecated)]
impl<T: Config> OnRuntimeUpgrade for SplitDelegatorStateIntoDelegationScheduledRequests<T> {
	fn on_runtime_upgrade() -> Weight {
		use sp_std::collections::btree_map::BTreeMap;

		log::info!(
			target: "SplitDelegatorStateIntoDelegationScheduledRequests",
			"running migration for DelegatorState to new version and DelegationScheduledRequests \
			storage item"
		);

		let mut reads: u64 = 0;
		let mut writes: u64 = 0;

		let mut scheduled_requests: BTreeMap<
			AccountIdOf<T>,
			Vec<ScheduledRequest<AccountIdOf<T>, BalanceOf<T>>>,
		> = BTreeMap::new();
		<DelegatorState<T>>::translate(
			|delegator, old_state: OldDelegator<AccountIdOf<T>, BalanceOf<T>>| {
				reads = reads.saturating_add(1u64);
				writes = writes.saturating_add(1u64);

				for (collator, request) in old_state.requests.requests.into_iter() {
					let action = match request.action {
						DelegationChange::Revoke => DelegationAction::Revoke(request.amount),
						DelegationChange::Decrease => DelegationAction::Decrease(request.amount),
					};
					let entry = scheduled_requests.entry(collator.clone()).or_default();
					entry.push(ScheduledRequest {
						delegator: delegator.clone(),
						when_executable: request.when_executable,
						action,
					});
				}

				let new_state = Delegator {
					id: old_state.id,
					delegations: old_state.delegations,
					total: old_state.total,
					less_total: old_state.requests.less_total,
					status: old_state.status,
				};

				Some(new_state)
			},
		);

		writes = writes.saturating_add(scheduled_requests.len() as u64); // 1 write per request
		for (collator, requests) in scheduled_requests {
			<DelegationScheduledRequests<T>>::insert(collator, requests);
		}

		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		use sp_std::collections::btree_map::BTreeMap;

		let mut expected_delegator_state_entries = 0u64;
		let mut expected_requests = 0u64;
		let mut delegator_state_map: BTreeMap<String, BalanceOf<T>> = BTreeMap::new();

		let mut collator_state_map: BTreeMap<String, String> = BTreeMap::new();
		for (_key, state) in migration::storage_iter::<OldDelegator<AccountIdOf<T>, BalanceOf<T>>>(
			Self::PALLET_PREFIX,
			Self::DELEGATOR_STATE_PREFIX,
		) {
			log::info!(
				target: "SplitDelegatorStateIntoDelegationScheduledRequests",
				"delegator: {:?}, less total: {:?}. fmt: {:?}",
				state.id, state.requests.less_total, &*format!("expected_delegator-{:?}_decrease_amount", state.id,),
			);
			delegator_state_map.insert(
				(&*format!("expected_delegator-{:?}_decrease_amount", state.id)).to_string(),
				state.requests.less_total,
			);

			for (collator, request) in state.requests.requests.iter() {
				collator_state_map.insert(
					(&*format!(
						"expected_collator-{:?}_delegator-{:?}_request",
						collator, state.id,
					))
						.to_string(),
					Self::old_request_to_string(&state.id, &request),
				);
			}
			expected_delegator_state_entries =
				expected_delegator_state_entries.saturating_add(1 as u64);
			expected_requests =
				expected_requests.saturating_add(state.requests.requests.len() as u64);
		}

		use frame_support::migration;

		Ok((
			delegator_state_map,
			collator_state_map,
			expected_delegator_state_entries,
			expected_requests,
		)
			.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
		use sp_std::collections::btree_map::BTreeMap;

		let (
			delegator_state_map,
			collator_state_map,
			expected_delegator_state_entries,
			expected_requests,
		): (BTreeMap<String, BalanceOf<T>>, BTreeMap<String, String>, u64, u64) =
			Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");
		// Scheduled decrease amount (bond_less) is correctly migrated
		let mut actual_delegator_state_entries = 0;
		for (delegator, state) in <DelegatorState<T>>::iter() {
			let expected_delegator_decrease_amount: BalanceOf<T> = delegator_state_map
				.get(&(&*format!("expected_delegator-{:?}_decrease_amount", state.id)).to_string())
				.expect("must exist")
				.clone();
			assert_eq!(
				expected_delegator_decrease_amount, state.less_total,
				"decrease amount did not match for delegator {:?}",
				delegator,
			);
			actual_delegator_state_entries = actual_delegator_state_entries.saturating_add(1);
		}

		assert_eq!(
			expected_delegator_state_entries, actual_delegator_state_entries,
			"unexpected change in the number of DelegatorState entries"
		);

		// Scheduled requests are correctly migrated
		let mut actual_requests = 0u64;
		for (collator, scheduled_requests) in <DelegationScheduledRequests<T>>::iter() {
			for request in scheduled_requests {
				let expected_delegator_request: String = collator_state_map
					.get(
						&*format!(
							"expected_collator-{:?}_delegator-{:?}_request",
							collator, request.delegator,
						)
						.to_string(),
					)
					.expect("must exist")
					.clone();
				let actual_delegator_request = Self::new_request_to_string(&request);
				assert_eq!(
					expected_delegator_request, actual_delegator_request,
					"scheduled request did not match for collator {:?}, delegator {:?}",
					collator, request.delegator,
				);

				actual_requests = actual_requests.saturating_add(1);
			}
		}

		assert_eq!(
			expected_requests, actual_requests,
			"number of scheduled request entries did not match",
		);

		Ok(())
	}
}

/// Migration to patch the incorrect delegations sums for all candidates
pub struct PatchIncorrectDelegationSums<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for PatchIncorrectDelegationSums<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(
			target: "PatchIncorrectDelegationSums",
			"running migration to patch incorrect delegation sums"
		);
		let pallet_prefix: &[u8] = b"ParachainStaking";
		let top_delegations_prefix: &[u8] = b"TopDelegations";
		let bottom_delegations_prefix: &[u8] = b"BottomDelegations";
		// Read all the data into memory.
		// https://crates.parity.io/frame_support/storage/migration/fn.storage_key_iter.html
		let stored_top_delegations: Vec<_> = storage_key_iter::<
			AccountIdOf<T>,
			Delegations<AccountIdOf<T>, BalanceOf<T>>,
			Twox64Concat,
		>(pallet_prefix, top_delegations_prefix)
		.collect();
		let migrated_candidates_top_count = stored_top_delegations.len() as u64;
		let stored_bottom_delegations: Vec<_> = storage_key_iter::<
			AccountIdOf<T>,
			Delegations<AccountIdOf<T>, BalanceOf<T>>,
			Twox64Concat,
		>(pallet_prefix, bottom_delegations_prefix)
		.collect();
		let migrated_candidates_bottom_count = stored_bottom_delegations.len() as u64;
		fn fix_delegations<T: Config>(
			delegations: Delegations<AccountIdOf<T>, BalanceOf<T>>,
		) -> Delegations<AccountIdOf<T>, BalanceOf<T>> {
			let correct_total = delegations
				.delegations
				.iter()
				.fold(BalanceOf::<T>::zero(), |acc, b| acc + b.amount);
			log::info!(
				target: "PatchIncorrectDelegationSums",
				"Correcting total from {:?} to {:?}",
				delegations.total, correct_total
			);
			Delegations { delegations: delegations.delegations, total: correct_total }
		}
		for (account, old_top_delegations) in stored_top_delegations {
			let new_top_delegations = fix_delegations::<T>(old_top_delegations);
			let mut candidate_info = <CandidateInfo<T>>::get(&account)
				.expect("TopDelegations exists => CandidateInfo exists");
			candidate_info.total_counted = candidate_info.bond + new_top_delegations.total;
			if candidate_info.is_active() {
				Pallet::<T>::update_active(account.clone(), candidate_info.total_counted);
			}
			<CandidateInfo<T>>::insert(&account, candidate_info);
			<TopDelegations<T>>::insert(&account, new_top_delegations);
		}
		for (account, old_bottom_delegations) in stored_bottom_delegations {
			let new_bottom_delegations = fix_delegations::<T>(old_bottom_delegations);
			<BottomDelegations<T>>::insert(&account, new_bottom_delegations);
		}
		let weight = T::DbWeight::get();
		let top = migrated_candidates_top_count.saturating_mul(3 * weight.write + 3 * weight.read);
		let bottom = migrated_candidates_bottom_count.saturating_mul(weight.write + weight.read);
		// 20% max block weight as margin for error
		Weight::from_ref_time(top.saturating_add(bottom).saturating_add(100_000_000_000))
	}
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		use sp_std::collections::btree_map::BTreeMap;

		let mut candidate_total_counted_map: BTreeMap<String, BalanceOf<T>> = BTreeMap::new();
		// get total counted for all candidates
		for (account, state) in <CandidateInfo<T>>::iter() {
			candidate_total_counted_map.insert(
				(&format!("Candidate{:?}TotalCounted", account)[..]).to_string(),
				state.total_counted,
			);
		}
		Ok(candidate_total_counted_map.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
		use sp_std::collections::btree_map::BTreeMap;

		let candidate_total_counted_map: BTreeMap<String, BalanceOf<T>> =
			Decode::decode(&mut &state[..]).expect("pre_upgrade provides a valid state; qed");
		// ensure new total counted = top_delegations.sum() + collator self bond
		for (account, state) in <CandidateInfo<T>>::iter() {
			let old_count = candidate_total_counted_map
				.get(&(&format!("Candidate{:?}TotalCounted", account)[..]).to_string())
				.expect("qed")
				.clone();
			let new_count = state.total_counted;
			let top_delegations_sum = <TopDelegations<T>>::get(account)
				.expect("CandidateInfo exists => TopDelegations exists")
				.delegations
				.iter()
				.fold(BalanceOf::<T>::zero(), |acc, b| acc + b.amount);
			let correct_total_counted = top_delegations_sum + state.bond;
			assert_eq!(new_count, correct_total_counted);
			if new_count != old_count {
				log::info!(
					target: "PatchIncorrectDelegationSums",
					"Corrected total from {:?} to {:?}",
					old_count, new_count
				);
			}
		}
		Ok(())
	}
}

/// Migration to purge staking storage bloat for `Points` and `AtStake` storage items
pub struct PurgeStaleStorage<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for PurgeStaleStorage<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: "PurgeStaleStorage", "running migration to remove storage bloat");
		let current_round = <Round<T>>::get().current;
		let payment_delay = T::RewardPaymentDelay::get();
		let db_weight = T::DbWeight::get();
		let (reads, mut writes) = (3u64, 0u64);
		if current_round <= payment_delay {
			// early enough so no storage bloat exists yet
			// (only relevant for chains <= payment_delay rounds old)
			return db_weight.reads(reads);
		}
		// already paid out at the beginning of current round
		let most_recent_round_to_kill = current_round - payment_delay;
		for i in 1..=most_recent_round_to_kill {
			writes = writes.saturating_add(2u64);
			<Staked<T>>::remove(i);
			<Points<T>>::remove(i);
		}
		// 5% of the max block weight as safety margin for computation
		Weight::from_ref_time(25_000_000_000).saturating_add(db_weight.reads_writes(reads, writes))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		// trivial migration
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		// expect only the storage items for the last 2 rounds to be stored
		let staked_count = Staked::<T>::iter().count() as u32;
		let points_count = Points::<T>::iter().count() as u32;
		let delay = T::RewardPaymentDelay::get();
		assert_eq!(
			staked_count, delay,
			"Expected {} for `Staked` count, Found: {}",
			delay, staked_count
		);
		assert_eq!(
			points_count, delay,
			"Expected {} for `Points` count, Found: {}",
			delay, staked_count
		);
		Ok(())
	}
}
