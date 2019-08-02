// Copyright 2019 Liebi Technologies.
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

use rstd::prelude::*;
use parity_codec::{Encode, Decode};
use srml_support::{StorageValue, StorageMap, Parameter, decl_module, decl_event, decl_storage, ensure};
use sr_primitives::traits::{Member, SimpleArithmetic, One, Zero, StaticLookup};
use system::{ensure_signed, ensure_root};

mod mock;
mod tests;

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct Token<Balance> {
	symbol: Vec<u8>,
	precision: u16,
	total_supply: Balance,
}

#[derive(Encode, Decode, Default, Eq, PartialEq, Debug, Clone, Copy)]
pub struct BalanceDuration<BlockNumber, SettlementId, Duration> {
	/// current settlement index
	index: SettlementId,
	/// the block number recorded last time
	last_calculate_block: BlockNumber,
	/// Duration of balance, value = balance * (now_block - last_calculate_block)
	value: Duration,
}

/// The module configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy + Zero + From<Self::BlockNumber>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// Settlement id
	type SettlementId: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// The value that represent the duration of balance.
	type Duration: Member + Parameter + SimpleArithmetic + Default + Copy + From<Self::Balance>;
}

decl_event!(
	pub enum Event<T>
		where <T as system::Trait>::AccountId,
			<T as Trait>::Balance,
			<T as Trait>::AssetId,
	{
		/// Some assets were created.
		Created(AssetId, Token<Balance>),
		/// Some assets were issued.
		Issued(AssetId, AccountId, Balance),
		/// Some assets were transferred.
		Transferred(AssetId, AccountId, AccountId, Balance),
		/// Some assets were destroyed.
		Destroyed(AssetId, AccountId, Balance),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Assets {
		/// The number of units of assets held by any given asset ans given account.
		Balances get(balances): map (T::AssetId, T::AccountId) => T::Balance;
		/// The next asset identifier up for grabs.
		NextAssetId get(next_asset_id): T::AssetId;
		/// Details of the token corresponding to an asset id.
		Tokens get(token_details): map T::AssetId => Token<T::Balance>;
		/// Records for asset clearing corresponding to an account id
		ClearingAssets get(clearing_assets): map (T::AssetId, T::AccountId, T::SettlementId)
			=> BalanceDuration<T::BlockNumber, T::SettlementId, T::Duration>;
		/// Records for token clearing corresponding to an asset id
		ClearingTokens get(clearing_tokens): map (T::AssetId, T::SettlementId)
			=> BalanceDuration<T::BlockNumber, T::SettlementId, T::Duration>;
		/// The next settlement identifier up for grabs.
		NextSettlementId get(next_settlement_id): T::SettlementId;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		/// Create a new class of fungible assets. It will have an
		/// identifier `AssetId` instance: this will be specified in the `Created` event.
		fn create(origin, symbol: Vec<u8>, precision: u16) {
			let _origin = ensure_root(origin)?;

			ensure!(symbol.len() <= 32, "token symbol cannot exceed 32 bytes");
			ensure!(precision <= 16, "token precision cannot exceed 16");

			let id = Self::next_asset_id();
			<NextAssetId<T>>::mutate(|id| *id += One::one());

			// Initial total supply is zero.
			let total_supply: T::Balance = 0.into();

			let token = Token {
				symbol: symbol.clone(),
				precision: precision.clone(),
				total_supply: total_supply,
			};

			<Tokens<T>>::insert(id, token.clone());

			Self::deposit_event(RawEvent::Created(id, token));
		}

		/// Issue any amount of fungible assets.
		fn issue(origin,
			#[compact] id: T::AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance)
		{
			let _origin = ensure_root(origin)?;

			ensure!(<Tokens<T>>::exists(&id), "asset should be created first");

			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), "issue amount should be non-zero");

			Self::asset_issue(id, target.clone(), amount);

			Self::deposit_event(RawEvent::Issued(id, target, amount));
		}

		/// Move some assets from one holder to another.
		fn transfer(origin,
			#[compact] id: T::AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance)
		{
			let origin = ensure_signed(origin)?;
			let origin_account = (id, origin.clone());
			let origin_balance = <Balances<T>>::get(&origin_account);
			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), "transfer amount should be non-zero");
			ensure!(origin_balance >= amount,
				"origin account balance must be greater than or equal to the transfer amount");

			Self::asset_transfer(id, origin.clone(), target.clone(), amount);

			Self::deposit_event(RawEvent::Transferred(id, origin, target, amount));
		}

		/// Destroy any amount of assets of `id` owned by `origin`.
		fn destroy(origin, #[compact] id: T::AssetId, #[compact] amount: T::Balance) {
			let origin = ensure_signed(origin)?;
			let origin_account = (id, origin.clone());

			let balance = <Balances<T>>::get(&origin_account);
			ensure!(amount <= balance , "amount should be less than or equal to origin balance");

			Self::asset_destroy(id, origin.clone(), amount);

			Self::deposit_event(RawEvent::Destroyed(id, origin, amount));
		}
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {
	fn asset_issue(asset_id: T::AssetId, target: T::AccountId, amount: T::Balance) {
		let now_block: T::BlockNumber = <system::Module<T>>::block_number();
		let curr_stl_id: T::SettlementId = Self::next_settlement_id();

		Self::asset_clearing(asset_id, target.clone(), curr_stl_id, now_block, amount);
		<Balances<T>>::mutate((asset_id, target), |balance| *balance += amount);

		Self::token_clearing(asset_id, curr_stl_id, now_block, amount);
		<Tokens<T>>::mutate(asset_id, |token| token.total_supply += amount);
	}

	fn asset_transfer(asset_id: T::AssetId, from: T::AccountId, to: T::AccountId, amount: T::Balance) {
		let now_block: T::BlockNumber = <system::Module<T>>::block_number();
		let curr_stl_id: T::SettlementId = Self::next_settlement_id();

		Self::asset_clearing(asset_id, from.clone(), curr_stl_id, now_block, amount);
		<Balances<T>>::mutate((asset_id, from), |balance| *balance -= amount);

		Self::asset_clearing(asset_id, to.clone(), curr_stl_id, now_block, amount);
		<Balances<T>>::mutate((asset_id, to), |balance| *balance += amount);
	}

	fn asset_destroy(asset_id: T::AssetId, target: T::AccountId, amount: T::Balance) {
		let now_block: T::BlockNumber = <system::Module<T>>::block_number();
		let curr_stl_id: T::SettlementId = Self::next_settlement_id();

		Self::asset_clearing(asset_id, target.clone(), curr_stl_id, now_block, amount);
		<Balances<T>>::mutate((asset_id, target), |balance| *balance -= amount);

		Self::token_clearing(asset_id, curr_stl_id, now_block, amount);
		<Tokens<T>>::mutate(asset_id, |token| token.total_supply -= amount);
	}

	fn asset_clearing(asset_id: T::AssetId, target: T::AccountId, curr_stl_id: T::SettlementId,
		now_block: T::BlockNumber, amount: T::Balance)
	{
		<ClearingAssets<T>>::mutate((asset_id, target, curr_stl_id), |clearing_asset| {
			let blocks = now_block - clearing_asset.last_calculate_block;
			clearing_asset.value += (amount * blocks.into()).into();
		});
	}

	fn token_clearing(asset_id: T::AssetId, curr_stl_id: T::SettlementId,
		now_block: T::BlockNumber, amount: T::Balance)
	{
		<ClearingTokens<T>>::mutate((asset_id, curr_stl_id), |clearing_token| {
			let blocks = now_block - clearing_token.last_calculate_block;
			clearing_token.value += (amount * blocks.into()).into();
		});
	}
}
