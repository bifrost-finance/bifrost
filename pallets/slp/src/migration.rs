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

use crate::{
	primitives::{
		MoonbeamLedgerUpdateOperation, OneToManyDelegationAction, OneToManyDelegatorStatus,
		SubstrateLedgerUpdateOperation, UnlockChunk,
	},
	BalanceOf, Config, CurrencyId, DelegatorLatestTuneRecord, DelegatorLedgerXcmUpdateQueue,
	DelegatorLedgers, DelegatorsIndex2Multilocation, DelegatorsMultilocation2Index, FeeSources,
	Hash, HostingFees, SupplementFeeAccountWhitelist, Validators, ValidatorsByDelegator,
	ValidatorsByDelegatorXcmUpdateQueue, Weight, XcmDestWeightAndFee,
};
use codec::alloc::collections::BTreeMap;
use frame_support::{
	log, migration::storage_iter, pallet_prelude::*, traits::OnRuntimeUpgrade,
	ReversibleStorageHasher, StoragePrefixedMap,
};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::TimeUnit;
use sp_arithmetic::Permill;
use sp_std::{marker::PhantomData, vec::Vec};
use xcm::v3::prelude::*;

/// A type for accommodating validators by delegator update entries for different kinds of
/// currencies.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ValidatorsByDelegatorUpdateEntry<HashT> {
	/// A type for substrate validators by delegator updating entries
	Substrate(SubstrateValidatorsByDelegatorUpdateEntry<HashT>),
}

/// A type for substrate validators by delegator updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubstrateValidatorsByDelegatorUpdateEntry<HashT> {
	/// The currency id of the delegator that needs to be update
	pub currency_id: CurrencyId,
	/// The delegator id that needs to be update
	pub delegator_id: xcm::v2::MultiLocation,
	/// Validators vec to be updated
	pub validators: Vec<(xcm::v2::MultiLocation, HashT)>,
}

/// A type for accommodating delegator update entries for different kinds of currencies.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum LedgerUpdateEntry<Balance> {
	/// A type for substrate ledger updating entries
	Substrate(SubstrateLedgerUpdateEntry<Balance>),
	Moonbeam(MoonbeamLedgerUpdateEntry<Balance>),
	ParachainStaking(MoonbeamLedgerUpdateEntry<Balance>),
}

/// A type for substrate ledger updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubstrateLedgerUpdateEntry<Balance> {
	/// The currency id of the delegator that needs to be update
	pub currency_id: CurrencyId,
	/// The delegator id that needs to be update
	pub delegator_id: xcm::v2::MultiLocation,
	/// Update operation type
	pub update_operation: SubstrateLedgerUpdateOperation,
	/// The unlocking/bonding amount.
	#[codec(compact)]
	pub amount: Balance,
	/// If this entry is an unlocking entry, it should have unlock_time value. If it is a bonding
	/// entry, this field should be None. If it is a liquidize entry, this filed is the ongoing
	/// timeunit when the xcm message is sent.
	pub unlock_time: Option<TimeUnit>,
}

