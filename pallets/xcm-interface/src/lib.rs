// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

mod benchmarking;
mod calls;

use crate::calls::{AssetHubCall, PolkadotXcmCall};
use bifrost_asset_registry::AssetMetadata;
use bifrost_primitives::{
	traits::XcmDestWeightAndFeeHandler, AssetHubLocation, CurrencyId, CurrencyIdMapping,
	EthereumLocation, XcmOperationType,
};
use cumulus_primitives_core::ParaId;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_core::H160;
use sp_runtime::traits::{Convert, UniqueSaturatedInto};
use sp_std::{convert::From, prelude::*, vec, vec::Vec};
use xcm::{
	v4::{prelude::*, Asset, Location},
	DoubleEncoded,
};
use frame_system::WeightInfo;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type MultiCurrency: MultiCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Origin represented Governance
		type UpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// Xcm transfer interface
		type XcmRouter: SendXcm;

		/// Convert `T::AccountId` to `Location`.
		type AccountIdToLocation: Convert<Self::AccountId, Location>;

		/// Convert Location to `T::CurrencyId`.
		type CurrencyIdConvert: CurrencyIdMapping<
			CurrencyId,
			Location,
			AssetMetadata<BalanceOf<Self>>,
		>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		XcmSendFailed,
		OperationWeightAndFeeNotExist,
		FailToConvert,
		UnweighableMessage,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		XcmDestWeightAndFeeUpdated(XcmOperationType, CurrencyId, Weight, BalanceOf<T>),
		TransferredEthereumAssets(T::AccountId, H160, BalanceOf<T>),
	}

	/// The current storage version, we set to 2 our new version(after migrate stroage
	/// XcmWeightAndFee from SLP module).
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	/// The dest weight limit and fee for execution XCM msg sent by XcmInterface. Must be
	/// sufficient, otherwise the execution of XCM msg on relaychain will fail.
	///
	/// XcmWeightAndFee: map: XcmOperationType => (Weight, Balance)
	#[pallet::storage]
	pub type XcmWeightAndFee<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		XcmOperationType,
		(Weight, BalanceOf<T>),
		OptionQuery,
	>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the xcm_dest_weight and fee for XCM operation of XcmInterface.
		///
		/// Parameters:
		/// - `updates`: vec of tuple: (XcmOperationType, WeightChange, FeeChange).
		#[pallet::call_index(0)]
		#[pallet::weight({16_690_000})]
		pub fn update_xcm_dest_weight_and_fee(
			origin: OriginFor<T>,
			updates: Vec<(CurrencyId, XcmOperationType, Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			for (currency_id, operation, weight_change, fee_change) in updates {
				Self::set_xcm_dest_weight_and_fee(
					currency_id,
					operation,
					Some((weight_change, fee_change)),
				)?;

				Self::deposit_event(Event::<T>::XcmDestWeightAndFeeUpdated(
					operation,
					currency_id,
					weight_change,
					fee_change,
				));
			}

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight({2_000_000_000})]
		pub fn transfer_ethereum_assets(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
			to: H160,
		) -> DispatchResult {
			let who = ensure_signed(origin.clone())?;
			let asset_location =
				T::CurrencyIdConvert::get_location(currency_id).ok_or(Error::<T>::FailToConvert)?;

			let asset: Asset = Asset {
				id: AssetId(asset_location),
				fun: Fungible(UniqueSaturatedInto::<u128>::unique_saturated_into(amount)),
			};

			let (require_weight_at_most, xcm_fee) =
				XcmWeightAndFee::<T>::get(currency_id, XcmOperationType::EthereumTransfer)
					.ok_or(Error::<T>::OperationWeightAndFeeNotExist)?;

			let fee: Asset = Asset {
				id: AssetId(Location::parent()),
				fun: Fungible(UniqueSaturatedInto::<u128>::unique_saturated_into(xcm_fee)),
			};

			T::MultiCurrency::withdraw(currency_id, &who, amount)?;

			let remote_call: DoubleEncoded<()> =
				AssetHubCall::PolkadotXcm(PolkadotXcmCall::LimitedReserveTransferAssets(
					Box::new(EthereumLocation::get().into()),
					Box::new(
						Location::new(
							0,
							[AccountKey20 { network: None, key: to.to_fixed_bytes() }],
						)
						.into(),
					),
					Box::new(asset.into()),
					0,
					Unlimited,
				))
				.encode()
				.into();

			let remote_xcm = Xcm(vec![
				WithdrawAsset(fee.clone().into()),
				BuyExecution { fees: fee.clone(), weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most,
					call: remote_call,
				},
				RefundSurplus,
				DepositAsset {
					assets: All.into(),
					beneficiary: Location::new(1, [Parachain(T::ParachainId::get().into())]),
				},
			]);
			let (ticket, _) =
				T::XcmRouter::validate(&mut Some(AssetHubLocation::get()), &mut Some(remote_xcm))
					.map_err(|_| Error::<T>::UnweighableMessage)?;
			T::XcmRouter::deliver(ticket).map_err(|_| Error::<T>::XcmSendFailed)?;
			Self::deposit_event(Event::<T>::TransferredEthereumAssets(who, to, amount));
			Ok(())
		}
	}

	impl<T: Config> XcmDestWeightAndFeeHandler<CurrencyId, BalanceOf<T>> for Pallet<T> {
		fn get_operation_weight_and_fee(
			token: CurrencyId,
			operation: XcmOperationType,
		) -> Option<(Weight, BalanceOf<T>)> {
			XcmWeightAndFee::<T>::get(token, operation)
		}

		fn set_xcm_dest_weight_and_fee(
			currency_id: CurrencyId,
			operation: XcmOperationType,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// If param weight_and_fee is a none, it will delete the storage. Otherwise, revise the
			// storage to the new value if exists, or insert a new record if not exists before.
			XcmWeightAndFee::<T>::mutate_exists(currency_id, &operation, |wt_n_f| {
				*wt_n_f = weight_and_fee;
			});

			Ok(())
		}
	}
}
