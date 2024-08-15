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
#![allow(unused_imports)]

pub mod calls;
use bifrost_asset_registry::AssetMetadata;
pub use bifrost_primitives::xcm_interface::{ChainId, MessageId, Nonce, SalpHelper};
use bifrost_primitives::{traits::XcmDestWeightAndFeeHandler, CurrencyIdMapping, XcmOperationType};
pub use calls::*;
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_runtime::traits::UniqueSaturatedInto;

macro_rules! use_relay {
    ({ $( $code:tt )* }) => {
        if T::RelayNetwork::get() == NetworkId::Polkadot {
            use polkadot::RelaychainCall;
            use polkadot::AssetHubCall;

			$( $code )*
        } else if T::RelayNetwork::get() == NetworkId::Kusama {
            use kusama::RelaychainCall;
            use kusama::AssetHubCall;

			$( $code )*
        } else {
            unreachable!()
        }
    }
}

pub(crate) type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use cumulus_primitives_core::ParaId;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use orml_traits::{currency::TransferAll, MultiCurrency, MultiReservableCurrency};
	use sp_runtime::{traits::Convert, DispatchError};
	use sp_std::{convert::From, prelude::*, vec, vec::Vec};
	use xcm::{
		v4::{prelude::*, Asset, ExecuteXcm, Location},
		DoubleEncoded, VersionedXcm,
	};

	use super::*;
	use bifrost_primitives::xcm_interface::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type MultiCurrency: TransferAll<AccountIdOf<Self>>
			+ MultiCurrency<AccountIdOf<Self>>
			+ MultiReservableCurrency<AccountIdOf<Self>>;

		/// Origin represented Governance
		type UpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// The currency id of the RelayChain
		#[pallet::constant]
		type RelaychainCurrencyId: Get<CurrencyIdOf<Self>>;

		/// The account of parachain on the relaychain.
		#[pallet::constant]
		type ParachainSovereignAccount: Get<AccountIdOf<Self>>;

		/// XCM executor.
		type XcmExecutor: ExecuteXcm<<Self as frame_system::Config>::RuntimeCall>;

		/// Convert `T::AccountId` to `Location`.
		type AccountIdToLocation: Convert<AccountIdOf<Self>, Location>;

		/// Salp call encode
		type SalpHelper: SalpHelper<
			AccountIdOf<Self>,
			<Self as pallet_xcm::Config>::RuntimeCall,
			BalanceOf<Self>,
		>;

		/// Convert Location to `T::CurrencyId`.
		type CurrencyIdConvert: CurrencyIdMapping<
			CurrencyIdOf<Self>,
			xcm::v3::MultiLocation,
			AssetMetadata<BalanceOf<Self>>,
		>;

		#[pallet::constant]
		type RelayNetwork: Get<NetworkId>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		#[pallet::constant]
		type CallBackTimeOut: Get<BlockNumberFor<Self>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		FeeConvertFailed,
		XcmExecutionFailed,
		XcmSendFailed,
		OperationWeightAndFeeNotExist,
		FailToConvert,
		UnweighableMessage,
		LocalExecutionIncomplete,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		XcmDestWeightAndFeeUpdated(XcmOperationType, CurrencyIdOf<T>, Weight, BalanceOf<T>),
		TransferredStatemineMultiAsset(AccountIdOf<T>, BalanceOf<T>),
		TransferredEthereumAssets(AccountIdOf<T>, sp_core::H160, BalanceOf<T>),
	}

	/// The current storage version, we set to 2 our new version(after migrate stroage
	/// XcmWeightAndFee from SLP module).
	#[allow(unused)]
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	/// The dest weight limit and fee for execution XCM msg sent by XcmInterface. Must be
	/// sufficient, otherwise the execution of XCM msg on relaychain will fail.
	///
	/// XcmWeightAndFee: map: XcmOperationType => (Weight, Balance)
	#[pallet::storage]
	pub type XcmWeightAndFee<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		XcmOperationType,
		(Weight, BalanceOf<T>),
		OptionQuery,
	>;

	// Tracker for the next nonce index
	#[pallet::storage]
	pub(super) type CurrentNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, ChainId, Nonce, ValueQuery>;

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
			updates: Vec<(CurrencyIdOf<T>, XcmOperationType, Weight, BalanceOf<T>)>,
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
		#[pallet::call_index(1)]
		#[pallet::weight({2_000_000_000})]
		pub fn transfer_statemine_assets(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			asset_id: u32,
			dest: Option<AccountIdOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let dest = match dest {
				Some(account) => account,
				None => who.clone(),
			};

			let amount_u128 =
				TryInto::<u128>::try_into(amount).map_err(|_| Error::<T>::FeeConvertFailed)?;

			// get currency_id from asset_id
			let asset_location = Location::new(
				1,
				[
					Parachain(parachains::Statemine::ID),
					PalletInstance(parachains::Statemine::PALLET_ID),
					GeneralIndex(asset_id.into()),
				],
			);
			let currency_id = T::CurrencyIdConvert::get_currency_id(asset_location)
				.ok_or(Error::<T>::FailToConvert)?;

			// first, we need to withdraw the statemine asset from the user's account
			T::MultiCurrency::withdraw(currency_id, &who, amount)?;

			let dst_location = T::AccountIdToLocation::convert(dest.clone());

			let (dest_weight, xcm_fee) = XcmWeightAndFee::<T>::get(
				T::RelaychainCurrencyId::get(),
				XcmOperationType::StatemineTransfer,
			)
			.ok_or(Error::<T>::OperationWeightAndFeeNotExist)?;

			let xcm_fee_u128 =
				TryInto::<u128>::try_into(xcm_fee).map_err(|_| Error::<T>::FeeConvertFailed)?;

			let mut assets = Assets::new();
			let statemine_asset = Asset {
				id: AssetId(Location::new(
					0,
					[
						PalletInstance(parachains::Statemine::PALLET_ID),
						GeneralIndex(asset_id.into()),
					],
				)),
				fun: Fungible(amount_u128),
			};
			let fee_asset =
				Asset { id: AssetId(Location::new(1, Here)), fun: Fungible(xcm_fee_u128) };
			assets.push(statemine_asset.clone());
			assets.push(fee_asset.clone());
			let msg = Xcm(vec![
				WithdrawAsset(assets),
				BuyExecution { fees: fee_asset, weight_limit: Limited(dest_weight) },
				DepositAsset { assets: AllCounted(2).into(), beneficiary: dst_location },
			]);

			pallet_xcm::Pallet::<T>::send_xcm(
				Here,
				Location::new(1, Parachain(parachains::Statemine::ID)),
				msg,
			)
			.map_err(|_| Error::<T>::XcmExecutionFailed)?;

			Self::deposit_event(Event::<T>::TransferredStatemineMultiAsset(dest, amount));

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight({2_000_000_000})]
		pub fn transfer_ethereum_assets(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
			to: sp_core::H160,
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

			let remote_call: DoubleEncoded<()> = use_relay!({
				AssetHubCall::PolkadotXcm(PolkadotXcmCall::LimitedReserveTransferAssets(
					Box::new(Location::new(2, [GlobalConsensus(Ethereum { chain_id: 1 })]).into()),
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
				.into()
			});

			let remote_xcm = Xcm(vec![
				WithdrawAsset(fee.clone().into()),
				BuyExecution { fees: fee.clone(), weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most,
					call: remote_call,
				},
				DepositAsset {
					assets: All.into(),
					beneficiary: Location::new(1, [Parachain(T::ParachainId::get().into())]),
				},
			]);
			let (ticket, _) = <T as pallet_xcm::Config>::XcmRouter::validate(
				&mut Some(Location::new(1, [Parachain(parachains::Statemine::ID)])),
				&mut Some(remote_xcm),
			)
			.map_err(|_| Error::<T>::UnweighableMessage)?;
			<T as pallet_xcm::Config>::XcmRouter::deliver(ticket)
				.map_err(|_| Error::<T>::XcmExecutionFailed)?;
			Self::deposit_event(Event::<T>::TransferredEthereumAssets(who, to, amount));
			Ok(())
		}
	}

	impl<T: Config> XcmHelper<AccountIdOf<T>, BalanceOf<T>> for Pallet<T> {
		fn contribute(
			contributor: AccountIdOf<T>,
			index: ChainId,
			amount: BalanceOf<T>,
		) -> Result<MessageId, DispatchError> {
			// Construct contribute call data
			let contribute_call = Self::build_ump_crowdloan_contribute(index, amount);
			let (dest_weight, xcm_fee) = XcmWeightAndFee::<T>::get(
				T::RelaychainCurrencyId::get(),
				XcmOperationType::UmpContributeTransact,
			)
			.ok_or(Error::<T>::OperationWeightAndFeeNotExist)?;

			// Construct confirm_contribute_call
			let confirm_contribute_call = T::SalpHelper::confirm_contribute_call();
			// Generate query_id
			let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
				Location::parent(),
				confirm_contribute_call,
				T::CallBackTimeOut::get(),
				xcm::v4::Junctions::Here,
			);

			// Bind query_id and contribution
			T::SalpHelper::bind_query_id_and_contribution(query_id, index, contributor, amount);

			let (msg_id, msg) =
				Self::build_ump_transact(query_id, contribute_call, dest_weight, xcm_fee)?;

			let result = pallet_xcm::Pallet::<T>::send_xcm(
				xcm::v4::Junctions::Here,
				xcm::v4::Parent,
				xcm::v4::Xcm::try_from(msg).unwrap(),
			);
			ensure!(result.is_ok(), Error::<T>::XcmSendFailed);
			Ok(msg_id)
		}
	}

	impl<T: Config> XcmDestWeightAndFeeHandler<CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
		fn get_operation_weight_and_fee(
			token: CurrencyIdOf<T>,
			operation: XcmOperationType,
		) -> Option<(Weight, BalanceOf<T>)> {
			XcmWeightAndFee::<T>::get(token, operation)
		}

		fn set_xcm_dest_weight_and_fee(
			currency_id: CurrencyIdOf<T>,
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

	impl<T: Config> Pallet<T> {
		pub(crate) fn transact_id(data: &[u8]) -> MessageId {
			return sp_io::hashing::blake2_256(data);
		}

		pub(crate) fn build_ump_transact(
			query_id: bifrost_primitives::xcm_interface::QueryId,
			call: DoubleEncoded<()>,
			weight: Weight,
			fee: BalanceOf<T>,
		) -> Result<(MessageId, Xcm<()>), Error<T>> {
			let sovereign_account: AccountIdOf<T> = T::ParachainSovereignAccount::get();
			let sovereign_location: Location = T::AccountIdToLocation::convert(sovereign_account);
			let fee_amount =
				TryInto::<u128>::try_into(fee).map_err(|_| Error::<T>::FeeConvertFailed)?;
			let asset: Asset =
				Asset { id: AssetId(Location::here()), fun: Fungibility::from(fee_amount) };
			let message = Xcm(vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: weight,
					call,
				},
				ReportTransactStatus(QueryResponseInfo {
					destination: Location::from([Parachain(u32::from(T::ParachainId::get()))]),
					query_id,
					max_weight: weight,
				}),
				RefundSurplus,
				DepositAsset { assets: AllCounted(1).into(), beneficiary: sovereign_location },
			]);
			let data = VersionedXcm::<()>::from(message.clone()).encode();
			let id = Self::transact_id(&data[..]);
			Ok((id, message))
		}

		pub(crate) fn build_ump_crowdloan_contribute(
			index: ChainId,
			value: BalanceOf<T>,
		) -> DoubleEncoded<()> {
			use_relay!({
				let contribute_call =
					RelaychainCall::Crowdloan::<BalanceOf<T>, AccountIdOf<T>, BlockNumberFor<T>>(
						ContributeCall::Contribute(Contribution { index, value, signature: None }),
					)
					.encode()
					.into();
				contribute_call
			})
		}
	}
}
