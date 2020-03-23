// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use codec::{Encode, Decode};
use node_primitives::{ClearingHandler, AssetIssue};
use frame_support::{
	Parameter, decl_module, decl_event, decl_storage, traits::Get
};
use sp_runtime::traits::{Member, AtLeast32Bit, One, Zero, SaturatedConversion, Saturating};

mod mock;
mod tests;

#[derive(Encode, Decode, Default, Eq, PartialEq, Debug, Clone, Copy)]
pub struct BalanceDuration<BlockNumber, Balance, Duration> {
	/// the block number recorded last time
	last_block: BlockNumber,
	/// the balance recorded last time
	last_balance: Balance,
	/// Duration of balance, value = balance * (last_block - start_block)
	value: Duration,
}

impl<BlockNumber, Balance, Duration> BalanceDuration<BlockNumber, Balance, Duration> where
	BlockNumber: Copy + AtLeast32Bit,
	Balance: Copy + AtLeast32Bit + From<BlockNumber>,
	Duration: Copy + AtLeast32Bit + From<Balance>,
{
	fn new<SettlementId, SettlementPeriod>(
		stl_id: SettlementId,
		last_block: BlockNumber,
		prev_amount: Balance,
		curr_amount: Balance,
	) -> Self
		where
			SettlementId: Copy + AtLeast32Bit,
			SettlementPeriod: Get<BlockNumber>,
	{
		let stl_index = stl_id.saturated_into::<u64>();
		let stl_period = SettlementPeriod::get().saturated_into::<u64>();
		let start_block: BlockNumber = (stl_index * stl_period).saturated_into::<BlockNumber>();
		let blocks: BlockNumber = last_block - start_block;
		let value: Duration = (prev_amount * blocks.into()).into();

		Self {
			last_block,
			last_balance: curr_amount,
			value,
		}
	}

	fn update(&mut self, last_block: BlockNumber, curr_amount: Balance) {
		let blocks = last_block - self.last_block;
		self.value += (self.last_balance * blocks.into()).into();
		self.last_block = last_block;
		self.last_balance = curr_amount;
	}
}

/// The module configuration trait.
pub trait Trait: system::Trait + brml_assets::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// Settlement id
	type SettlementId: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::BlockNumber>;

	/// How often (in blocks) new settlement are started.
	type SettlementPeriod: Get<Self::BlockNumber>;

	/// The value that represent the duration of balance.
	type Duration: Member + Parameter + AtLeast32Bit + Default + Copy + From<Self::Balance>;

	// Assets issue handler
	type AssetIssue: AssetIssue<Self::AssetId, Self::AccountId, Self::Balance>;
}

