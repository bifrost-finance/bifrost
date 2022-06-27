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
use crate::RoundIndex;
use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use node_primitives::PoolId;
use sp_arithmetic::per_things::Permill;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
/// The current round index and transition information
pub struct RoundInfo<BlockNumber> {
	/// Current round index
	pub current: RoundIndex,
	/// The first block of the current round
	pub first: BlockNumber,
	/// The length of the current round in number of blocks
	pub length: u32,
}
impl<
		B: Copy + sp_std::ops::Add<Output = B> + sp_std::ops::Sub<Output = B> + From<u32> + PartialOrd,
	> RoundInfo<B>
{
	pub fn new(current: RoundIndex, first: B, length: u32) -> RoundInfo<B> {
		RoundInfo { current, first, length }
	}
	/// Check if the round should be updated
	pub fn should_update(&self, now: B) -> bool {
		now - self.first >= self.length.into()
	}
	/// New round
	pub fn update(&mut self, now: B) {
		self.current = self.current.saturating_add(1u32);
		self.first = now;
	}

	/// Check exec_delay match
	pub fn check_delay(&self, now: B, delay: u32) -> bool {
		now - self.first == delay.into()
	}
}
impl<
		B: Copy + sp_std::ops::Add<Output = B> + sp_std::ops::Sub<Output = B> + From<u32> + PartialOrd,
	> Default for RoundInfo<B>
{
	fn default() -> RoundInfo<B> {
		RoundInfo::new(1u32, 1u32.into(), 20u32)
	}
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TokenInfo<Balance: Copy> {
	pub system_stakable_amount: Balance,
	pub system_shadow_amount: Balance,
	pub pending_redeem_amount: Balance,
	// config params
	pub current_config: TokenConfig<Balance>,
	pub new_config: TokenConfig<Balance>,
}

impl<Balance: Zero + Copy> Default for TokenInfo<Balance> {
	fn default() -> TokenInfo<Balance> {
		TokenInfo {
			system_stakable_amount: Balance::zero(),
			system_shadow_amount: Balance::zero(),
			pending_redeem_amount: Balance::zero(),
			current_config: TokenConfig::<Balance>::default(),
			new_config: TokenConfig::<Balance>::default(),
		}
	}
}

impl<Balance: Copy + PartialEq> TokenInfo<Balance> {
	pub fn check_config_change(&self) -> bool {
		self.current_config != self.new_config
	}

	pub fn update_config(&mut self) {
		self.current_config = self.new_config.clone();
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct TokenConfig<Balance> {
	pub exec_delay: u32,
	pub system_stakable_farming_rate: Permill,
	pub lptoken_rates: Vec<Permill>,
	pub add_or_sub: bool, // true: add, false: sub
	pub system_stakable_base: Balance,
	pub farming_poolids: Vec<PoolId>,
}

impl<Balance: Zero> Default for TokenConfig<Balance> {
	fn default() -> TokenConfig<Balance> {
		TokenConfig {
			exec_delay: 0u32,
			system_stakable_farming_rate: Permill::from_percent(0),
			lptoken_rates: Vec::new(),
			system_stakable_base: Balance::zero(),
			add_or_sub: true, // default add
			farming_poolids: Vec::new(),
		}
	}
}
