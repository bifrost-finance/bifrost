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

pub mod calls;
pub mod traits;
pub use calls::*;
use orml_traits::MultiCurrency;
pub use pallet::*;
pub use traits::{ChainId, MessageId, Nonce, SalpHelper};

macro_rules! use_relay {
    ({ $( $code:tt )* }) => {
        if T::RelayNetwork::get() == NetworkId::Polkadot {
            use polkadot::RelaychainCall;

			$( $code )*
        } else if T::RelayNetwork::get() == NetworkId::Kusama {
            use kusama::RelaychainCall;

			$( $code )*
        } else if T::RelayNetwork::get() == NetworkId::Rococo {
            use rococo::RelaychainCall;

			$( $code )*
        } else {
            unreachable!()
        }
    }
}

pub(crate) type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub(crate) type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

pub(crate) type BalanceOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use cumulus_primitives_core::ParaId;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use orml_traits::{currency::TransferAll, MultiCurrency, MultiReservableCurrency};
	use scale_info::TypeInfo;
	use sp_runtime::{
		traits::{Convert, Zero},
		DispatchError,
	};
	use sp_std::{convert::From, prelude::*, vec, vec::Vec};
	use xcm::{
		v3::{prelude::*, ExecuteXcm, Parent},
		DoubleEncoded, VersionedXcm,
	};

	use super::*;
	use crate::traits::*;

	#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
	pub enum XcmInterfaceOperation {
		UmpContributeTransact,
		StatemineTransfer,
	}

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

		/// Convert `T::AccountId` to `MultiLocation`.
		type AccountIdToMultiLocation: Convert<AccountIdOf<Self>, MultiLocation>;

		/// Salp call encode
		type SalpHelper: SalpHelper<
			AccountIdOf<Self>,
			<Self as pallet_xcm::Config>::RuntimeCall,
			BalanceOf<Self>,
		>;

		#[pallet::constant]
		type RelayNetwork: Get<NetworkId>;

		#[pallet::constant]
		type StatemineTransferFee: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type StatemineTransferWeight: Get<Weight>;

		#[pallet::constant]
		type ContributionFee: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type ContributionWeight: Get<Weight>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		#[pallet::constant]
		type CallBackTimeOut: Get<Self::BlockNumber>;
	}

	#[pallet::error]
	pub enum Error<T> {
		FeeConvertFailed,
		XcmExecutionFailed,
		XcmSendFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Xcm dest weight has been updated. \[xcm_operation, new_xcm_dest_weight\]
		XcmDestWeightUpdated(XcmInterfaceOperation, Weight),
		/// Xcm dest weight has been updated. \[xcm_operation, new_xcm_dest_weight\]
		XcmFeeUpdated(XcmInterfaceOperation, BalanceOf<T>),
		TransferredStatemineMultiAsset(AccountIdOf<T>, BalanceOf<T>),
	}

	/// The dest weight limit and fee for execution XCM msg sent by XcmInterface. Must be
	/// sufficient, otherwise the execution of XCM msg on relaychain will fail.
	///
	/// XcmDestWeightAndFee: map: XcmInterfaceOperation => (Weight, Balance)
	#[pallet::storage]
	#[pallet::getter(fn xcm_dest_weight_and_fee)]
	pub type XcmDestWeightAndFee<T: Config> =
		StorageMap<_, Twox64Concat, XcmInterfaceOperation, (Weight, BalanceOf<T>), OptionQuery>;

	/// Tracker for the next nonce index
	#[pallet::storage]
	#[pallet::getter(fn current_nonce)]
	pub(super) type CurrentNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, ChainId, Nonce, ValueQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the xcm_dest_weight and fee for XCM operation of XcmInterface.
		///
		/// Parameters:
		/// - `updates`: vec of tuple: (XcmInterfaceOperation, WeightChange, FeeChange).
		#[pallet::call_index(0)]
		#[pallet::weight({16_690_000})]
		pub fn update_xcm_dest_weight_and_fee(
			origin: OriginFor<T>,
			updates: Vec<(XcmInterfaceOperation, Option<Weight>, Option<BalanceOf<T>>)>,
		) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			for (operation, weight_change, fee_change) in updates {
				XcmDestWeightAndFee::<T>::mutate_exists(operation, |info| {
					if let Some(new_weight) = weight_change {
						match info.as_mut() {
							Some(info) => info.0 = new_weight,
							None => *info = Some((new_weight, Zero::zero())),
						}
						Self::deposit_event(Event::<T>::XcmDestWeightUpdated(
							operation, new_weight,
						));
					}
					if let Some(new_fee) = fee_change {
						match info.as_mut() {
							Some(info) => info.1 = new_fee,
							None => *info = Some((Zero::zero(), new_fee)),
						}
						Self::deposit_event(Event::<T>::XcmFeeUpdated(operation, new_fee));
					}
				});
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
			let origin_location = T::AccountIdToMultiLocation::convert(who.clone());
			let dst_location = T::AccountIdToMultiLocation::convert(dest.clone());
			let amount_u128 =
				TryInto::<u128>::try_into(amount).map_err(|_| Error::<T>::FeeConvertFailed)?;

			let (dest_weight, xcm_fee) =
				Self::xcm_dest_weight_and_fee(XcmInterfaceOperation::StatemineTransfer)
					.unwrap_or((T::StatemineTransferWeight::get(), T::StatemineTransferFee::get()));
			let xcm_fee_u128 =
				TryInto::<u128>::try_into(xcm_fee).map_err(|_| Error::<T>::FeeConvertFailed)?;

			let mut assets = MultiAssets::new();
			let statemine_asset = MultiAsset {
				id: Concrete(MultiLocation::new(
					1,
					X3(
						Parachain(parachains::Statemine::ID),
						PalletInstance(parachains::Statemine::PALLET_ID),
						GeneralIndex(asset_id.into()),
					),
				)),
				fun: Fungible(amount_u128),
			};
			let fee_asset = MultiAsset {
				id: Concrete(MultiLocation::new(1, Junctions::Here)),
				fun: Fungible(xcm_fee_u128),
			};
			assets.push(statemine_asset);
			assets.push(fee_asset.clone());
			let msg = Xcm(vec![
				WithdrawAsset(assets),
				InitiateReserveWithdraw {
					assets: All.into(),
					reserve: MultiLocation::new(1, X1(Parachain(parachains::Statemine::ID))),
					xcm: Xcm(vec![
						BuyExecution { fees: fee_asset, weight_limit: Unlimited },
						DepositAsset { assets: AllCounted(2).into(), beneficiary: dst_location },
					]),
				},
			]);
			let hash = msg.using_encoded(sp_io::hashing::blake2_256);
			<T as pallet_xcm::Config>::XcmExecutor::execute_xcm_in_credit(
				origin_location,
				msg,
				hash,
				dest_weight,
				dest_weight,
			)
			.ensure_complete()
			.map_err(|_| Error::<T>::XcmExecutionFailed)?;

			Self::deposit_event(Event::<T>::TransferredStatemineMultiAsset(dest, amount));

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
			let (dest_weight, xcm_fee) =
				Self::xcm_dest_weight_and_fee(XcmInterfaceOperation::UmpContributeTransact)
					.unwrap_or((T::ContributionWeight::get(), T::ContributionFee::get()));

			// Construct confirm_contribute_call
			let confirm_contribute_call = T::SalpHelper::confirm_contribute_call();
			// Generate query_id
			let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
				MultiLocation::parent(),
				confirm_contribute_call,
				T::CallBackTimeOut::get(),
				Here,
			);

			// Bind query_id and contribution
			T::SalpHelper::bind_query_id_and_contribution(query_id, index, contributor, amount);

			let (msg_id, msg) =
				Self::build_ump_transact(query_id, contribute_call, dest_weight, xcm_fee)?;

			let result = pallet_xcm::Pallet::<T>::send_xcm(Here, Parent, msg);
			ensure!(result.is_ok(), Error::<T>::XcmSendFailed);
			Ok(msg_id)
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn transact_id(data: &[u8]) -> MessageId {
			return sp_io::hashing::blake2_256(data);
		}

		pub(crate) fn build_ump_transact(
			query_id: QueryId,
			call: DoubleEncoded<()>,
			weight: Weight,
			fee: BalanceOf<T>,
		) -> Result<(MessageId, Xcm<()>), Error<T>> {
			let sovereign_account: AccountIdOf<T> = T::ParachainSovereignAccount::get();
			let sovereign_location: MultiLocation =
				T::AccountIdToMultiLocation::convert(sovereign_account);
			let fee_amount =
				TryInto::<u128>::try_into(fee).map_err(|_| Error::<T>::FeeConvertFailed)?;
			let asset: MultiAsset = MultiAsset {
				id: Concrete(MultiLocation::here()),
				fun: Fungibility::from(fee_amount),
			};
			let message = Xcm(vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: weight,
					call,
				},
				ReportTransactStatus(QueryResponseInfo {
					destination: MultiLocation::from(X1(Parachain(u32::from(
						T::ParachainId::get(),
					)))),
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
