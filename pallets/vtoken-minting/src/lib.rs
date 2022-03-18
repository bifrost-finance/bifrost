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

pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, CheckedSub, Saturating, Zero},
		SaturatedConversion,
	},
	transactional, BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, TimeUnit, VtokenMintingOperator};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_std::vec::Vec;
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

pub type MintId = u32;

#[frame_support::pallet]
pub mod pallet {
	// use sp_runtime::traits::CheckedAdd;
	// use orml_traits::arithmetic::CheckedSub;
	use frame_support::traits::tokens::currency;

	use super::*;

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

		/// The amount of mint
		#[pallet::constant]
		type MaximumMintId: Get<u32>;

		#[pallet::constant]
		type EntranceAccount: Get<PalletId>;

		#[pallet::constant]
		type ExitAccount: Get<PalletId>;

		#[pallet::constant]
		type FeeAccount: Get<Self::AccountId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Minted {
			token: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		},
		Redeemed {
			token: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		},
		Rebonded {
			token: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		},
		UnlockDurationSet {
			token: CurrencyIdOf<T>,
			era_count: u32,
		},
		MinimumMintSet {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		MinimumRedeemSet {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		SupportRebondTokenAdded {
			token: CurrencyIdOf<T>,
		},
		SupportRebondTokenRemoved {
			token: CurrencyIdOf<T>,
		},
		/// Several fees has been set.
		FeeSet {
			mint_fee: BalanceOf<T>,
			redeem_fee: BalanceOf<T>,
			// hosting_fee: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BelowMinimumMint,
		BelowMinimumRedeem,
		/// Invalid token to rebond.
		InvalidRebondToken,
		/// Invalid token.
		InvalidToken,
		/// Token type not support.
		NotSupportTokenType,
		NotEnoughBalanceToUnlock,
		TokenToRebondNotZero,
		MaxUserUnlockingChunksNotSet,
		MaxEraUnlockingChunksNotSet,
		OngoingTimeUnitNotSet,
		TokenUnlockLedgerNotFound,
		UserUnlockLedgerNotFound,
		TimeUnitUnlockLedgerNotFound,
		Unexpected,
	}

	#[pallet::storage]
	#[pallet::getter(fn fees)]
	pub type Fees<T: Config> = StorageValue<_, (BalanceOf<T>, BalanceOf<T>), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_pool)]
	pub type TokenPool<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unlock_duration)]
	pub type UnlockDuration<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>; // time_unit

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
		MintId,
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
		(BalanceOf<T>, BoundedVec<MintId, T::MaximumMintId>),
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
		(BalanceOf<T>, BoundedVec<MintId, T::MaximumMintId>, CurrencyIdOf<T>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_deduct)]
	pub type TokenToDeduct<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_add)]
	pub type TokenToAdd<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_rebond)]
	pub type TokenToRebond<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn min_time_unit)]
	pub type MinTimeUnit<T: Config> = StorageValue<_, TimeUnit, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let time_unit = MinTimeUnit::<T>::get();
			TimeUnitUnlockLedger::<T>::iter_prefix_values(time_unit.clone()).for_each(
				|(total_locked, ledger_list, token_id)| {
					let mut entrance_account_balance = T::MultiCurrency::free_balance(
						token_id,
						&T::EntranceAccount::get().into_account(),
					);
					for index in ledger_list.iter() {
						if let Some((account, unlock_amount, time_unit)) =
							Self::token_unlock_ledger(token_id, index)
						{
							if entrance_account_balance >= unlock_amount {
								entrance_account_balance =
									entrance_account_balance.saturating_sub(unlock_amount);
								T::MultiCurrency::transfer(
									token_id,
									&T::EntranceAccount::get().into_account(),
									&account,
									unlock_amount,
								);

								TimeUnitUnlockLedger::<T>::mutate(
									&time_unit,
									&token_id,
									|value| -> Result<(), Error<T>> {
										if let Some((total_locked_origin, ledger_list_origin, _)) =
											value
										{
											*total_locked_origin = total_locked_origin
												.checked_sub(&unlock_amount)
												.ok_or(Error::<T>::Unexpected)?;
											ledger_list_origin.retain(|x| x != index);
										} else {
											return Err(
												Error::<T>::TimeUnitUnlockLedgerNotFound.into()
											);
										}
										return Ok(());
									},
								);

								TokenUnlockLedger::<T>::remove(&token_id, &index);

								UserUnlockLedger::<T>::mutate(
									&account,
									&token_id,
									|value| -> Result<(), Error<T>> {
										if let Some((total_locked_origin, ledger_list_origin)) =
											value
										{
											ledger_list_origin.retain(|x| x != index);
											*total_locked_origin = total_locked_origin
												.checked_sub(&unlock_amount)
												.ok_or(Error::<T>::Unexpected)?;
										} else {
											return Err(Error::<T>::UserUnlockLedgerNotFound.into());
										}
										return Ok(());
									},
								);
							} else {
								break;
							}
						}
					}
				},
			);
			let a: Vec<(BalanceOf<T>, BoundedVec<MintId, T::MaximumMintId>, CurrencyIdOf<T>)> =
				TimeUnitUnlockLedger::<T>::iter_prefix_values(time_unit).collect();
			if a.len() == 0 {
				MinTimeUnit::<T>::mutate(|time_unit| -> Result<(), Error<T>> {
					match time_unit {
						TimeUnit::Era(era) => {
							*era = era.checked_add(1).ok_or(Error::<T>::Unexpected)?;
							Ok(())
						},
						_ => Ok(()),
					}
				});
			}

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// #[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		#[pallet::weight(10000)]
		pub fn mint(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			ensure!(token_amount >= MinimumMint::<T>::get(token_id), Error::<T>::BelowMinimumMint);

			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);
			let vtoken_amount = token_amount.saturating_mul(vtoken_total_issuance.into()) /
				token_pool_amount.into();
			// Transfer the user's token to EntranceAccount.
			T::MultiCurrency::transfer(
				token_id,
				&exchanger,
				&T::EntranceAccount::get().into_account(),
				token_amount,
			)?;
			// Issue the corresponding vtoken to the user's account.
			T::MultiCurrency::deposit(vtoken_id, &exchanger, vtoken_amount)?;
			TokenPool::<T>::mutate(token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::Unexpected)?;
				Ok(())
			});
			TokenToAdd::<T>::mutate(token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::Unexpected)?;
				Ok(())
			});

			Self::deposit_event(Event::Minted { token: token_id, token_amount });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn redeem(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let token_id = vtoken_id.to_token().map_err(|_| Error::<T>::NotSupportTokenType)?;
			ensure!(
				vtoken_amount >= MinimumRedeem::<T>::get(token_id),
				Error::<T>::BelowMinimumRedeem
			);

			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			if let Some(time_unit) = OngoingTimeUnit::<T>::get(token_id) {
				T::MultiCurrency::withdraw(vtoken_id, &exchanger, vtoken_amount)?;
				let token_amount = vtoken_amount.saturating_mul(token_pool_amount.into()) /
					vtoken_total_issuance.into();
				TokenPool::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
					*pool = pool.checked_sub(&token_amount).ok_or(Error::<T>::Unexpected)?;
					Ok(())
				});
				TokenToDeduct::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
					*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::Unexpected)?;
					Ok(())
				});
				let next_id = Self::token_unlock_next_id(token_id);
				TokenUnlockLedger::<T>::insert(
					&token_id,
					&next_id,
					(&exchanger, vtoken_amount, &time_unit),
				);

				if let Some((total_locked, ledger_list)) =
					UserUnlockLedger::<T>::get(&exchanger, &token_id)
				{
					UserUnlockLedger::<T>::mutate(
						&exchanger,
						&token_id,
						|value| -> Result<(), Error<T>> {
							if let Some((total_locked, ledger_list)) = value {
								ledger_list.try_push(next_id);
								*total_locked = total_locked
									.checked_add(&vtoken_amount)
									.ok_or(Error::<T>::Unexpected)?;
							};
							return Ok(());
						},
					);
				} else {
					let mut ledger_list_origin = BoundedVec::<MintId, T::MaximumMintId>::default();
					ledger_list_origin.try_push(next_id);
					UserUnlockLedger::<T>::insert(
						&exchanger,
						&token_id,
						(vtoken_amount, ledger_list_origin),
					);
				}

				if let Some((total_locked, ledger_list, _token_id)) =
					TimeUnitUnlockLedger::<T>::get(&time_unit, &token_id)
				{
					TimeUnitUnlockLedger::<T>::mutate(
						&time_unit,
						&token_id,
						|value| -> Result<(), Error<T>> {
							if let Some((total_locked, ledger_list, _token_id)) = value {
								ledger_list.try_push(next_id);
								*total_locked = total_locked
									.checked_add(&vtoken_amount)
									.ok_or(Error::<T>::Unexpected)?;
							};
							return Ok(());
						},
					);
				} else {
					let mut ledger_list_origin = BoundedVec::<MintId, T::MaximumMintId>::default();
					ledger_list_origin.try_push(next_id);
					TimeUnitUnlockLedger::<T>::insert(
						&time_unit,
						&token_id,
						(vtoken_amount, ledger_list_origin, token_id),
					);
				}
			} else {
				return Err(Error::<T>::OngoingTimeUnitNotSet.into());
			}

			TokenUnlockNextId::<T>::mutate(&token_id, |unlock_id| -> Result<(), Error<T>> {
				*unlock_id = unlock_id.checked_add(1).ok_or(Error::<T>::Unexpected)?;
				Ok(())
			});

			Self::deposit_event(Event::Redeemed { token: token_id, vtoken_amount });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn rebond(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
			let token_amount_to_rebond =
				Self::token_to_rebond(token_id).ok_or(Error::<T>::InvalidRebondToken)?;
			if let Some((user_unlock_amount, mut ledger_list)) =
				Self::user_unlock_ledger(&exchanger, token_id)
			{
				ensure!(user_unlock_amount >= token_amount, Error::<T>::NotEnoughBalanceToUnlock);
				let mut tmp_amount = token_amount;
				let mut ledger_list_rev: Vec<MintId> = ledger_list.into_iter().rev().collect();
				ledger_list =
					BoundedVec::<MintId, T::MaximumMintId>::try_from(ledger_list_rev).unwrap();
				ledger_list.retain(|index| {
					// todo: order
					if let Some((account, unlock_amount, time_unit)) =
						Self::token_unlock_ledger(token_id, index)
					{
						if tmp_amount >= unlock_amount {
							if let Some((_, total_locked, time_unit)) =
								TokenUnlockLedger::<T>::take(&token_id, &index)
							{
								TimeUnitUnlockLedger::<T>::mutate(
									&time_unit,
									&token_id,
									|value| -> Result<(), Error<T>> {
										if let Some((total_locked_origin, ledger_list_origin, _)) =
											value
										{
											*total_locked_origin = total_locked_origin
												.checked_sub(&unlock_amount)
												.ok_or(Error::<T>::Unexpected)?;
											ledger_list_origin.retain(|x| x != index);
										} else {
											return Err(
												Error::<T>::TimeUnitUnlockLedgerNotFound.into()
											);
										}
										return Ok(());
									},
								);
								tmp_amount = tmp_amount.saturating_sub(unlock_amount);
								// } else {
								// 	return Err(Error::<T>::TokenUnlockLedgerNotFound.into());
							}
							return false;
						} else {
							TokenUnlockLedger::<T>::mutate(
								&token_id,
								&index,
								|value| -> Result<(), Error<T>> {
									if let Some((_, total_locked_origin, _)) = value {
										*total_locked_origin = total_locked_origin
											.checked_sub(&tmp_amount)
											.ok_or(Error::<T>::Unexpected)?;
									} else {
										return Err(Error::<T>::TokenUnlockLedgerNotFound.into());
									}
									return Ok(());
								},
							);
							TimeUnitUnlockLedger::<T>::mutate(
								&time_unit,
								&token_id,
								|value| -> Result<(), Error<T>> {
									if let Some((total_locked_origin, _, _)) = value {
										*total_locked_origin = total_locked_origin
											.checked_sub(&tmp_amount)
											.ok_or(Error::<T>::Unexpected)?;
									} else {
										return Err(Error::<T>::TimeUnitUnlockLedgerNotFound.into());
									}
									return Ok(());
								},
							);
							return true;
						}
					} else {
						return true;
					}
				});
				let mut ledger_list_tmp: Vec<MintId> = ledger_list.into_iter().rev().collect();

				ledger_list =
					BoundedVec::<MintId, T::MaximumMintId>::try_from(ledger_list_tmp).unwrap();

				UserUnlockLedger::<T>::mutate(
					&exchanger,
					&token_id,
					|value| -> Result<(), Error<T>> {
						if let Some((total_locked_origin, ledger_list_origin)) = value {
							*ledger_list_origin = ledger_list;
							*total_locked_origin = total_locked_origin
								.checked_sub(&token_amount)
								.ok_or(Error::<T>::Unexpected)?;
						} else {
							return Err(Error::<T>::UserUnlockLedgerNotFound.into());
						}
						return Ok(());
					},
				);
			} else {
				return Err(Error::<T>::UserUnlockLedgerNotFound.into());
			}

			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);
			let vtoken_amount = token_amount.saturating_mul(vtoken_total_issuance.into()) /
				token_pool_amount.into();
			// Issue the corresponding vtoken to the user's account.
			T::MultiCurrency::deposit(vtoken_id, &exchanger, vtoken_amount)?;
			TokenPool::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::Unexpected)?;
				Ok(())
			});
			TokenToAdd::<T>::mutate(&token_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::Unexpected)?;
				Ok(())
			});
			Self::deposit_event(Event::Minted { token: token_id, token_amount });

			TokenToRebond::<T>::mutate(&token_id, |value| -> Result<(), Error<T>> {
				if let Some(value_info) = value {
					*value_info =
						value_info.checked_add(&token_amount).ok_or(Error::<T>::Unexpected)?;
				} else {
					return Err(Error::<T>::InvalidRebondToken.into());
				}
				return Ok(());
			});
			// TokenPool::<T>::mutate(token_id, |pool| pool.saturating_add(token_amount));
			Self::deposit_event(Event::Rebonded { token: token_id, token_amount });

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_unlock_duration(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			era_count: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !UnlockDuration::<T>::contains_key(token) {
				UnlockDuration::<T>::insert(token, era_count);
			} else {
				UnlockDuration::<T>::mutate(token, |old_era_count| {
					*old_era_count = era_count;
				});
			}

			Self::deposit_event(Event::UnlockDurationSet { token, era_count });

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_minimum_mint(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !MinimumMint::<T>::contains_key(token) {
				// mutate_exists
				MinimumMint::<T>::insert(token, amount);
			} else {
				MinimumMint::<T>::mutate(token, |old_amount| {
					*old_amount = amount;
				});
			}

			Self::deposit_event(Event::MinimumMintSet { token, amount });

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_minimum_redeem(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !MinimumRedeem::<T>::contains_key(token) {
				MinimumRedeem::<T>::insert(token, amount);
			} else {
				MinimumRedeem::<T>::mutate(token, |old_amount| {
					*old_amount = amount;
				});
			}

			Self::deposit_event(Event::MinimumRedeemSet { token, amount });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn add_support_rebond_token(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !TokenToRebond::<T>::contains_key(token) {
				TokenToRebond::<T>::insert(token, BalanceOf::<T>::zero());
				Self::deposit_event(Event::SupportRebondTokenAdded { token });
			}

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn remove_support_rebond_token(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if TokenToRebond::<T>::contains_key(token) {
				let token_amount_to_rebond =
					Self::token_to_rebond(token).ok_or(Error::<T>::InvalidRebondToken)?;
				ensure!(
					token_amount_to_rebond == BalanceOf::<T>::zero(),
					Error::<T>::TokenToRebondNotZero
				);

				TokenToRebond::<T>::remove(token);
				Self::deposit_event(Event::SupportRebondTokenRemoved { token });
			}
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_fees(
			origin: OriginFor<T>,
			mint_fee: BalanceOf<T>,
			redeem_fee: BalanceOf<T>,
			// hosting_fee: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Fees::<T>::mutate(|fees| *fees = (mint_fee, redeem_fee));

			Self::deposit_event(Event::FeeSet { mint_fee, redeem_fee });
			Ok(())
		}
	}
}

impl<T: Config> VtokenMintingOperator<CurrencyId, BalanceOf<T>, AccountIdOf<T>, TimeUnit>
	for Pallet<T>
{
	fn increase_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_add(&token_amount).ok_or(Error::<T>::Unexpected)?;

			Ok(())
		})?;

		Ok(())
	}

	fn decrease_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_sub(&token_amount).ok_or(Error::<T>::Unexpected)?;
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

	fn deduct_unlock_amount(
		currency_id: CurrencyId,
		index: u32,
		deduct_amount: BalanceOf<T>,
	) -> DispatchResult {
		if let Some((who, unlock_amount, time_unit)) = Self::token_unlock_ledger(currency_id, index)
		{
			ensure!(unlock_amount >= deduct_amount, Error::<T>::NotEnoughBalanceToUnlock);

			TokenPool::<T>::mutate(&currency_id, |pool| -> Result<(), Error<T>> {
				*pool = pool.checked_add(&deduct_amount).ok_or(Error::<T>::Unexpected)?;
				Ok(())
			});

			TimeUnitUnlockLedger::<T>::mutate(
				&time_unit,
				&currency_id,
				|value| -> Result<(), Error<T>> {
					if let Some((total_locked_origin, ledger_list_origin, _)) = value {
						*total_locked_origin = total_locked_origin
							.checked_sub(&deduct_amount)
							.ok_or(Error::<T>::Unexpected)?;
						if unlock_amount == deduct_amount {
							ledger_list_origin.retain(|&x| x != index);
						}
					} else {
						return Err(Error::<T>::TimeUnitUnlockLedgerNotFound.into());
					}
					return Ok(());
				},
			);

			UserUnlockLedger::<T>::mutate(&who, &currency_id, |value| -> Result<(), Error<T>> {
				if let Some((total_locked_origin, ledger_list_origin)) = value {
					*total_locked_origin = total_locked_origin
						.checked_sub(&deduct_amount)
						.ok_or(Error::<T>::Unexpected)?;
					if unlock_amount == deduct_amount {
						ledger_list_origin.retain(|&x| x != index);
					}
				} else {
					return Err(Error::<T>::UserUnlockLedgerNotFound.into());
				}
				return Ok(());
			});

			if unlock_amount == deduct_amount {
				TokenUnlockLedger::<T>::remove(&currency_id, &index);
			} else {
				TokenUnlockLedger::<T>::mutate(
					&currency_id,
					&index,
					|value| -> Result<(), Error<T>> {
						if let Some((_, total_locked_origin, _)) = value {
							*total_locked_origin = total_locked_origin
								.checked_sub(&deduct_amount)
								.ok_or(Error::<T>::Unexpected)?;
						} else {
							return Err(Error::<T>::TokenUnlockLedgerNotFound.into());
						}
						return Ok(());
					},
				);
			}
		}
		Ok(())
	}

	fn get_entrance_and_exit_accounts() -> (AccountIdOf<T>, AccountIdOf<T>) {
		(T::EntranceAccount::get().into_account(), T::ExitAccount::get().into_account())
	}

	fn get_token_unlock_ledger(
		currency_id: CurrencyId,
		index: u32,
	) -> Option<(AccountIdOf<T>, BalanceOf<T>, TimeUnit)> {
		Self::token_unlock_ledger(currency_id, index)
	}

	fn increase_token_to_add(currency_id: CurrencyId, value: BalanceOf<T>) -> DispatchResult {
		TokenToAdd::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_add(&value).ok_or(Error::<T>::Unexpected)?;

			Ok(())
		})?;

		Ok(())
	}

	fn decrease_token_to_add(currency_id: CurrencyId, value: BalanceOf<T>) -> DispatchResult {
		TokenToAdd::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_sub(&value).ok_or(Error::<T>::Unexpected)?;

			Ok(())
		})?;

		Ok(())
	}

	fn increase_token_to_deduct(currency_id: CurrencyId, value: BalanceOf<T>) -> DispatchResult {
		TokenToDeduct::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_add(&value).ok_or(Error::<T>::Unexpected)?;

			Ok(())
		})?;

		Ok(())
	}

	fn decrease_token_to_deduct(currency_id: CurrencyId, value: BalanceOf<T>) -> DispatchResult {
		TokenToDeduct::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.checked_sub(&value).ok_or(Error::<T>::Unexpected)?;
			Ok(())
		})?;

		Ok(())
	}
}
