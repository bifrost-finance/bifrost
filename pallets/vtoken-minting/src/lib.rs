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
#![allow(deprecated)] // TODO: clear transaction

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod traits;
pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, CheckedSub, Saturating, Zero},
		DispatchError, Permill, SaturatedConversion,
	},
	transactional, BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{
	CurrencyId, SlpOperator, TimeUnit, TokenSymbol, VtokenMintingInterface, VtokenMintingOperator,
};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_core::U256;
use sp_std::vec::Vec;
pub use traits::*;
pub use weights::WeightInfo;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type UnlockId = u32;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::DispatchResultWithPostInfo;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		// + MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		/// Handler to notify the runtime when redeem success
		/// If you don't need it, you can specify the type `()`.
		type OnRedeemSuccess: OnRedeemSuccess<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

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

		type BifrostSlp: SlpOperator<CurrencyId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
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
			// hosting_fee: BalanceOf<T>,
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
	}

	#[pallet::storage]
	#[pallet::getter(fn fees)]
	pub type Fees<T: Config> = StorageValue<_, (Permill, Permill), ValueQuery>;

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
		(T::AccountId, BalanceOf<T>, TimeUnit),
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

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			Self::handle_on_initialize()
				.map_err(|e| {
					log::error!(
						target: "runtime::vtoken-minting",
						"Received invalid justification for {:?}",
						e,
					);
					e
				})
				.ok();

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(T::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			Self::mint_inner(exchanger, token_id, token_amount)
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let exchanger = ensure_signed(origin)?;
			Self::redeem_inner(exchanger, vtoken_id, vtoken_amount)
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::rebond())]
		pub fn rebond(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
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
				ledger_list.retain(|index| {
					if let Some((_, unlock_amount, time_unit)) =
						Self::token_unlock_ledger(token_id, index)
					{
						if tmp_amount >= unlock_amount {
							if let Some((_, _, time_unit)) =
								TokenUnlockLedger::<T>::take(&token_id, &index)
							{
								TimeUnitUnlockLedger::<T>::mutate_exists(
									&time_unit,
									&token_id,
									|value| -> Result<(), Error<T>> {
										if let Some((total_locked_origin, ledger_list_origin, _)) =
											value
										{
											if total_locked_origin == &unlock_amount {
												*value = None;
												return Ok(());
											}
											*total_locked_origin = total_locked_origin
												.checked_sub(&unlock_amount)
												.ok_or(Error::<T>::CalculationOverflow)?;
											ledger_list_origin.retain(|x| x != index);
										} else {
											return Err(Error::<T>::TimeUnitUnlockLedgerNotFound);
										}
										Ok(())
									},
								)
								.ok();
								tmp_amount = tmp_amount.saturating_sub(unlock_amount);
								// } else {
								// 	return Err(Error::<T>::TokenUnlockLedgerNotFound.into());
							}
							false
						} else {
							TokenUnlockLedger::<T>::mutate_exists(
								&token_id,
								&index,
								|value| -> Result<(), Error<T>> {
									if let Some((_, total_locked_origin, _)) = value {
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
							)
							.ok();
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
							)
							.ok();
							true
						}
					} else {
						true
					}
				});
				let ledger_list_tmp: Vec<UnlockId> = ledger_list.into_iter().rev().collect();

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
			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::rebond_by_unlock_id())]
		pub fn rebond_by_unlock_id(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			unlock_id: UnlockId,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
			let _token_amount_to_rebond =
				Self::token_to_rebond(token_id).ok_or(Error::<T>::InvalidRebondToken)?;

			let unlock_amount = match Self::token_unlock_ledger(token_id, unlock_id) {
				Some((who, unlock_amount, time_unit)) => {
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
			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::set_unlock_duration())]
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

		#[transactional]
		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
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

			Self::deposit_event(Event::MinimumMintSet { token_id, amount });

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::set_minimum_redeem())]
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

		#[transactional]
		#[pallet::weight(T::WeightInfo::add_support_rebond_token())]
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

		#[transactional]
		#[pallet::weight(T::WeightInfo::remove_support_rebond_token())]
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

		#[transactional]
		#[pallet::weight(T::WeightInfo::set_fees())]
		pub fn set_fees(
			origin: OriginFor<T>,
			mint_fee: Permill,
			redeem_fee: Permill,
		) -> DispatchResult {
			ensure_root(origin)?;

			Fees::<T>::mutate(|fees| *fees = (mint_fee, redeem_fee));

			Self::deposit_event(Event::FeeSet { mint_fee, redeem_fee });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::set_hook_iteration_limit())]
		pub fn set_hook_iteration_limit(origin: OriginFor<T>, limit: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			HookIterationLimit::<T>::mutate(|old_limit| {
				*old_limit = limit;
			});

			Self::deposit_event(Event::HookIterationLimitSet { limit });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
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

		#[transactional]
		#[pallet::weight(0)]
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
	}

	impl<T: Config> Pallet<T> {
		#[transactional]
		pub fn add_time_unit(a: TimeUnit, b: TimeUnit) -> Result<TimeUnit, DispatchError> {
			let result = match a {
				TimeUnit::Era(era_a) => match b {
					TimeUnit::Era(era_b) => TimeUnit::Era(era_a + era_b),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::Round(round_a) => match b {
					TimeUnit::Round(round_b) => TimeUnit::Round(round_a + round_b),
					_ => return Err(Error::<T>::Unexpected.into()),
				},
				TimeUnit::SlashingSpan(slashing_span_a) => match b {
					TimeUnit::SlashingSpan(slashing_span_b) =>
						TimeUnit::SlashingSpan(slashing_span_a + slashing_span_b),
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
			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);
			let (mint_rate, _redeem_rate) = Fees::<T>::get();
			let mint_fee = mint_rate * token_amount;
			let token_amount_excluding_fee =
				token_amount.checked_sub(&mint_fee).ok_or(Error::<T>::CalculationOverflow)?;
			let mut vtoken_amount = token_amount_excluding_fee;
			if token_pool_amount != BalanceOf::<T>::zero() {
				vtoken_amount = U256::from(token_amount_excluding_fee.saturated_into::<u128>())
					.saturating_mul(vtoken_total_issuance.saturated_into::<u128>().into())
					.checked_div(token_pool_amount.saturated_into::<u128>().into())
					.ok_or(Error::<T>::CalculationOverflow)?
					.as_u128()
					.saturated_into();
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
		) -> DispatchResult {
			if entrance_account_balance >= unlock_amount {
				TokenUnlockLedger::<T>::remove(&token_id, &index);

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
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}
							ledger_list_origin.retain(|x| x != index);
							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
								.ok_or(Error::<T>::CalculationOverflow)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound);
						}
						Ok(())
					},
				)?;
			} else {
				unlock_amount = entrance_account_balance;
				TokenUnlockLedger::<T>::mutate_exists(
					&token_id,
					&index,
					|value| -> Result<(), Error<T>> {
						if let Some((_, total_locked_origin, _)) = value {
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
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
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}
							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
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
							if total_locked_origin == &unlock_amount {
								*value = None;
								return Ok(());
							}

							*total_locked_origin = total_locked_origin
								.checked_sub(&unlock_amount)
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

			T::MultiCurrency::transfer(
				token_id,
				&T::EntranceAccount::get().into_account_truncating(),
				&account,
				unlock_amount,
			)?;

			UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_sub(&unlock_amount).ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			T::OnRedeemSuccess::on_redeem_success(token_id, account.clone(), unlock_amount);

			Self::deposit_event(Event::RedeemSuccess {
				unlock_id: *index,
				token_id,
				to: account,
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
				_ => 0,
			};
			let ongoing_elem = match OngoingTimeUnit::<T>::get(currency) {
				Some(TimeUnit::Era(ongoing_era)) => ongoing_era,
				Some(TimeUnit::Round(ongoing_round)) => ongoing_round,
				_ => 0,
			};
			if let Some((_total_locked, ledger_list, token_id)) =
				TimeUnitUnlockLedger::<T>::get(time_unit.clone(), currency)
			{
				let entrance_account_balance = T::MultiCurrency::free_balance(
					token_id,
					&T::EntranceAccount::get().into_account_truncating(),
				);
				for index in ledger_list.iter().take(Self::hook_iteration_limit() as usize) {
					if let Some((account, unlock_amount, time_unit)) =
						Self::token_unlock_ledger(token_id, index)
					{
						if entrance_account_balance != BalanceOf::<T>::zero() {
							Self::on_initialize_update_ledger(
								token_id,
								account,
								index,
								unlock_amount,
								entrance_account_balance,
								time_unit,
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
		) -> DispatchResultWithPostInfo {
			ensure!(token_amount >= MinimumMint::<T>::get(token_id), Error::<T>::BelowMinimumMint);

			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
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
			});
			Ok(().into())
		}

		#[transactional]
		pub fn redeem_inner(
			exchanger: AccountIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let token_id = vtoken_id.to_token().map_err(|_| Error::<T>::NotSupportTokenType)?;
			ensure!(
				vtoken_amount >= MinimumRedeem::<T>::get(vtoken_id),
				Error::<T>::BelowMinimumRedeem
			);
			if token_id == CurrencyId::Token(TokenSymbol::MOVR) {
				ensure!(
					!T::BifrostSlp::all_delegation_requests_occupied(token_id),
					Error::<T>::CanNotRedeem,
				);
			};
			let (_mint_rate, redeem_rate) = Fees::<T>::get();
			let redeem_fee = redeem_rate * vtoken_amount;
			let vtoken_amount =
				vtoken_amount.checked_sub(&redeem_fee).ok_or(Error::<T>::CalculationOverflow)?;
			// Charging fees
			T::MultiCurrency::transfer(vtoken_id, &exchanger, &T::FeeAccount::get(), redeem_fee)?;

			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);
			let token_amount = U256::from(vtoken_amount.saturated_into::<u128>())
				.saturating_mul(token_pool_amount.saturated_into::<u128>().into())
				.checked_div(vtoken_total_issuance.saturated_into::<u128>().into())
				.ok_or(Error::<T>::CalculationOverflow)?
				.as_u128()
				.saturated_into();

			match OngoingTimeUnit::<T>::get(token_id) {
				Some(time_unit) => {
					let result_time_unit = Self::add_time_unit(
						Self::unlock_duration(token_id)
							.ok_or(Error::<T>::UnlockDurationNotFound)?,
						time_unit,
					)?;

					T::MultiCurrency::withdraw(vtoken_id, &exchanger, vtoken_amount)?;
					TokenPool::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool
							.checked_sub(&token_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					})?;
					UnlockingTotal::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
						*pool = pool
							.checked_add(&token_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						Ok(())
					})?;
					let next_id = Self::token_unlock_next_id(token_id);
					TokenUnlockLedger::<T>::insert(
						&token_id,
						&next_id,
						(&exchanger, token_amount, &result_time_unit),
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
										.checked_add(&token_amount)
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
							(token_amount, ledger_list_origin),
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
										.checked_add(&token_amount)
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
							(token_amount, ledger_list_origin, token_id),
						);
					}
				},
				None => return Err(Error::<T>::OngoingTimeUnitNotSet.into()),
			}

			TokenUnlockNextId::<T>::mutate(&token_id, |unlock_id| -> Result<(), Error<T>> {
				*unlock_id = unlock_id.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;
				Ok(())
			})?;

			let extra_weight = T::OnRedeemSuccess::on_redeemed(
				exchanger.clone(),
				token_id,
				token_amount,
				vtoken_amount,
				redeem_fee,
			);

			Self::deposit_event(Event::Redeemed {
				address: exchanger,
				token_id,
				vtoken_amount,
				token_amount,
				fee: redeem_fee,
			});
			Ok(Some(T::WeightInfo::redeem() + extra_weight).into())
		}

		pub fn token_to_vtoken_inner(
			token_id: CurrencyIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> BalanceOf<T> {
			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			U256::from(token_amount.saturated_into::<u128>())
				.saturating_mul(vtoken_total_issuance.saturated_into::<u128>().into())
				.checked_div(token_pool_amount.saturated_into::<u128>().into())
				.unwrap_or(U256::zero())
				.as_u128()
				.saturated_into()
		}

		pub fn vtoken_to_token_inner(
			token_id: CurrencyIdOf<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> BalanceOf<T> {
			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			U256::from(vtoken_amount.saturated_into::<u128>())
				.saturating_mul(token_pool_amount.saturated_into::<u128>().into())
				.checked_div(vtoken_total_issuance.saturated_into::<u128>().into())
				.unwrap_or(U256::zero())
				.as_u128()
				.saturated_into()
		}

		pub fn vtoken_id_inner(token_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
			match token_id.to_vtoken() {
				Ok(vtoken_id) => Some(vtoken_id),
				Err(_) => None,
			}
		}

		pub fn token_id_inner(vtoken_id: CurrencyIdOf<T>) -> Option<CurrencyIdOf<T>> {
			match vtoken_id.to_token() {
				Ok(token_id) => Some(token_id),
				Err(_) => None,
			}
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
		if let Some((who, unlock_amount, time_unit)) = Self::token_unlock_ledger(currency_id, index)
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
						if let Some((_, total_locked_origin, _)) = value {
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
	) -> Option<(AccountIdOf<T>, BalanceOf<T>, TimeUnit)> {
		Self::token_unlock_ledger(currency_id, index)
	}
}

impl<T: Config> VtokenMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>>
	for Pallet<T>
{
	fn mint(
		exchanger: AccountIdOf<T>,
		token_id: CurrencyIdOf<T>,
		token_amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		Self::mint_inner(exchanger, token_id, token_amount)
	}

	fn redeem(
		exchanger: AccountIdOf<T>,
		vtoken_id: CurrencyIdOf<T>,
		vtoken_amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		Self::redeem_inner(exchanger, vtoken_id, vtoken_amount)
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
}