decl_event!(
	pub enum Event<T> where <T as Trait>::SettlementId,
	{
		/// New Settlement Started.
		NewSettlement(SettlementId),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Settlement {
		/// Records for asset clearing corresponding to an account id
		ClearingAssets get(fn clearing_assets): linked_map hasher(blake2_128_concat) (T::AssetId, T::AccountId, T::SettlementId)
			=> BalanceDuration<T::BlockNumber, T::Balance, T::Duration>;
		/// Records for token clearing corresponding to an asset id
		ClearingTokens get(fn clearing_tokens): linked_map hasher(blake2_128_concat) (T::AssetId, T::SettlementId)
			=> BalanceDuration<T::BlockNumber, T::Balance, T::Duration>;
		/// The next settlement identifier up for grabs.
		NextSettlementId get(fn next_settlement_id): T::SettlementId;
//		/// Records for settlements
//		Settlements get(settlements): map (T::AssetId, T::SettlementId)
////			=> Settlement<T::Balance, T::BlockNumber>;
//			=> SettlementStatus<T::Balance, T::BlockNumber>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		/// How often (in blocks) new settlement are started.
		const SettlementPeriod: T::BlockNumber = T::SettlementPeriod::get();

		fn deposit_event() = default;

		fn on_initialize(now_block: T::BlockNumber) {
			// check if need to begin a new settlement
			if (now_block % T::SettlementPeriod::get()).is_zero() {
				Self::new_settlement();
			}
		}

		fn on_finalize(now_block: T::BlockNumber) {
			Self::settlement(now_block);
		}
	}
}

impl<T: Trait> ClearingHandler<T::AssetId, T::AccountId, T::BlockNumber, T::Balance> for Module<T> {
	fn asset_clearing(
		asset_id: T::AssetId,
		target: T::AccountId,
		last_block: T::BlockNumber,
		prev_amount: T::Balance,
		curr_amount: T::Balance,
	) {
		let curr_stl_id: T::SettlementId = Self::current_settlement_id();

		let index = (asset_id, target.clone(), curr_stl_id);
		if <ClearingAssets<T>>::contains_key(&index) {
			<ClearingAssets<T>>::mutate(&index, |clearing_asset| {
				clearing_asset.update(last_block, curr_amount);
			});
		} else {
			let balance_duration = BalanceDuration::new::<_, T::SettlementPeriod>(
				curr_stl_id,
				last_block,
				prev_amount,
				curr_amount,
			);
			<ClearingAssets<T>>::insert(&index, balance_duration);
		}
	}

	fn token_clearing(
		asset_id: T::AssetId,
		last_block: T::BlockNumber,
		prev_amount: T::Balance,
		curr_amount: T::Balance,
	) {
		let curr_stl_id: T::SettlementId = Self::current_settlement_id();

		let index = (asset_id, curr_stl_id);
		if <ClearingTokens<T>>::contains_key(&index) {
			<ClearingTokens<T>>::mutate(&index, |clearing_token| {
				clearing_token.update(last_block, curr_amount);
			});
		} else {
			let balance_duration = BalanceDuration::new::<_, T::SettlementPeriod>(
				curr_stl_id,
				last_block,
				prev_amount,
				curr_amount,
			);
			<ClearingTokens<T>>::insert(&index, balance_duration);
		}
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {
	fn current_settlement_id() -> T::SettlementId {
		Self::next_settlement_id().saturating_sub(1.into())
	}

	fn new_settlement() {
		let new_stl_id: T::SettlementId = <NextSettlementId<T>>::mutate(|id| {
			*id += One::one();
			*id
		});

		Self::deposit_event(RawEvent::NewSettlement(new_stl_id));
	}

	fn settlement(_now_block: T::BlockNumber) {
		let curr_stl_id: T::SettlementId = Self::current_settlement_id();
		let stl_blocks = T::SettlementPeriod::get();

		// Update token's balance duration
		for ((asset_id, stl_id), _clearing_token) in <ClearingTokens<T>>::enumerate()
			.filter(|((_, stl_id), _)| *stl_id < curr_stl_id)
		{
			let last_block = stl_blocks * (stl_id + One::one()).into() - One::one();
			let token = <brml_assets::Tokens<T>>::get(asset_id);
			Self::token_clearing(asset_id, last_block, token.total_supply, token.total_supply);
		}

		for (index, _clearing_asset) in <ClearingAssets<T>>::enumerate()
			.filter(|((_, _, stl_id), _)| *stl_id < curr_stl_id)
		{
			let (asset_id, target, stl_id) = index.clone();

			// Calculate account balance duration
			let last_block = stl_blocks * (stl_id + One::one()).into() - One::one();
			let amount = <brml_assets::Balances<T>>::get((asset_id, target.clone()));
			Self::asset_clearing(asset_id, target.clone(), last_block, amount, amount);

			// Transfer to Balance, and remove clearing_asset record
			let clearing_asset = <ClearingAssets<T>>::take(&index);
			let amount = clearing_asset.last_balance;
			T::AssetIssue::asset_issue(asset_id, target.clone(), amount);
		}
	}
}
