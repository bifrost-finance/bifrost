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

// use super::{Config, RelaychainLease, Weight};
use crate::*;
use codec::HasCompact;
use frame_support::traits::Get;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct DeprecatedPoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, AccountIdOf, BlockNumberFor>
{
	pub tokens_proportion: BTreeMap<CurrencyIdOf, Perbill>,
	/// Total shares amount
	pub total_shares: BalanceOf,
	pub basic_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
	/// Reward infos <reward_currency, (total_reward, total_withdrawn_reward)>
	pub rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
	pub state: PoolState,
	pub keeper: AccountIdOf,
	pub reward_issuer: AccountIdOf,
	/// Gauge pool id
	pub gauge: Option<PoolId>,
	pub block_startup: Option<BlockNumberFor>,
	pub min_deposit_to_start: BalanceOf,
	pub after_block_to_start: BlockNumberFor,
	pub withdraw_limit_time: BlockNumberFor,
	pub claim_limit_time: BlockNumberFor,
	pub withdraw_limit_count: u8,
}

#[allow(dead_code)]
pub fn update_pool_info<T: Config>() -> Weight {
	let _ = PoolInfos::<T>::translate::<
		DeprecatedPoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>,
		_,
	>(
		|_key,
		 pool|
		 -> Option<PoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>> {
			// if let Some(pool) = pool_info {
			let a = pool.tokens_proportion.keys().cloned().collect::<Vec<CurrencyIdOf<T>>>()[0];
			let b = pool.tokens_proportion.values().cloned().collect::<Vec<Perbill>>()[0];
			let new_entry =
				PoolInfo::<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>> {
					tokens_proportion: pool.tokens_proportion,
					basic_token: (a, b),
					total_shares: pool.total_shares,
					basic_rewards: pool.basic_rewards,
					rewards: pool.rewards,
					state: pool.state,
					keeper: pool.keeper,
					reward_issuer: pool.reward_issuer,
					gauge: pool.gauge,
					block_startup: pool.block_startup,
					min_deposit_to_start: pool.min_deposit_to_start,
					after_block_to_start: pool.after_block_to_start,
					withdraw_limit_time: pool.withdraw_limit_time,
					claim_limit_time: pool.claim_limit_time,
					withdraw_limit_count: pool.withdraw_limit_count,
				};
			Some(new_entry)
			// } else {
			// 	None
			// }
		},
	);

	T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1)
}
