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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod migration;
pub mod traits;
pub mod weights;
pub use weights::WeightInfo;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, CheckedSub, Saturating, Zero},
		ArithmeticError, DispatchError, Permill, SaturatedConversion,
	},
	transactional, BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{
	currency::VFIL, traits::BridgeOperator, CurrencyId, CurrencyIdConversion, CurrencyIdExt,
	CurrencyIdRegister, ReceiveFromAnchor, RedeemType, SlpOperator, SlpxOperator, TimeUnit,
	VTokenSupplyProvider, VtokenMintingInterface, VtokenMintingOperator, XcmOperationType,
	CROSSCHAIN_AMOUNT_LENGTH, CROSSCHAIN_CURRENCY_ID_LENGTH, CROSSCHAIN_OPERATION_LENGTH,
};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_core::U256;
use sp_std::{vec, vec::Vec};
pub use traits::*;
use xcm::opaque::lts::NetworkId;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type UnlockId = u32;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::DispatchResultWithPostInfo;
	use node_primitives::{currency::BNC, FIL};
	use orml_traits::XcmTransfer;
	use xcm::{prelude::*, v3::MultiLocation};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_bcmp::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		// + MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Handler to notify the runtime when redeem success
		/// If you don't need it, you can specify the type `()`.
		type OnRedeemSuccess: OnRedeemSuccess<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		/// xtokens xcm transfer interface
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyIdOf<Self>>;

		/// The amount of mint
		#[pallet::constant]
		type MaximumUnlockIdOfUser: Get<u32>;

		#[pallet::constant]
		type MaximumUnlockIdOfTimeUnit: Get<u32>;

		#[pallet::constant]
		type EntranceAccount: Get<PalletId>;

		#[pallet::constant]
		type ExitAccount: Get<PalletId>;

		#[pallet::constant]
		type FeeAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type AstarParachainId: Get<u32>;

		#[pallet::constant]
		type MoonbeamParachainId: Get<u32>;

		#[pallet::constant]
		type HydradxParachainId: Get<u32>;

		type BifrostSlp: SlpOperator<CurrencyId>;

		type BifrostSlpx: SlpxOperator<BalanceOf<Self>>;

		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;

		type CurrencyIdRegister: CurrencyIdRegister<CurrencyId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		// Bool bridge operator to send cross out message
		type BridgeOperator: BridgeOperator<AccountIdOf<Self>, BalanceOf<Self>, CurrencyId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Minted {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
		},
		Redeemed {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
		},
		RedeemSuccess {
			unlock_id: UnlockId,
			token_id: CurrencyIdOf<T>,
			to: AccountIdOf<T>,
			token_amount: BalanceOf<T>,
		},
		Rebonded {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
		},
		RebondedByUnlockId {
			address: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			vtoken_amount: BalanceOf<T>,
			fee: BalanceOf<T>,
		},
		UnlockDurationSet {
			token_id: CurrencyIdOf<T>,
			unlock_duration: TimeUnit,
		},
		MinimumMintSet {
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		MinimumRedeemSet {
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		SupportRebondTokenAdded {
			token_id: CurrencyIdOf<T>,
		},
		SupportRebondTokenRemoved {
			token_id: CurrencyIdOf<T>,
		},
		/// Several fees has been set.
		FeeSet {
			mint_fee: Permill,
			redeem_fee: Permill,
			cancel_fee: Permill,
		},
		HookIterationLimitSet {
			limit: u32,
		},
		UnlockingTotalSet {
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		MinTimeUnitSet {
			token_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		},
		FastRedeemFailed {
			err: DispatchError,
		},
		SpecialVtokenExchangeRateSet {
			token_id: CurrencyIdOf<T>,
			exchange_rate: Option<(BalanceOf<T>, BalanceOf<T>)>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BelowMinimumMint,
		BelowMinimumRedeem,
		/// Invalid token to rebond.
		InvalidRebondToken,
		/// Token type not support.
		NotSupportTokenType,
		NotEnoughBalanceToUnlock,
		TokenToRebondNotZero,
		OngoingTimeUnitNotSet,
		TokenUnlockLedgerNotFound,
		UserUnlockLedgerNotFound,
		TimeUnitUnlockLedgerNotFound,
		UnlockDurationNotFound,
		Unexpected,
		CalculationOverflow,
		ExceedMaximumUnlockId,
		TooManyRedeems,
		CanNotRedeem,
		CanNotRebond,
		FailToSendCrossOutMessage,
		FailToConvert,
		NetworkIdError,
		FailToGetPayload,
		ExchangeRateError,
		FailToGetFee,
		FailedToSendMessage,
		InvalidExchangeRate,
		InvalidPayloadLength,
		InvalidXcmOperation,
	}

	// 【mint_fee, redeem_fee, cancel_fee】
	#[pallet::storage]
	#[pallet::getter(fn fees)]
	pub type Fees<T: Config> = StorageValue<_, (Permill, Permill, Permill), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_pool)]
	pub type TokenPool<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unlock_duration)]
	pub type UnlockDuration<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	#[pallet::storage]
	#[pallet::getter(fn ongoing_time_unit)]
	pub type OngoingTimeUnit<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	#[pallet::storage]
	#[pallet::getter(fn minimum_mint)]
	pub type MinimumMint<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn minimum_redeem)]
	pub type MinimumRedeem<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_unlock_next_id)]
	pub type TokenUnlockNextId<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_unlock_ledger)]
	pub type TokenUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		UnlockId,
		(T::AccountId, BalanceOf<T>, TimeUnit, RedeemType<AccountIdOf<T>>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_unlock_ledger)]
	pub type UserUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(BalanceOf<T>, BoundedVec<UnlockId, T::MaximumUnlockIdOfUser>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn time_unit_unlock_ledger)]
	pub type TimeUnitUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TimeUnit,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(BalanceOf<T>, BoundedVec<UnlockId, T::MaximumUnlockIdOfTimeUnit>, CurrencyIdOf<T>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_rebond)]
	pub type TokenToRebond<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn min_time_unit)]
	pub type MinTimeUnit<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unlocking_total)]
	pub type UnlockingTotal<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn hook_iteration_limit)]
	pub type HookIterationLimit<T: Config> = StorageValue<_, u32, ValueQuery>;

	// token_id => (numerator, denominator). For now, only FIL is supported.
	#[pallet::storage]
	#[pallet::getter(fn special_vtoken_exchange_rate)]
	pub type SpecialVtokenExchangeRate<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, (BalanceOf<T>, BalanceOf<T>), OptionQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			Self::handle_on_initialize()
				.map_err(|err| {
					Self::deposit_event(Event::FastRedeemFailed { err });
					log::error!(
						target: "runtime::vtoken-minting",
						"Received invalid justification for {:?}",
						err,
					);
					err
				})
				.ok();

			<T as Config>::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			Self::mint_inner(exchanger, token_id, token_amount, remark)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let exchanger = ensure_signed(origin)?;
			Self::redeem_inner(exchanger, vtoken_id, vtoken_amount, RedeemType::Native)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::rebond())]
		pub fn rebond(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let _token_amount_to_rebond =
				Self::token_to_rebond(token_id).ok_or(Error::<T>::InvalidRebondToken)?;
			if let Some((user_unlock_amount, mut ledger_list)) =
				Self::user_unlock_ledger(&exchanger, token_id)
			{
				ensure!(user_unlock_amount >= token_amount, Error::<T>::NotEnoughBalanceToUnlock);
				let mut tmp_amount = token_amount;
				let ledger_list_rev: Vec<UnlockId> = ledger_list.into_iter().rev().collect();
				ledger_list =
					BoundedVec::<UnlockId, T::MaximumUnlockIdOfUser>::try_from(ledger_list_rev)
						.map_err(|_| Error::<T>::ExceedMaximumUnlockId)?;
				let mut tmp = ledger_list
					.iter()
					.map(|&index| -> Result<(UnlockId, bool), Error<T>> {
						if let Some((_, unlock_amount, time_unit, _)) =
							Self::token_unlock_ledger(token_id, index)
						{
							if tmp_amount >= unlock_amount {
								if let Some((_, _, time_unit, _)) =
									TokenUnlockLedger::<T>::take(&token_id, &index)
								{
									TimeUnitUnlockLedger::<T>::mutate_exists(
										&time_unit,
										&token_id,
										|value| -> Result<(), Error<T>> {
											if let Some((
												total_locked_origin,
												ledger_list_origin,
												_,
											)) = value
											{
												if total_locked_origin == &unlock_amount {
													*value = None;
													return Ok(());
												}
												*total_locked_origin = total_locked_origin
													.checked_sub(&unlock_amount)
													.ok_or(Error::<T>::CalculationOverflow)?;
												ledger_list_origin.retain(|&x| x != index);
											} else {
												return Err(
													Error::<T>::TimeUnitUnlockLedgerNotFound,
												);
											}
											Ok(())
										},
									)?;
									tmp_amount = tmp_amount.saturating_sub(unlock_amount);
								} else {
									return Err(Error::<T>::TokenUnlockLedgerNotFound.into());
								}
								Ok((index, false))
							} else {
								TokenUnlockLedger::<T>::mutate_exists(
									&token_id,
									&index,
									|value| -> Result<(), Error<T>> {
										if let Some((_, total_locked_origin, _, _)) = value {
											if total_locked_origin == &tmp_amount {
												*value = None;
												return Ok(());
											}
											*total_locked_origin = total_locked_origin
												.checked_sub(&tmp_amount)
												.ok_or(Error::<T>::CalculationOverflow)?;
										} else {
											return Err(Error::<T>::TokenUnlockLedgerNotFound);
										}
										Ok(())
									},
								)?;
								TimeUnitUnlockLedger::<T>::mutate_exists(
									&time_unit,
									&token_id,
									|value| -> Result<(), Error<T>> {
										if let Some((total_locked_origin, _, _)) = value {
											if total_locked_origin == &tmp_amount {
												*value = None;
												return Ok(());
											}
											*total_locked_origin = total_locked_origin
												.checked_sub(&tmp_amount)
												.ok_or(Error::<T>::CalculationOverflow)?;
										} else {
											return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
										}
										Ok(())
									},
								)?;
								Ok((index, true))
							}
						} else {
							Ok((index, true))
						}
					})
					.collect::<Result<Vec<(UnlockId, bool)>, Error<T>>>()?;
				tmp.retain(|(_index, result)| *result);

				let ledger_list_tmp: Vec<UnlockId> =
					tmp.into_iter().map(|(index, _)| index).rev().collect();

				ledger_list =
					BoundedVec::<UnlockId, T::MaximumUnlockIdOfUser>::try_from(ledger_list_tmp)
						.map_err(|_| Error::<T>::ExceedMaximumUnlockId)?;

				UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
					*pool =
						pool.checked_sub(&token_amount).ok_or(Error::<T>::CalculationOverflow)?;
					Ok(())
				})?;
				UserUnlockLedger::<T>::mutate_exists(
					&exchanger,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, ledger_list_origin)) = value {
							if total_locked_origin == &token_amount {
								*value = None;
								return Ok(());
							}
							*ledger_list_origin = ledger_list;
							*total_locked_origin = total_locked_origin
								.checked_sub(&token_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
			} else {
				return Err(Error::<T>::UserUnlockLedgerNotFound.into());
			}

			if token_id != FIL {
				let (_, vtoken_amount, fee) =
					Self::mint_without_tranfer(&exchanger, vtoken_id, token_id, token_amount)?;

				TokenToRebond::<T>::mutate(&token_id, |value| -> Result<(), Error<T>> {
					if let Some(value_info) = value {
						*value_info = value_info
							.checked_add(&token_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
					} else {
						return Err(Error::<T>::InvalidRebondToken);
					}
					Ok(())
				})?;

				Self::deposit_event(Event::Rebonded {
					address: exchanger,
					token_id,
					token_amount,
					vtoken_amount,
					fee,
				});
			} else {
				// in Vfil case, token_amount is the amount of vfil. We transfer the cancelling
				// amount from under fil key to vfil key.
				Self::vtoken_cancel_redeem_operation(
					&exchanger,
					token_amount,
					vtoken_id,
					token_id,
				)?;
			}

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::rebond_by_unlock_id())]
		pub fn rebond_by_unlock_id(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			unlock_id: UnlockId,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let _token_amount_to_rebond =
				Self::token_to_rebond(token_id).ok_or(Error::<T>::InvalidRebondToken)?;

			let unlock_amount = match Self::token_unlock_ledger(token_id, unlock_id) {
				Some((who, unlock_amount, time_unit, _)) => {
					ensure!(who == exchanger, Error::<T>::CanNotRebond);
					TimeUnitUnlockLedger::<T>::mutate_exists(
						&time_unit,
						&token_id,
						|value| -> Result<(), Error<T>> {
							if let Some((total_locked_origin, ledger_list_origin, _)) = value {
								if total_locked_origin == &unlock_amount {
									*value = None;
									return Ok(());
								}
								*total_locked_origin = total_locked_origin
									.checked_sub(&unlock_amount)
									.ok_or(Error::<T>::CalculationOverflow)?;
								ledger_list_origin.retain(|&x| x != unlock_id);
							} else {
								return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
							}
							Ok(())
						},
					)?;

					UserUnlockLedger::<T>::mutate_exists(
						&who,
						&token_id,
						|value| -> Result<(), Error<T>> {
							if let Some((total_locked_origin, ledger_list_origin)) = value {
								if total_locked_origin == &unlock_amount {
									*value = None;
									return Ok(());
								}
								*total_locked_origin = total_locked_origin
									.checked_sub(&unlock_amount)
									.ok_or(Error::<T>::CalculationOverflow)?;
								ledger_list_origin.retain(|&x| x != unlock_id);
							} else {
								return Err(Error::<T>::UserUnlockLedgerNotFound);
							}
							Ok(())
						},
					)?;
					UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool
							.checked_sub(&unlock_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					})?;

					TokenUnlockLedger::<T>::remove(&token_id, &unlock_id);
					unlock_amount
				},
				_ => return Err(Error::<T>::TokenUnlockLedgerNotFound.into()),
			};

			if token_id != FIL {
				let (token_amount, vtoken_amount, fee) =
					Self::mint_without_tranfer(&exchanger, vtoken_id, token_id, unlock_amount)?;

				TokenToRebond::<T>::mutate(&token_id, |value| -> Result<(), Error<T>> {
					if let Some(value_info) = value {
						*value_info = value_info
							.checked_add(&token_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
					} else {
						return Err(Error::<T>::InvalidRebondToken);
					}
					Ok(())
				})?;

				Self::deposit_event(Event::RebondedByUnlockId {
					address: exchanger,
					token_id,
					token_amount: unlock_amount,
					vtoken_amount,
					fee,
				});
			} else {
				// in Vfil case, token_amount is the amount of vfil. We transfer the cancelling
				// amount from under fil key to vfil key.
				Self::vtoken_cancel_redeem_operation(
					&exchanger,
					unlock_amount,
					vtoken_id,
					token_id,
				)?;
			}

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::set_unlock_duration())]
		pub fn set_unlock_duration(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			unlock_duration: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			UnlockDuration::<T>::mutate(token_id, |old_unlock_duration| {
				*old_unlock_duration = Some(unlock_duration.clone());
			});

			Self::deposit_event(Event::UnlockDurationSet { token_id, unlock_duration });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_minimum_mint())]
		pub fn set_minimum_mint(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if !MinimumMint::<T>::contains_key(token_id) {
				// mutate_exists
				MinimumMint::<T>::insert(token_id, amount);
			} else {
				MinimumMint::<T>::mutate(token_id, |old_amount| {
					*old_amount = amount;
				});
			}

			match token_id {
				CurrencyId::Token(token_symbol) =>
					if !T::CurrencyIdRegister::check_vtoken_registered(token_symbol) {
						T::CurrencyIdRegister::register_vtoken_metadata(token_symbol)?;
					},
				CurrencyId::Token2(token_id) =>
					if !T::CurrencyIdRegister::check_vtoken2_registered(token_id) {
						T::CurrencyIdRegister::register_vtoken2_metadata(token_id)?;
					},
				_ => (),
			}

			Self::deposit_event(Event::MinimumMintSet { token_id, amount });

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_minimum_redeem())]
		pub fn set_minimum_redeem(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			MinimumRedeem::<T>::mutate(token_id, |old_amount| {
				*old_amount = amount;
			});

			Self::deposit_event(Event::MinimumRedeemSet { token_id, amount });
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::add_support_rebond_token())]
		pub fn add_support_rebond_token(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if !TokenToRebond::<T>::contains_key(token_id) {
				TokenToRebond::<T>::insert(token_id, BalanceOf::<T>::zero());
				Self::deposit_event(Event::SupportRebondTokenAdded { token_id });
			}

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_support_rebond_token())]
		pub fn remove_support_rebond_token(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if TokenToRebond::<T>::contains_key(token_id) {
				let token_amount_to_rebond =
					Self::token_to_rebond(token_id).ok_or(Error::<T>::InvalidRebondToken)?;
				ensure!(
					token_amount_to_rebond == BalanceOf::<T>::zero(),
					Error::<T>::TokenToRebondNotZero
				);

				TokenToRebond::<T>::remove(token_id);
				Self::deposit_event(Event::SupportRebondTokenRemoved { token_id });
			}
			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::set_fees())]
		pub fn set_fees(
			origin: OriginFor<T>,
			mint_fee: Permill,
			redeem_fee: Permill,
			cancel_fee: Permill,
		) -> DispatchResult {
			ensure_root(origin)?;

			Fees::<T>::mutate(|fees| *fees = (mint_fee, redeem_fee, cancel_fee));

			Self::deposit_event(Event::FeeSet { mint_fee, redeem_fee, cancel_fee });
			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::set_hook_iteration_limit())]
		pub fn set_hook_iteration_limit(origin: OriginFor<T>, limit: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			HookIterationLimit::<T>::mutate(|old_limit| {
				*old_limit = limit;
			});

			Self::deposit_event(Event::HookIterationLimitSet { limit });
			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight({0})]
		pub fn set_unlocking_total(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			UnlockingTotal::<T>::mutate(&token_id, |unlocking_total| *unlocking_total = amount);

			Self::deposit_event(Event::UnlockingTotalSet { token_id, amount });
			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight({0})]
		pub fn set_min_time_unit(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			time_unit: TimeUnit,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			MinTimeUnit::<T>::mutate(&token_id, |old_time_unit| *old_time_unit = time_unit.clone());

			Self::deposit_event(Event::MinTimeUnitSet { token_id, time_unit });
			Ok(())
		}

		#[pallet::call_index(13)]
		#[pallet::weight({0})]
		pub fn set_special_vtoken_exchange_rate(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			exchange_rate: Option<(BalanceOf<T>, BalanceOf<T>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Self::inner_set_special_vtoken_exchange_rate(token_id, exchange_rate)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		#[transactional]
		pub fn add_time_unit(a: TimeUnit, b: TimeUnit) -> Result<TimeUnit, DispatchError> {
			let result = match a {
				TimeUnit::Era(era_a) => match b {
					TimeUnit::Era(era_b) => TimeUnit::Era(
						era_a.checked_add(era_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::Round(round_a) => match b {
					TimeUnit::Round(round_b) => TimeUnit::Round(
						round_a.checked_add(round_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::SlashingSpan(slashing_span_a) => match b {
					TimeUnit::SlashingSpan(slashing_span_b) => TimeUnit::SlashingSpan(
						slashing_span_a
							.checked_add(slashing_span_b)
							.ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::Kblock(kblock_a) => match b {
					TimeUnit::Kblock(kblock_b) => TimeUnit::Kblock(
						kblock_a.checked_add(kblock_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::Hour(hour_a) => match b {
					TimeUnit::Hour(hour_b) => TimeUnit::Hour(
						hour_a.checked_add(hour_b).ok_or(Error::<T>::CalculationOverflow)?,
					),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				// _ => return Err(Error::<T>::Unexpected.into()),
			};

			Ok(result)
		}

		#[transactional]
		pub fn mint_without_tranfer(
			exchanger: &AccountIdOf<T>,
			vtoken_id: CurrencyId,
			token_id: CurrencyId,
			token_amount: BalanceOf<T>,
		) -> Result<(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), DispatchError> {
			let (mint_rate, _redeem_rate, _cancel_rate) = Fees::<T>::get();
			let mint_fee = mint_rate * token_amount;
			let token_amount_excluding_fee =
				token_amount.checked_sub(&mint_fee).ok_or(Error::<T>::CalculationOverflow)?;
			let mut vtoken_amount =
				Self::get_token_exchange_amount(token_id, token_amount_excluding_fee)?;
			if vtoken_amount == BalanceOf::<T>::zero() {
				vtoken_amount = token_amount_excluding_fee;
			}

			// Charging fees
			T::MultiCurrency::transfer(token_id, exchanger, &T::FeeAccount::get(), mint_fee)?;
			// Issue the corresponding vtoken to the user's account.
			T::MultiCurrency::deposit(vtoken_id, exchanger, vtoken_amount)?;
			TokenPool::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool
					.checked_add(&token_amount_excluding_fee)
					.ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;
			Ok((token_amount_excluding_fee, vtoken_amount, mint_fee))
		}

		#[transactional]
		fn on_initialize_update_ledger(
			token_id: CurrencyId,
			account: AccountIdOf<T>,
			index: &UnlockId,
			mut unlock_amount: BalanceOf<T>,
			entrance_account_balance: BalanceOf<T>,
			time_unit: TimeUnit,
			redeem_type: RedeemType<AccountIdOf<T>>,
		) -> DispatchResult {
			// unlock_amount is the amount of token.
			// for FIL, the incoming unlock amount is VFIL amount. We change it to FIL amount, and
			// give the VFIL amount to the ledger_unlock_amount variable. ledger_unlock_amount is
			// for updating ledger storage. for other tokens, unlock_amount and ledger_unlock_amount
			// are the same.
			let mut ledger_unlock_amount = unlock_amount;

			// if it is FIL, since the stored value is vfil, we need to convert it to fil by current
			// exchange rate.
			if token_id == FIL {
				let vtoken_id =
					token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
				unlock_amount = Self::get_vtoken_exchange_amount(vtoken_id, unlock_amount)?;
			}

			let ed = T::MultiCurrency::minimum_balance(token_id);
			let mut account_to_send = account.clone();

			if unlock_amount < ed {
				let receiver_balance = T::MultiCurrency::total_balance(token_id, &account);

				let receiver_balance_after = receiver_balance
					.checked_add(&unlock_amount)
					.ok_or(ArithmeticError::Overflow)?;
				if receiver_balance_after < ed {
					account_to_send = T::FeeAccount::get();
				}
			}
			if entrance_account_balance >= unlock_amount {
				T::MultiCurrency::transfer(
					token_id,
					&T::EntranceAccount::get().into_account_truncating(),
					&account_to_send,
					unlock_amount,
				)?;
				TokenUnlockLedger::<T>::remove(&token_id, &index);

				TimeUnitUnlockLedger::<T>::mutate_exists(
					&time_unit,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, ledger_list_origin, _)) = value {
							if total_locked_origin == &ledger_unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&ledger_unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
							ledger_list_origin.retain(|x| x != index);
						} else {
							return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;

				UserUnlockLedger::<T>::mutate_exists(
					&account,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, ledger_list_origin)) = value {
							if total_locked_origin == &ledger_unlock_amount {
								*value = None;
								return Ok(());
							}
							ledger_list_origin.retain(|x| x != index);
							*total_locked_origin = total_locked_origin
								.checked_sub(&ledger_unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
				match redeem_type {
					RedeemType::Native => {},
					RedeemType::Astar(receiver) => {
						let dest = MultiLocation {
							parents: 1,
							interior: X2(
								Parachain(T::AstarParachainId::get()),
								AccountId32 {
									network: None,
									id: receiver.encode().try_into().unwrap(),
								},
							),
						};
						T::XcmTransfer::transfer(
							account.clone(),
							token_id,
							unlock_amount,
							dest,
							Unlimited,
						)?;
					},
					RedeemType::Hydradx(receiver) => {
						let dest = MultiLocation {
							parents: 1,
							interior: X2(
								Parachain(T::HydradxParachainId::get()),
								AccountId32 {
									network: None,
									id: receiver.encode().try_into().unwrap(),
								},
							),
						};
						T::XcmTransfer::transfer(
							account.clone(),
							token_id,
							unlock_amount,
							dest,
							Unlimited,
						)?;
					},
					RedeemType::Moonbeam(receiver) => {
						let dest = MultiLocation {
							parents: 1,
							interior: X2(
								Parachain(T::MoonbeamParachainId::get()),
								AccountKey20 { network: None, key: receiver.to_fixed_bytes() },
							),
						};
						if token_id == FIL {
							let assets = vec![
								(token_id, unlock_amount),
								(BNC, T::BifrostSlpx::get_moonbeam_transfer_to_fee()),
							];

							T::XcmTransfer::transfer_multicurrencies(
								account.clone(),
								assets,
								1,
								dest,
								Unlimited,
							)?;
						} else {
							T::XcmTransfer::transfer(
								account.clone(),
								token_id,
								unlock_amount,
								dest,
								Unlimited,
							)?;
						}
					},
				};
			} else {
				match redeem_type {
					RedeemType::Astar(_) | RedeemType::Moonbeam(_) | RedeemType::Hydradx(_) => {
						return Ok(());
					},
					RedeemType::Native => {},
				};
				unlock_amount = entrance_account_balance;

				ledger_unlock_amount = unlock_amount;
				if token_id == FIL {
					ledger_unlock_amount =
						Self::get_token_exchange_amount(token_id, unlock_amount)?;
				}

				T::MultiCurrency::transfer(
					token_id,
					&T::EntranceAccount::get().into_account_truncating(),
					&account_to_send,
					unlock_amount,
				)?;

				TokenUnlockLedger::<T>::mutate_exists(
					&token_id,
					&index,
					|value| -> Result<(), Error<T>> {
						if let Some((_, total_locked_origin, _, _)) = value {
							if total_locked_origin == &ledger_unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&ledger_unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::TokenUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;

				TimeUnitUnlockLedger::<T>::mutate_exists(
					&time_unit,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, _ledger_list_origin, _)) = value {
							if total_locked_origin == &ledger_unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&ledger_unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;

				UserUnlockLedger::<T>::mutate_exists(
					&account,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, _ledger_list_origin)) = value {
							if total_locked_origin == &ledger_unlock_amount {
								*value = None;
								return Ok(());
							}

							*total_locked_origin = total_locked_origin
								.checked_sub(&ledger_unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
			}

			entrance_account_balance
				.checked_sub(&unlock_amount)
				.ok_or(Error::<T>::CalculationOverflow)?;

			UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool
					.checked_sub(&ledger_unlock_amount)
					.ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			T::OnRedeemSuccess::on_redeem_success(token_id, account.clone(), unlock_amount);

			Self::deposit_event(Event::RedeemSuccess {
				unlock_id: *index,
				token_id,
				to: account_to_send,
				token_amount: unlock_amount,
			});
			Ok(())
		}

		#[transactional]
		fn handle_on_initialize() -> DispatchResult {
			for currency in OngoingTimeUnit::<T>::iter_keys() {
				Self::handle_ledger_by_currency(currency)?;
			}
			Ok(())
		}

		fn handle_ledger_by_currency(currency: CurrencyId) -> DispatchResult {
			let time_unit = MinTimeUnit::<T>::get(currency);
			let unlock_duration_elem = match UnlockDuration::<T>::get(currency) {
				Some(TimeUnit::Era(unlock_duration_era)) => unlock_duration_era,
				Some(TimeUnit::Round(unlock_duration_round)) => unlock_duration_round,
				Some(TimeUnit::Kblock(unlock_duration_kblock)) => unlock_duration_kblock,
				Some(TimeUnit::Hour(unlock_duration_hour)) => unlock_duration_hour,
				_ => 0,
			};
			let ongoing_elem = match OngoingTimeUnit::<T>::get(currency) {
				Some(TimeUnit::Era(ongoing_era)) => ongoing_era,
				Some(TimeUnit::Round(ongoing_round)) => ongoing_round,
				Some(TimeUnit::Kblock(ongoing_kblock)) => ongoing_kblock,
				Some(TimeUnit::Hour(ongoing_hour)) => ongoing_hour,
				_ => 0,
			};
			if let Some((_total_locked, ledger_list, token_id)) =
				TimeUnitUnlockLedger::<T>::get(time_unit.clone(), currency)
			{
				for index in ledger_list.iter().take(Self::hook_iteration_limit() as usize) {
					if let Some((account, unlock_amount, time_unit, redeem_type)) =
						Self::token_unlock_ledger(token_id, index)
					{
						let entrance_account_balance = T::MultiCurrency::free_balance(
							token_id,
							&T::EntranceAccount::get().into_account_truncating(),
						);
						if entrance_account_balance != BalanceOf::<T>::zero() {
							Self::on_initialize_update_ledger(
								token_id,
								account,
								index,
								unlock_amount,
								entrance_account_balance,
								time_unit,
								redeem_type,
							)
							.ok();
						}
					}
				}
			} else {
				MinTimeUnit::<T>::mutate(currency, |time_unit| -> Result<(), Error<T>> {
					match time_unit {
						TimeUnit::Era(era) => {
							if ongoing_elem + unlock_duration_elem > *era {
								*era = era.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						TimeUnit::Round(round) => {
							if ongoing_elem + unlock_duration_elem > *round {
								*round =
									round.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						TimeUnit::Kblock(kblock) => {
							if ongoing_elem + unlock_duration_elem > *kblock {
								*kblock =
									kblock.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						TimeUnit::Hour(hour) => {
							if ongoing_elem + unlock_duration_elem > *hour {
								*hour =
									hour.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
							}
							Ok(())
						},
						_ => Ok(()),
					}
				})?;
			};

			Ok(())
		}

		#[transactional]
		pub fn mint_inner(
			exchanger: AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			remark: BoundedVec<u8, ConstU32<32>>,
		) -> DispatchResultWithPostInfo {
			ensure!(token_amount >= MinimumMint::<T>::get(token_id), Error::<T>::BelowMinimumMint);

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(token_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let (token_amount_excluding_fee, vtoken_amount, fee) =
				Self::mint_without_tranfer(&exchanger, vtoken_id, token_id, token_amount)?;
			// Transfer the user's token to EntranceAccount.
			T::MultiCurrency::transfer(
				token_id,
				&exchanger,
				&T::EntranceAccount::get().into_account_truncating(),
				token_amount_excluding_fee,
			)?;

			Self::deposit_event(Event::Minted {
				address: exchanger,
				token_id,
				token_amount,
				vtoken_amount,
				fee,
				remark,
			});
			Ok(().into())
		}

		#[transactional]
		pub fn redeem_inner(
			exchanger: AccountIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
			redeem_type: RedeemType<AccountIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			let token_id = T::CurrencyIdConversion::convert_to_token(vtoken_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			ensure!(
				vtoken_amount >= MinimumRedeem::<T>::get(vtoken_id),
				Error::<T>::BelowMinimumRedeem
			);

			ensure!(
				!T::BifrostSlp::all_delegation_requests_occupied(token_id),
				Error::<T>::CanNotRedeem,
			);

			let (_mint_rate, redeem_rate, _cancel_rate) = Fees::<T>::get();
			let redeem_fee = redeem_rate * vtoken_amount;
			let vtoken_amount =
				vtoken_amount.checked_sub(&redeem_fee).ok_or(Error::<T>::CalculationOverflow)?;
			// Charging fees
			T::MultiCurrency::transfer(vtoken_id, &exchanger, &T::FeeAccount::get(), redeem_fee)?;

			let token_amount = Self::get_vtoken_exchange_amount(vtoken_id, vtoken_amount)?;

			// Burn the corresponding vtoken from the user's account.
			T::MultiCurrency::withdraw(vtoken_id, &exchanger, vtoken_amount)?;

			// if it is vFIL, we record redeeming vFIL amount under the name of FIL
			// however, when we fast-redeem the users' vfil, we will culculate the FIL/VFIL exchange
			// at the time point and return corresponding amount of FIL to the users
			let record_amount = if vtoken_id == VFIL { vtoken_amount } else { token_amount };

			// record the amount of token to return to user
			Self::record_redeem_list(&exchanger, token_id, record_amount, redeem_type)?;

			let extra_weight = T::OnRedeemSuccess::on_redeemed(
				exchanger.clone(),
				token_id,
				token_amount,
				vtoken_amount,
				redeem_fee,
			);

			// if it is vFIL, we need to send cross chain message
			if vtoken_id == VFIL {
				let entrance_account = T::EntranceAccount::get().into_account_truncating();
				let token_id = vtoken_id.to_token().map_err(|_| Error::<T>::NotSupportTokenType)?;

				// first message, to send vFIL to the delegate-staking contract in filecoin network.
				// cross-chain fee paid by the user
				let receiver_location =
					T::BridgeOperator::get_registered_outer_multilocation_from_account(
						token_id,
						entrance_account,
					)
					.map_err(|_| Error::<T>::FailToConvert)?;

				Self::send_message(
					XcmOperationType::TransferTo,
					exchanger.clone(),
					Some(&receiver_location),
					vtoken_amount,
					vtoken_id,
					token_id,
				)
				.map_err(|_| Error::<T>::FailToSendCrossOutMessage)?;

				// second message, to send redeem message to the delegate-staking contract in
				// filecoin network. cross-chain fee paid by the user
				Self::send_message(
					XcmOperationType::Redeem,
					exchanger.clone(),
					Some(&receiver_location),
					vtoken_amount,
					vtoken_id,
					token_id,
				)
				.map_err(|_| Error::<T>::FailToSendCrossOutMessage)?;
			}

			Self::deposit_event(Event::Redeemed {
				address: exchanger,
				token_id,
				vtoken_amount,
				token_amount,
				fee: redeem_fee,
			});

			Ok(Some(<T as Config>::WeightInfo::redeem() + extra_weight).into())
		}

		// record the amount of token to return to user
		// if the dispatchable is call with the token_id param of vFIL, we will record the amount of
		// vFIL to be returned to the user due to cancel_redeem operation
		pub(crate) fn record_redeem_list(
			exchanger: &AccountIdOf<T>,
			token_id: CurrencyIdOf<T>,
			record_amount: BalanceOf<T>,
			redeem_type: RedeemType<AccountIdOf<T>>,
		) -> DispatchResult {
			match OngoingTimeUnit::<T>::get(token_id) {
				Some(time_unit) => {
					// Calculate the time to be locked
					let result_time_unit = Self::add_time_unit(
						Self::unlock_duration(token_id)
							.ok_or(Error::<T>::UnlockDurationNotFound)?,
						time_unit,
					)?;

					// if overflow, return 0
					TokenPool::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool.checked_sub(&record_amount).unwrap_or(BalanceOf::<T>::zero());

						Ok(())
					})?;
					UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool
							.checked_add(&record_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					})?;
					let next_id = Self::token_unlock_next_id(token_id);
					TokenUnlockLedger::<T>::insert(
						&token_id,
						&next_id,
						(&exchanger, record_amount, &result_time_unit, redeem_type),
					);

					if UserUnlockLedger::<T>::get(&exchanger, &token_id).is_some() {
						UserUnlockLedger::<T>::mutate(
							&exchanger,
							&token_id,
							|value| -> Result<(), Error<T>> {
								if let Some((total_locked, ledger_list)) = value {
									ledger_list
										.try_push(next_id)
										.map_err(|_| Error::<T>::TooManyRedeems)?;

									*total_locked = total_locked
										.checked_add(&record_amount)
										.ok_or(Error::<T>::CalculationOverflow)?;
								};
								Ok(())
							},
						)?;
					} else {
						let mut ledger_list_origin =
							BoundedVec::<UnlockId, T::MaximumUnlockIdOfUser>::default();
						ledger_list_origin
							.try_push(next_id)
							.map_err(|_| Error::<T>::TooManyRedeems)?;
						UserUnlockLedger::<T>::insert(
							&exchanger,
							&token_id,
							(record_amount, ledger_list_origin),
						);
					}

					if let Some((_, _, _token_id)) =
						TimeUnitUnlockLedger::<T>::get(&result_time_unit, &token_id)
					{
						TimeUnitUnlockLedger::<T>::mutate(
							&result_time_unit,
							&token_id,
							|value| -> Result<(), Error<T>> {
								if let Some((total_locked, ledger_list, _token_id)) = value {
									ledger_list
										.try_push(next_id)
										.map_err(|_| Error::<T>::TooManyRedeems)?;
									*total_locked = total_locked
										.checked_add(&record_amount)
										.ok_or(Error::<T>::CalculationOverflow)?;
								};
								Ok(())
							},
						)?;
					} else {
						let mut ledger_list_origin =
							BoundedVec::<UnlockId, T::MaximumUnlockIdOfTimeUnit>::default();
						ledger_list_origin
							.try_push(next_id)
							.map_err(|_| Error::<T>::TooManyRedeems)?;

						TimeUnitUnlockLedger::<T>::insert(
							&result_time_unit,
							&token_id,
							(record_amount, ledger_list_origin, token_id),
						);
					}
				},
				None => return Err(Error::<T>::OngoingTimeUnitNotSet.into()),
			}

			TokenUnlockNextId::<T>::mutate(&token_id, |unlock_id| -> Result<(), Error<T>> {
				*unlock_id = unlock_id.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			Ok(().into())
		}

		pub fn token_to_vtoken_inner(
			token_id: CurrencyIdOf<T>,
			_vtoken_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> BalanceOf<T> {
			Self::get_token_exchange_amount(token_id, token_amount)
				.unwrap_or(BalanceOf::<T>::zero())
		}

		pub fn vtoken_to_token_inner(
			_token_id: CurrencyIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> BalanceOf<T> {
			Self::get_vtoken_exchange_amount(vtoken_id, vtoken_amount)
				.unwrap_or(BalanceOf::<T>::zero())
		}

		pub fn vtoken_id_inner(token_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
			T::CurrencyIdConversion::convert_to_vtoken(token_id).ok()
		}

		pub fn token_id_inner(vtoken_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
			T::CurrencyIdConversion::convert_to_token(vtoken_id).ok()
		}

		pub(crate) fn send_message(
			operation: XcmOperationType,
			fee_payer: AccountIdOf<T>,
			to_location_op: Option<&MultiLocation>,
			amount: BalanceOf<T>,
			currency_id: CurrencyId,
			dest_native_currency_id: CurrencyId,
		) -> Result<(), Error<T>> {
			let (_network_id, dst_chain) =
				T::BridgeOperator::get_chain_network_and_id(dest_native_currency_id)
					.map_err(|_| Error::<T>::NetworkIdError)?;

			let receiver_op = to_location_op.and_then(|to_location| {
				T::BridgeOperator::get_receiver_from_multilocation(
					dest_native_currency_id,
					to_location,
				)
				.map_err(|_| Error::<T>::FailToConvert)
				.ok()
			});
			let receiver_slice_op = receiver_op.as_ref().map(Vec::as_slice);
			let payload = T::BridgeOperator::get_cross_out_payload(
				operation,
				currency_id,
				amount,
				receiver_slice_op,
			)
			.map_err(|_| Error::<T>::FailToGetPayload)?;

			T::BridgeOperator::send_message_to_anchor(fee_payer, dst_chain, &payload)
				.map_err(|_| Error::<T>::FailToSendCrossOutMessage)
				.map_err(|_| Error::<T>::FailToSendCrossOutMessage)?;

			Ok(())
		}

		fn vtoken_cancel_redeem_operation(
			exchanger: &AccountIdOf<T>,
			vtoken_amount: BalanceOf<T>,
			vtoken_id: CurrencyId,
			token_id: CurrencyId,
		) -> DispatchResult {
			// In FIL case, user redeeming VFIL amount is stored under the key of FIL
			// when user cancel redeeming, we need to change the cancelling amount to be under
			// the key of VFL meaning we need to return the VFIL amount to the user
			// we use mint_rate here analog to mint VFIL again
			let (_mint_rate, _redeem_rate, cancel_rate) = Fees::<T>::get();
			let cancel_fee = cancel_rate * vtoken_amount;
			let vtoken_amount_excluding_fee =
				vtoken_amount.checked_sub(&cancel_fee).ok_or(Error::<T>::CalculationOverflow)?;

			// record the amount of vFIL to be returned to the user due to cancel_redeem
			// operation
			Self::record_redeem_list(
				&exchanger,
				vtoken_id,
				vtoken_amount_excluding_fee,
				RedeemType::Native,
			)?;

			// charge fee
			Self::record_redeem_list(
				&T::FeeAccount::get(),
				vtoken_id,
				cancel_fee,
				RedeemType::Native,
			)?;

			Self::deposit_event(Event::Rebonded {
				address: exchanger.clone(),
				token_id: vtoken_id,
				token_amount: Zero::zero(),
				vtoken_amount: vtoken_amount_excluding_fee,
				fee: cancel_fee,
			});

			// send cancel_redeem cross-chain message
			// we apply for token_amount_excluding_fee since the vfil-minting contract will not
			// charge fee again for delegate-staking contract
			Self::send_message(
				XcmOperationType::CancelRedeem,
				exchanger.clone(),
				None,
				vtoken_amount_excluding_fee,
				vtoken_id,
				token_id,
			)
			.map_err(|_| Error::<T>::FailToSendCrossOutMessage)?;

			Ok(())
		}

		fn get_vtoken_exchange_rate(
			token_id: CurrencyIdOf<T>,
		) -> Result<Option<(BalanceOf<T>, BalanceOf<T>)>, Error<T>> {
			let result = if token_id == FIL {
				Self::special_vtoken_exchange_rate(token_id)
			} else {
				let vtoken_id =
					token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
				let token_pool_amount = Self::token_pool(token_id);
				let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

				Some((token_pool_amount, vtoken_total_issuance))
			};

			Ok(result)
		}

		fn get_vtoken_exchange_amount(
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> Result<BalanceOf<T>, Error<T>> {
			let token_id = vtoken_id.to_token().map_err(|_| Error::<T>::NotSupportTokenType)?;
			let (nominator, denominator) =
				Self::get_vtoken_exchange_rate(token_id)?.ok_or(Error::<T>::ExchangeRateError)?;

			if denominator == Zero::zero() {
				Ok(BalanceOf::<T>::zero())
			} else {
				let token_amount = U256::from(vtoken_amount.saturated_into::<u128>())
					.saturating_mul(nominator.saturated_into::<u128>().into())
					.checked_div(denominator.saturated_into::<u128>().into())
					.ok_or(Error::<T>::CalculationOverflow)?
					.as_u128()
					.saturated_into();

				Ok(token_amount)
			}
		}

		fn get_token_exchange_amount(
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> Result<BalanceOf<T>, Error<T>> {
			let (nominator, denominator) =
				Self::get_vtoken_exchange_rate(token_id)?.ok_or(Error::<T>::ExchangeRateError)?;

			if nominator == Zero::zero() {
				Ok(BalanceOf::<T>::zero())
			} else {
				let vtoken_amount = U256::from(token_amount.saturated_into::<u128>())
					.saturating_mul(denominator.saturated_into::<u128>().into())
					.checked_div(nominator.saturated_into::<u128>().into())
					.ok_or(Error::<T>::CalculationOverflow)?
					.as_u128()
					.saturated_into();

				Ok(vtoken_amount)
			}
		}

		pub fn update_exchange_rate(payload: &[u8]) -> Result<(), Error<T>> {
			// get currency_id from payload. The second 32 bytes are currency_id.
			let currency_id_u64: u64 = U256::from_big_endian(&payload[32..64])
				.try_into()
				.map_err(|_| Error::<T>::FailToConvert)?;
			let currency_id =
				CurrencyId::try_from(currency_id_u64).map_err(|_| Error::<T>::FailToConvert)?;

			ensure!(currency_id == FIL, Error::<T>::NotSupportTokenType);

			// get exchange_rate nominator and denominator from payload
			let nominator: u128 = U256::from_big_endian(&payload[64..96])
				.try_into()
				.map_err(|_| Error::<T>::FailToConvert)?;
			let nominator: BalanceOf<T> = nominator.saturated_into::<BalanceOf<T>>();

			let denominator: u128 = U256::from_big_endian(&payload[96..128])
				.try_into()
				.map_err(|_| Error::<T>::FailToConvert)?;
			let denominator: BalanceOf<T> = denominator.saturated_into::<BalanceOf<T>>();

			// update SpecialVtokenExchangeRate
			Self::inner_set_special_vtoken_exchange_rate(
				currency_id,
				Some((nominator, denominator)),
			)?;

			Ok(())
		}

		fn inner_set_special_vtoken_exchange_rate(
			token_id: CurrencyIdOf<T>,
			exchange_rate: Option<(BalanceOf<T>, BalanceOf<T>)>,
		) -> Result<(), Error<T>> {
			if let Some((_nominator, denominator)) = exchange_rate {
				ensure!(denominator > Zero::zero(), Error::<T>::InvalidExchangeRate);
			}

			SpecialVtokenExchangeRate::<T>::mutate_exists(token_id, |old_exchange_rate| {
				*old_exchange_rate = exchange_rate
			});

			Self::deposit_event(Event::SpecialVtokenExchangeRateSet { token_id, exchange_rate });
			Ok(())
		}
	}
}

impl<T: Config> VtokenMintingOperator<CurrencyId, BalanceOf<T>, AccountIdOf<T>, TimeUnit>
	for Pallet<T>
{
	fn get_token_pool(currency_id: CurrencyId) -> BalanceOf<T> {
		Self::token_pool(currency_id)
	}

	fn increase_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::CalculationOverflow)?;

			Ok(())
		})?;

		Ok(())
	}

	fn decrease_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_sub(&token_amount).ok_or(Error::<T>::CalculationOverflow)?;
			Ok(())
		})?;

		Ok(())
	}

	fn update_ongoing_time_unit(currency_id: CurrencyId, time_unit: TimeUnit) -> DispatchResult {
		OngoingTimeUnit::<T>::mutate(currency_id, |time_unit_old| -> Result<(), Error<T>> {
			*time_unit_old = Some(time_unit);
			Ok(())
		})?;

		Ok(())
	}

	fn get_ongoing_time_unit(currency_id: CurrencyId) -> Option<TimeUnit> {
		Self::ongoing_time_unit(currency_id)
	}

	fn get_unlock_records(
		currency_id: CurrencyId,
		time_unit: TimeUnit,
	) -> Option<(BalanceOf<T>, Vec<u32>)> {
		if let Some((balance, list, _)) = Self::time_unit_unlock_ledger(&time_unit, currency_id) {
			Some((balance, list.into_inner()))
		} else {
			None
		}
	}

	#[transactional]
	fn deduct_unlock_amount(
		currency_id: CurrencyId,
		index: u32,
		deduct_amount: BalanceOf<T>,
	) -> DispatchResult {
		if let Some((who, unlock_amount, time_unit, _)) =
			Self::token_unlock_ledger(currency_id, index)
		{
			ensure!(unlock_amount >= deduct_amount, Error::<T>::NotEnoughBalanceToUnlock);

			UnlockingTotal::<T>::mutate(&currency_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_sub(&deduct_amount).ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			TimeUnitUnlockLedger::<T>::mutate_exists(
				&time_unit,
				&currency_id,
				|value| -> Result<(), Error<T>> {
					if let Some((total_locked_origin, ledger_list_origin, _)) = value {
						if total_locked_origin == &deduct_amount {
							*value = None;
							return Ok(());
						}
						*total_locked_origin = total_locked_origin
							.checked_sub(&deduct_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						if unlock_amount == deduct_amount {
							ledger_list_origin.retain(|&x| x != index);
						}
					} else {
						return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
					}
					Ok(())
				},
			)?;

			UserUnlockLedger::<T>::mutate_exists(
				&who,
				&currency_id,
				|value| -> Result<(), Error<T>> {
					if let Some((total_locked_origin, ledger_list_origin)) = value {
						if total_locked_origin == &deduct_amount {
							*value = None;
							return Ok(());
						}
						*total_locked_origin = total_locked_origin
							.checked_sub(&deduct_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						if unlock_amount == deduct_amount {
							ledger_list_origin.retain(|&x| x != index);
						}
					} else {
						return Err(Error::<T>::UserUnlockLedgerNotFound);
					}
					Ok(())
				},
			)?;

			if unlock_amount == deduct_amount {
				TokenUnlockLedger::<T>::remove(&currency_id, &index);
			} else {
				TokenUnlockLedger::<T>::mutate_exists(
					&currency_id,
					&index,
					|value| -> Result<(), Error<T>> {
						if let Some((_, total_locked_origin, _, _)) = value {
							if total_locked_origin == &deduct_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&deduct_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::TokenUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
			}
		}
		Ok(())
	}

	fn get_entrance_and_exit_accounts() -> (AccountIdOf<T>, AccountIdOf<T>) {
		(
			T::EntranceAccount::get().into_account_truncating(),
			T::ExitAccount::get().into_account_truncating(),
		)
	}

	fn get_token_unlock_ledger(
		currency_id: CurrencyId,
		index: u32,
	) -> Option<(AccountIdOf<T>, BalanceOf<T>, TimeUnit, RedeemType<AccountIdOf<T>>)> {
		Self::token_unlock_ledger(currency_id, index)
	}

	fn get_astar_parachain_id() -> u32 {
		T::AstarParachainId::get()
	}
	fn get_moonbeam_parachain_id() -> u32 {
		T::MoonbeamParachainId::get()
	}
	fn get_hydradx_parachain_id() -> u32 {
		T::HydradxParachainId::get()
	}
}

impl<T: Config> VtokenMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>>
	for Pallet<T>
{
	fn mint(
		exchanger: AccountIdOf<T>,
		token_id: CurrencyIdOf<T>,
		token_amount: BalanceOf<T>,
		remark: BoundedVec<u8, ConstU32<32>>,
	) -> DispatchResultWithPostInfo {
		Self::mint_inner(exchanger, token_id, token_amount, remark)
	}

	fn redeem(
		exchanger: AccountIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		vtoken_amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		Self::redeem_inner(exchanger, vtoken_id, vtoken_amount, RedeemType::Native)
	}

	fn slpx_redeem(
		exchanger: AccountIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		vtoken_amount: BalanceOf<T>,
		redeem_type: RedeemType<AccountIdOf<T>>,
	) -> DispatchResultWithPostInfo {
		Self::redeem_inner(exchanger, vtoken_id, vtoken_amount, redeem_type)
	}

	fn token_to_vtoken(
		token_id: CurrencyIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		token_amount: BalanceOf<T>,
	) -> BalanceOf<T> {
		Self::token_to_vtoken_inner(token_id, vtoken_id, token_amount)
	}

	fn vtoken_to_token(
		token_id: CurrencyIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		vtoken_amount: BalanceOf<T>,
	) -> BalanceOf<T> {
		Self::vtoken_to_token_inner(token_id, vtoken_id, vtoken_amount)
	}

	fn vtoken_id(token_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
		Self::vtoken_id_inner(token_id)
	}

	fn token_id(vtoken_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
		Self::token_id_inner(vtoken_id)
	}

	fn get_minimums_redeem(vtoken_id: CurrencyIdOf<T>) -> BalanceOf<T> {
		MinimumRedeem::<T>::get(vtoken_id)
	}

	fn get_astar_parachain_id() -> u32 {
		T::AstarParachainId::get()
	}
	fn get_moonbeam_parachain_id() -> u32 {
		T::MoonbeamParachainId::get()
	}
	fn get_hydradx_parachain_id() -> u32 {
		T::HydradxParachainId::get()
	}
}

impl<T: Config> VTokenSupplyProvider<CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn get_vtoken_supply(vtoken: CurrencyIdOf<T>) -> Option<BalanceOf<T>> {
		if CurrencyId::is_vtoken(&vtoken) {
			Some(T::MultiCurrency::total_issuance(vtoken))
		} else {
			None
		}
	}

	fn get_token_supply(token: CurrencyIdOf<T>) -> Option<BalanceOf<T>> {
		if CurrencyId::is_token(&token) {
			Some(Self::token_pool(token))
		} else {
			None
		}
	}
}

// For use of passing the FIL/VFIL exchange rate to SpecialVtokenExchangeRate storage
impl<T: Config> ReceiveFromAnchor for Pallet<T> {
	fn receive_from_anchor(
		operation: XcmOperationType,
		payload: &[u8],
		_src_chain_network: NetworkId,
	) -> DispatchResultWithPostInfo {
		match operation {
			XcmOperationType::PassExchangeRateBack => {
				let max_len = CROSSCHAIN_OPERATION_LENGTH +
					CROSSCHAIN_CURRENCY_ID_LENGTH +
					2 * CROSSCHAIN_AMOUNT_LENGTH;
				ensure!(payload.len() == max_len, Error::<T>::InvalidPayloadLength);
				Self::update_exchange_rate(&payload)
			},
			_ => Err(Error::<T>::InvalidXcmOperation),
		}?;

		Ok(().into())
	}

	fn match_operations(
		operation: XcmOperationType,
		payload: &[u8],
		src_chain_network: NetworkId,
	) -> DispatchResultWithPostInfo {
		if &operation == &XcmOperationType::PassExchangeRateBack {
			return Self::receive_from_anchor(operation, payload, src_chain_network);
		}

		Ok(().into())
	}
}
