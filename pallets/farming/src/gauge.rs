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

use codec::{FullCodec, HasCompact};
use frame_support::pallet_prelude::*;
use node_primitives::CurrencyId;
use orml_traits::RewardHandler;
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, Saturating, UniqueSaturatedInto,
		Zero,
	},
	FixedPointOperand, RuntimeDebug, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

use crate::*;

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct GaugePoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord> {
	pid: PoolId,
	token: CurrencyIdOf,
	gauge_amount: BalanceOf,
	gauge_state: GaugeState,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum GaugeState {
	Unbond,
	Bonded,
}

impl<BalanceOf, CurrencyIdOf> Default for GaugePoolInfo<BalanceOf, CurrencyIdOf>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord + Default,
{
	fn default() -> Self {
		Self {
			pid: Default::default(),
			token: Default::default(),
			gauge_amount: Default::default(),
			gauge_state: GaugeState::Unbond,
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn gauge_add(who: AccountIdOf<T>, pool: PoolId, gauge: PoolId) -> DispatchResult {
		SharesAndWithdrawnRewards::<T>::mutate(pool, who, |share_info| {});
		GaugePoolInfos::<T>::mutate(gauge, |gauge_info| {});
		Ok(())
	}

	pub fn gauge_cal_rewards(gauge_amount: BalanceOf<T>, gauge_last_block: BlockNumberFor<T>) -> DispatchResult {
		// SharesAndWithdrawnRewards::<T>::mutate(pool, who, |share_info| {});
		// GaugePoolInfos::<T>::mutate(gauge, |gauge_info| {});
		Ok(())
	}
}
