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

use codec::{Decode, Encode};
use frame_support::{decl_event, decl_module, decl_storage, ensure, Parameter};
use sp_runtime::traits::{Member, SimpleArithmetic, SaturatedConversion};
use sp_std::prelude::*;
use system::ensure_root;

use node_primitives::{
	AssetCreate, AssetIssue, AssetRedeem, BlockchainType,
	BridgeAssetBalance, BridgeAssetFrom, BridgeAssetSymbol, BridgeAssetTo
};

mod mock;
mod tests;

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct Bank {
	account: Vec<u8>,
	authorities: Vec<Vec<u8>>,
	threshold: u32,
}

/// The module configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// Assets create handler
	type AssetCreate: AssetCreate<Self::AssetId, Self::Balance>;

	/// Assets issue handler
	type AssetIssue: AssetIssue<Self::AssetId, Self::AccountId, Self::Balance>;

	/// The units in which we record asset precision.
	type Precision: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// Bridge asset to another blockchain.
	type BridgeAssetTo: BridgeAssetTo<Self::Precision, Self::Balance>;
}

decl_event!(
	pub enum Event<T> where <T as Trait>::AssetId {
		/// Transaction from another blockchain was mapped.
		BridgeTxMapped,
		/// Transaction from another blockchain was received.
		BridgeTxReceived,
		/// Transaction received from another blockchain was confirmed.
		BridgeTxReceiveConfirmed,
		/// Transaction to another blockchain was sent.
		BridgeTxSent,
		/// Bridge asset was created.
		BridgeAssetCreated(AssetId),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Bridge {
		// Associate account id in Bifrost to account in other blockchain
		BridgeAccountIdToAccount get(fn bridge_account): map T::AccountId => Vec<u8>;

		// Associate asset id in Bifrost to asset symbol in other blockchain
		BridgeAssetIdToAsset get(fn bridge_asset): map T::AssetId => BridgeAssetSymbol<T::Precision>;

		// Associate account in other blockchain to account id in Bifrost
		BridgeAccountToAccountId get(fn bridge_account_id): map Vec<u8> => T::AccountId ;

		// Associate asset symbol in other blockchain to asset id in Bifrost
		BridgeAssetToAssetId get(fn bridge_asset_id): map BridgeAssetSymbol<T::Precision> => T::AssetId;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event() = default;

		pub fn bridge_asset_create(
			origin,
			blockchain: BlockchainType,
			symbol: Vec<u8>,
			precision: T::Precision,
		) {
			ensure_root(origin)?;

			ensure!(symbol.len() > 0, "token symbol must great then 0");
			ensure!(symbol.len() <= 32, "token symbol cannot exceed 32 bytes");
			ensure!(precision.saturated_into::<u16>() <= 16, "token precision cannot exceed 16");

			let (asset_id, _) = T::AssetCreate::asset_create(symbol.clone(), precision.saturated_into::<u16>());
			let asset_symbol = BridgeAssetSymbol::new(blockchain, symbol, precision);
			<BridgeAssetIdToAsset<T>>::insert(asset_id, asset_symbol.clone());
			<BridgeAssetToAssetId<T>>::insert(asset_symbol, asset_id);

			Self::deposit_event(RawEvent::BridgeAssetCreated(asset_id));
		}
	}
}

impl<T: Trait> AssetRedeem<T::AssetId, T::AccountId, T::Balance> for Module<T> {
	fn asset_redeem(asset_id: T::AssetId, target: T::AccountId, amount: T::Balance, to_name: Option<Vec<u8>>) {
		let account = <BridgeAccountIdToAccount<T>>::get(target);
		let symbol = <BridgeAssetIdToAsset<T>>::get(asset_id);
		let bridge_asset = BridgeAssetBalance::<T::Precision, T::Balance> {
			symbol,
			amount,
		};
		T::BridgeAssetTo::bridge_asset_to(account, bridge_asset);
	}
}

impl<T: Trait> BridgeAssetFrom<T::AccountId, T::Precision, T::Balance> for Module<T> {
	fn bridge_asset_from(target: T::AccountId, bridge_asset: BridgeAssetBalance<T::Precision, T::Balance>) {
		let asset_id = <BridgeAssetToAssetId<T>>::get(bridge_asset.symbol);
		T::AssetIssue::asset_issue(asset_id, target.clone(), bridge_asset.amount);
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {

}