/// A type for Moonbeam ledger updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct MoonbeamLedgerUpdateEntry<Balance> {
	/// The currency id of the delegator that needs to be update
	pub currency_id: CurrencyId,
	/// The delegator id that needs to be update
	pub delegator_id: xcm::v2::MultiLocation,
	/// The validator id that needs to be update
	pub validator_id: Option<xcm::v2::MultiLocation>,
	/// Update operation type
	pub update_operation: MoonbeamLedgerUpdateOperation,
	#[codec(compact)]
	pub amount: Balance,
	/// If this entry is an unlocking entry, it should have unlock_time value. If it is a bonding
	/// entry, this field should be None. If it is a liquidize entry, this filed is the ongoing
	/// timeunit when the xcm message is sent.
	pub unlock_time: Option<TimeUnit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum Ledger<Balance> {
	Substrate(SubstrateLedger<Balance>),
	Moonbeam(OneToManyLedger<Balance>),
	ParachainStaking(OneToManyLedger<Balance>),
	Filecoin(FilecoinLedger<Balance>),
	Phala(PhalaLedger<Balance>),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubstrateLedger<Balance> {
	/// The delegator account Id
	pub account: xcm::v2::MultiLocation,
	/// The total amount of the delegator's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: Balance,
	/// The total amount of the delegator's balance that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out
	/// of the delegator (assuming it doesn't get slashed first).
	pub unlocking: Vec<UnlockChunk<Balance>>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct OneToManyLedger<Balance> {
	pub account: xcm::v2::MultiLocation,
	pub delegations: BTreeMap<xcm::v2::MultiLocation, Balance>,
	pub total: Balance,
	pub less_total: Balance,
	// request details.
	pub requests: Vec<OneToManyScheduledRequest<Balance>>,
	// fast check if request exists
	pub request_briefs: BTreeMap<xcm::v2::MultiLocation, (TimeUnit, Balance)>,
	pub status: OneToManyDelegatorStatus,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, PartialOrd, Ord)]
pub struct OneToManyScheduledRequest<Balance> {
	pub validator: xcm::v2::MultiLocation,
	pub when_executable: TimeUnit,
	pub action: OneToManyDelegationAction<Balance>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FilecoinLedger<Balance> {
	/// The delegator account Id
	pub account: xcm::v2::MultiLocation,
	// Initial pledge collateral for the miner
	#[codec(compact)]
	pub initial_pledge: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct PhalaLedger<Balance> {
	/// The delegator multilocation
	pub account: xcm::v2::MultiLocation,
	/// The total amount of the delegator's balance that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active_shares: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out
	/// of the delegator (assuming it doesn't get slashed first).
	#[codec(compact)]
	pub unlocking_shares: Balance,
	// The unlocking time unit
	pub unlocking_time_unit: Option<TimeUnit>,
	/// If the delegator is bonded, it should record the bonded pool id.
	pub bonded_pool_id: Option<u64>,
	/// If the delegator is bonded, it should record the bonded pool NFT collection id.
	pub bonded_pool_collection_id: Option<u32>,
}

/// Migrate MultiLocation v2 to v3
pub struct MigrateV2MultiLocationToV3<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrateV2MultiLocationToV3<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(
			"MigrateV2MultiLocationToV3::on_runtime_upgrade execute, will migrate from old MultiLocation(v1/v2) to v3",
		);

		let mut weight: Weight = Weight::zero();

		//migrate the value type of FeeSources
		FeeSources::<T>::translate(|_key, old_value: (xcm::v2::MultiLocation, BalanceOf<T>)| {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
			log::info!("FeeSources ====== old_value:{:?}", old_value);
			let new_multilocation: MultiLocation =
				old_value.0.try_into().expect("FeeSources ===== Stored xcm::v2::MultiLocation");
			log::info!("FeeSources ====== new_value:{:?}", new_multilocation);
			Some((new_multilocation, old_value.1))
		});

		//migrate the value type of HostingFees
		HostingFees::<T>::translate(|_key, old_value: (Permill, xcm::v2::MultiLocation)| {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
			log::info!("HostingFees ====== old_value:{:?}", old_value);
			let new_multilocation: MultiLocation =
				old_value.1.try_into().expect("HostingFees ===== Stored xcm::v2::MultiLocation");
			log::info!("HostingFees ====== new_value:{:?}", new_multilocation);
			Some((old_value.0, new_multilocation))
		});

		//migrate the value type of DelegatorsIndex2Multilocation
		DelegatorsIndex2Multilocation::<T>::translate(
			|_key1, _key2, old_value: xcm::v2::MultiLocation| {
				log::info!("DelegatorsIndex2Multilocation ====== old_value:{:?}", old_value);
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				let new_multilcation = MultiLocation::try_from(old_value)
					.expect("DelegatorsIndex2Multilocation ===== Stored xcm::v2::MultiLocation");
				log::info!("DelegatorsIndex2Multilocation ====== new_value:{:?}", new_multilcation);
				Some(new_multilcation)
			},
		);

		//migrate the value type of DelegatorsIndex2Multilocation
		XcmDestWeightAndFee::<T>::translate(
			|_key1, _key2, old_value: (xcm::v2::Weight, BalanceOf<T>)| {
				log::info!("XcmDestWeightAndFee ====== old_value:{:?}", old_value);
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				log::info!(
					"XcmDestWeightAndFee ====== new_value:{:?}",
					(Weight::from_ref_time(old_value.0), old_value.1)
				);
				Some((Weight::from_ref_time(old_value.0), old_value.1))
			},
		);

		//migrate the value type of Validators
		Validators::<T>::translate(|key, old_value: Vec<(xcm::v2::MultiLocation, Hash<T>)>| {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
			let mut new_value: Vec<(MultiLocation, Hash<T>)> = Vec::new();
			for i in old_value {
				if let CurrencyId::Token2(4) = key {
					log::info!("Validators ====== vFil does nothing ");
				} else {
					let new_multilcation =
						i.0.try_into().expect("Validators ====== Stored xcm::v2::MultiLocation");
					new_value.push((new_multilcation, i.1));
				}
			}
			Some(new_value)
		});

		//migrate the value type of Validators
		SupplementFeeAccountWhitelist::<T>::translate(
			|_key, old_value: Vec<(xcm::v2::MultiLocation, Hash<T>)>| {
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				let mut new_value: Vec<(MultiLocation, Hash<T>)> = Vec::new();
				for i in old_value {
					let new_multilcation =
						i.0.try_into().expect("Validators ====== Stored xcm::v2::MultiLocation");
					new_value.push((new_multilcation, i.1));
				}
				Some(new_value)
			},
		);

		//migrate the value type of ValidatorsByDelegatorXcmUpdateQueue
		ValidatorsByDelegatorXcmUpdateQueue::<T>::translate(
			|_key, old_value: (ValidatorsByDelegatorUpdateEntry<Hash<T>>, BlockNumberFor<T>)| {
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				log::info!("ValidatorsByDelegatorXcmUpdateQueue ====== old_value:{:?}", old_value);
				match old_value.0 {
					ValidatorsByDelegatorUpdateEntry::Substrate(d) => {
						let currency_id = d.currency_id;
						let new_delegator_id = d.delegator_id.try_into().expect("ValidatorsByDelegatorXcmUpdateQueue ====== Stored xcm::v2::MultiLocation");
						let mut new_validators: Vec<(MultiLocation, Hash<T>)> = Vec::new();
						for i in d.validators {
							let new_multilocation = i.0.try_into().expect("ValidatorsByDelegatorXcmUpdateQueue ====== Stored xcm::v2::MultiLocation");
							new_validators.push((new_multilocation, i.1));
						}
						let new_substrate_validators_by_delegator_update_entry =
							crate::primitives::SubstrateValidatorsByDelegatorUpdateEntry {
								currency_id,
								delegator_id: new_delegator_id,
								validators: new_validators,
							};
						let new_validators_by_delegator_update_entry =
							crate::ValidatorsByDelegatorUpdateEntry::Substrate(
								new_substrate_validators_by_delegator_update_entry,
							);
						log::info!(
							"ValidatorsByDelegatorXcmUpdateQueue ====== new_value:{:?}",
							(new_validators_by_delegator_update_entry.clone(), old_value.1)
						);
						Some((new_validators_by_delegator_update_entry, old_value.1))
					},
				};
				None
			},
		);

		//migrate the value type of DelegatorLedgerXcmUpdateQueue
		DelegatorLedgerXcmUpdateQueue::<T>::translate(
			|_key, old_value: (LedgerUpdateEntry<BalanceOf<T>>, BlockNumberFor<T>)| {
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				log::info!("DelegatorLedgerXcmUpdateQueue ====== old_value:{:?}", old_value);
				match old_value.0 {
					LedgerUpdateEntry::Substrate(s) => {
						let new_delegator_id = s.delegator_id.clone().try_into().expect(
							"DelegatorLedgerXcmUpdateQueue ====== Stored xcm::v2::MultiLocation",
						);
						let new_substrate_ledger_update_entry =
							crate::primitives::SubstrateLedgerUpdateEntry {
								currency_id: s.currency_id,
								delegator_id: new_delegator_id,
								update_operation: s.update_operation,
								amount: s.amount,
								unlock_time: s.unlock_time,
							};
						Some((
							crate::LedgerUpdateEntry::Substrate(new_substrate_ledger_update_entry),
							old_value.1,
						))
					},
					LedgerUpdateEntry::Moonbeam(m) => {
						let new_delegator_id: MultiLocation = m
							.delegator_id
							.clone()
							.try_into()
							.expect(
							"DelegatorLedgerXcmUpdateQueue ====== Stored xcm::v2::MultiLocation",
						);
						let mut new_validator_id: Option<MultiLocation> = None;
						if let Some(v) = m.validator_id {
							let new_multilocation = v.try_into().expect("DelegatorLedgerXcmUpdateQueue ====== Stored xcm::v2::MultiLocation");
							new_validator_id = Some(new_multilocation);
						}
						let new_moonbeam_ledger_update_entry =
							crate::primitives::MoonbeamLedgerUpdateEntry {
								currency_id: m.currency_id,
								delegator_id: new_delegator_id,
								validator_id: new_validator_id,
								update_operation: m.update_operation,
								amount: m.amount,
								unlock_time: m.unlock_time,
							};
						Some((
							crate::LedgerUpdateEntry::Moonbeam(new_moonbeam_ledger_update_entry),
							old_value.1,
						))
					},
					LedgerUpdateEntry::ParachainStaking(p) => {
						let new_delegator_id: MultiLocation = p
							.delegator_id
							.clone()
							.try_into()
							.expect(
							"DelegatorLedgerXcmUpdateQueue ====== Stored xcm::v2::MultiLocation",
						);
						let mut new_validator_id: Option<MultiLocation> = None;
						if let Some(v) = p.validator_id {
							let new_multilocation = v.try_into().expect("DelegatorLedgerXcmUpdateQueue ====== Stored xcm::v2::MultiLocation");
							new_validator_id = Some(new_multilocation);
						}
						let new_moonbeam_ledger_update_entry =
							crate::primitives::MoonbeamLedgerUpdateEntry {
								currency_id: p.currency_id,
								delegator_id: new_delegator_id,
								validator_id: new_validator_id,
								update_operation: p.update_operation,
								amount: p.amount,
								unlock_time: p.unlock_time,
							};
						Some((
							crate::LedgerUpdateEntry::ParachainStaking(
								new_moonbeam_ledger_update_entry,
							),
							old_value.1,
						))
					},
				}
			},
		);

		// // migrate the key type of DelegatorsMultilocation2Index
		let module_prefix = DelegatorsMultilocation2Index::<T>::module_prefix();
		let storage_prefix = DelegatorsMultilocation2Index::<T>::storage_prefix();
		let old_data =
			storage_iter::<u16>(module_prefix, storage_prefix).drain().collect::<Vec<_>>();

		for (raw_key, value) in old_data {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

			let mut k1_k2_material = Blake2_128Concat::reverse(&raw_key);
			let k1: CurrencyId = Decode::decode(&mut k1_k2_material)
				.expect("DelegatorsMultilocation2Index ===== Stored k1 CurrencyId");
			let mut k2_material = Blake2_128Concat::reverse(k1_k2_material);
			let k2: xcm::v2::MultiLocation = Decode::decode(&mut k2_material)
				.expect("DelegatorsMultilocation2Index ===== Stored k2 xcm::v2::MultiLocation");
			log::info!("DelegatorsMultilocation2Index ====== old_value:{:?}", k2);
			let new_k2: MultiLocation = k2
				.try_into()
				.expect("DelegatorsMultilocation2Index ===== Stored k2 xcm::v2::MultiLocation");
			log::info!("DelegatorsMultilocation2Index ====== new_value:{:?}", new_k2);
			DelegatorsMultilocation2Index::<T>::insert(k1, new_k2, value);
		}

		// migrate the key type of ValidatorsByDelegator
		let module_prefix = ValidatorsByDelegator::<T>::module_prefix();
		let storage_prefix = ValidatorsByDelegator::<T>::storage_prefix();
		let old_data =
			storage_iter::<Vec<(xcm::v2::MultiLocation, Hash<T>)>>(module_prefix, storage_prefix)
				.drain()
				.collect::<Vec<_>>();

		for (raw_key, old_value) in old_data {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

			let mut k1_k2_material = Blake2_128Concat::reverse(&raw_key);
			let k1: CurrencyId = Decode::decode(&mut k1_k2_material)
				.expect("ValidatorsByDelegator ===== Stored k1 CurrencyId");

			if let CurrencyId::Token2(4) = k1 {
				log::info!("ValidatorsByDelegator ====== vFil does nothing ");
			} else {
				let mut k2_material = Blake2_128Concat::reverse(k1_k2_material);
				let k2: xcm::v2::MultiLocation = Decode::decode(&mut k2_material)
					.expect("ValidatorsByDelegator ===== Stored k2 xcm::v2::MultiLocation");
				log::info!("ValidatorsByDelegator ====== old_value:{:?}", k2);
				let new_k2: MultiLocation = k2
					.try_into()
					.expect("ValidatorsByDelegator ===== Stored k2 xcm::v2::MultiLocation");
				log::info!("ValidatorsByDelegator ====== new_value:{:?}", new_k2);
				let mut new_value: Vec<(MultiLocation, Hash<T>)> = Vec::new();
				for i in old_value {
					let new_multilcation =
						i.0.try_into()
							.expect("ValidatorsByDelegator ====== Stored xcm::v2::MultiLocation");
					new_value.push((new_multilcation, i.1));
				}
				log::info!("ValidatorsByDelegator ====== new_value:{:?}", new_value);
				ValidatorsByDelegator::<T>::insert(k1, new_k2, new_value);
			}
		}

		// migrate the key type of DelegatorLatestTuneRecord
		let module_prefix = DelegatorLatestTuneRecord::<T>::module_prefix();
		let storage_prefix = DelegatorLatestTuneRecord::<T>::storage_prefix();
		let old_data = storage_iter::<TimeUnit>(module_prefix, storage_prefix)
			.drain()
			.collect::<Vec<_>>();

		for (raw_key, value) in old_data {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

			let mut k1_k2_material = Blake2_128Concat::reverse(&raw_key);
			let k1: CurrencyId = Decode::decode(&mut k1_k2_material).expect("Stored k1 CurrencyId");

			if let CurrencyId::Token2(4) = k1 {
				log::info!("DelegatorLatestTuneRecord ====== vFil does nothing ");
			} else {
				let mut k2_material = Blake2_128Concat::reverse(k1_k2_material);
				let k2: xcm::v2::MultiLocation = Decode::decode(&mut k2_material)
					.expect("DelegatorLatestTuneRecord ===== Stored k2 xcm::v2::MultiLocation");
				log::info!("DelegatorLatestTuneRecord ====== old_value:{:?}", k2);
				let new_k2: MultiLocation = k2
					.try_into()
					.expect("DelegatorLatestTuneRecord ===== Stored k2 xcm::v2::MultiLocation");
				log::info!("DelegatorLatestTuneRecord ====== new_value:{:?}", new_k2);
				DelegatorLatestTuneRecord::<T>::insert(k1, new_k2, value);
			}
		}

		// migrate the key type of DelegatorLedgers
		let module_prefix = DelegatorLedgers::<T>::module_prefix();
		let storage_prefix = DelegatorLedgers::<T>::storage_prefix();
		let old_data = storage_iter::<Ledger<BalanceOf<T>>>(module_prefix, storage_prefix)
			.drain()
			.collect::<Vec<_>>();

		for (raw_key, old_value) in old_data {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

			let mut k1_k2_material = Blake2_128Concat::reverse(&raw_key);
			let k1: CurrencyId = Decode::decode(&mut k1_k2_material).expect("Stored k1 CurrencyId");
			let mut k2_material = Blake2_128Concat::reverse(k1_k2_material);
			let k2: xcm::v2::MultiLocation = Decode::decode(&mut k2_material)
				.expect("DelegatorLedgers ===== Stored k2 xcm::v2::MultiLocation");
			let new_k2: MultiLocation =
				k2.try_into().expect("DelegatorLedgers ===== Stored k2 xcm::v2::MultiLocation");

			let new_ledger = match old_value {
				Ledger::Substrate(s) => {
					let new_account: MultiLocation = s
						.account
						.clone()
						.try_into()
						.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
					let new_substrate_ledger = crate::primitives::SubstrateLedger {
						account: new_account,
						total: s.total,
						active: s.active,
						unlocking: s.unlocking,
					};
					crate::Ledger::Substrate(new_substrate_ledger)
				},
				Ledger::Moonbeam(m) => {
					let new_account: MultiLocation = m
						.account
						.clone()
						.try_into()
						.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
					let mut new_delegations: BTreeMap<MultiLocation, BalanceOf<T>> =
						BTreeMap::new();
					for (x, y) in m.delegations {
						let new_multilocation: MultiLocation = x
							.try_into()
							.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
						new_delegations.insert(new_multilocation, y);
					}

					let mut new_requests: Vec<
						crate::primitives::OneToManyScheduledRequest<BalanceOf<T>>,
					> = Vec::new();
					for i in m.requests {
						let new_validator: MultiLocation = i
							.validator
							.try_into()
							.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
						let new_one_to_many_scheduled_request =
							crate::primitives::OneToManyScheduledRequest {
								validator: new_validator,
								when_executable: i.when_executable,
								action: i.action,
							};
						new_requests.push(new_one_to_many_scheduled_request);
					}

					let mut new_request_briefs: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<T>)> =
						BTreeMap::new();
					for (x, y) in m.request_briefs {
						let new_multilocation: MultiLocation = x
							.try_into()
							.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
						new_request_briefs.insert(new_multilocation, y);
					}

					let new_one_to_many_leder = crate::primitives::OneToManyLedger {
						account: new_account,
						delegations: new_delegations,
						total: m.total,
						less_total: m.less_total,
						requests: new_requests,
						request_briefs: new_request_briefs,
						status: m.status,
					};
					crate::Ledger::Moonbeam(new_one_to_many_leder)
				},
				Ledger::ParachainStaking(p) => {
					let new_account: MultiLocation = p
						.account
						.clone()
						.try_into()
						.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
					let mut new_delegations: BTreeMap<MultiLocation, BalanceOf<T>> =
						BTreeMap::new();
					for (x, y) in p.delegations {
						let new_multilocation: MultiLocation = x
							.try_into()
							.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
						new_delegations.insert(new_multilocation, y);
					}

					let mut new_requests: Vec<
						crate::primitives::OneToManyScheduledRequest<BalanceOf<T>>,
					> = Vec::new();
					for i in p.requests {
						let new_validator: MultiLocation = i
							.validator
							.try_into()
							.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
						let new_one_to_many_scheduled_request =
							crate::primitives::OneToManyScheduledRequest {
								validator: new_validator,
								when_executable: i.when_executable,
								action: i.action,
							};
						new_requests.push(new_one_to_many_scheduled_request);
					}

					let mut new_request_briefs: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<T>)> =
						BTreeMap::new();
					for (x, y) in p.request_briefs {
						let new_multilocation: MultiLocation = x
							.try_into()
							.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
						new_request_briefs.insert(new_multilocation, y);
					}

					let new_one_to_many_leder = crate::primitives::OneToManyLedger {
						account: new_account,
						delegations: new_delegations,
						total: p.total,
						less_total: p.less_total,
						requests: new_requests,
						request_briefs: new_request_briefs,
						status: p.status,
					};
					crate::Ledger::ParachainStaking(new_one_to_many_leder)
				},
				Ledger::Filecoin(f) => {
					let new_account: MultiLocation = f
						.account
						.clone()
						.try_into()
						.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
					let new_filecoin_ledger = crate::primitives::FilecoinLedger {
						account: new_account,
						initial_pledge: f.initial_pledge,
					};
					crate::Ledger::Filecoin(new_filecoin_ledger)
				},
				Ledger::Phala(p) => {
					let new_account: MultiLocation = p
						.account
						.clone()
						.try_into()
						.expect("DelegatorLedgers ====== Stored xcm::v2::MultiLocation");
					let new_phala_ledger = crate::primitives::PhalaLedger {
						account: new_account,
						active_shares: p.active_shares,
						unlocking_shares: p.unlocking_shares,
						unlocking_time_unit: p.unlocking_time_unit,
						bonded_pool_id: p.bonded_pool_id,
						bonded_pool_collection_id: p.bonded_pool_collection_id,
					};
					crate::Ledger::Phala(new_phala_ledger)
				},
			};
			DelegatorLedgers::<T>::insert(k1, new_k2, new_ledger);
		}
		weight
	}
}
