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

use super::{Config, MinimumsAndMaximums, Weight};
use crate::{BalanceOf, Decode, Encode, MinimumsMaximums, RuntimeDebug, TimeUnit, TypeInfo};
use frame_support::traits::Get;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct DeprecatedMinimumsMaximums<Balance> {
	/// The minimum bonded amount for a delegator at any time.
	#[codec(compact)]
	pub delegator_bonded_minimum: Balance,
	/// The minimum amount each time a delegator needs to bond for extra
	#[codec(compact)]
	pub bond_extra_minimum: Balance,
	/// The minimum unbond amount each time a delegator to unbond.
	#[codec(compact)]
	pub unbond_minimum: Balance,
	/// The minimum amount each time a delegator needs to rebond
	#[codec(compact)]
	pub rebond_minimum: Balance,
	/// The maximum number of unbond records at the same time.
	#[codec(compact)]
	pub unbond_record_maximum: u32,
	/// The maximum number of validators for a delegator to support at the same time.
	#[codec(compact)]
	pub validators_back_maximum: u32,
	/// The maximum amount of active staking for a delegator. It is used to control ROI.
	#[codec(compact)]
	pub delegator_active_staking_maximum: Balance,
	/// The maximum number of delegators for a validator to reward.
	#[codec(compact)]
	pub validators_reward_maximum: u32,
	/// The minimum amount for a delegation.
	#[codec(compact)]
	pub delegation_amount_minimum: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct DeprecatedDelays {
	/// The unlock delay for the unlocking amount to be able to be liquidized.
	pub unlock_delay: TimeUnit,
}

pub fn update_minimums_maximums<T: Config>() -> Weight {
	MinimumsAndMaximums::<T>::translate::<DeprecatedMinimumsMaximums<BalanceOf<T>>, _>(
		|_currency_id, mins_maxs| {
			let new_entry = MinimumsMaximums::<BalanceOf<T>> {
				delegator_bonded_minimum: mins_maxs.delegator_bonded_minimum,
				bond_extra_minimum: mins_maxs.bond_extra_minimum,
				unbond_minimum: mins_maxs.unbond_minimum,
				rebond_minimum: mins_maxs.rebond_minimum,
				unbond_record_maximum: mins_maxs.unbond_record_maximum,
				validators_back_maximum: mins_maxs.validators_back_maximum,
				delegator_active_staking_maximum: mins_maxs.delegator_active_staking_maximum,
				validators_reward_maximum: mins_maxs.validators_reward_maximum,
				delegation_amount_minimum: mins_maxs.delegation_amount_minimum,
				delegators_maximum: 100u16,
				validators_maximum: 300u16,
			};
			Some(new_entry)
		},
	);

	T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1)
}
